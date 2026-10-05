//! Modal surfaces: dialogs and edge sheets that own input until they close.
//!
//! A modal is a popup-class layer: it shares the popup paint order and the one
//! hit-test path (`Context::top_window`), so everything under it stops
//! receiving pointer, wheel, keys, text and shortcuts while it is open.
mod access;
mod dialog;
mod geometry;
mod lifecycle;
mod look;
mod overlay;
mod parts;
mod show;
mod surface;

pub use dialog::{Confirm, Confirmation, Dialog, DialogAction, DialogOutput};
pub(crate) use geometry::Measure;
pub use geometry::ModalAnchor;

use std::hash::Hash;

use super::{theme::ModalStyle, Ui};
use crate::{Id, Rect};
use surface::Content;

/// Why a modal closed. Exactly one reason is reported per closing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CloseReason {
    /// Escape was pressed with no popup open above the modal.
    Escape,
    /// The overlay was clicked.
    Overlay,
    /// The corner close button was clicked.
    CloseButton,
    /// An explicit choice: an action button or [`Ui::close_modal`].
    Action,
}

pub struct ModalOutput<R> {
    pub inner: R,
    /// Set on the one pass in which the modal closed.
    pub closed: Option<CloseReason>,
    /// Enter was pressed while focus was not in a control that handles Enter.
    pub default_action: bool,
    /// The surface in screen coordinates.
    pub rect: Rect,
}

/// A modal dialog or sheet. `open` is owned by the application; the modal sets
/// it to `false` when a close condition fires.
///
/// Escape and overlay clicks close by default; each is configured separately.
/// Layout: centered with a margin to the viewport edges, width by content between
/// the style's minimum and maximum (or explicit), height by content up to the
/// viewport; the body scrolls while header and actions stay in place.
pub struct Modal {
    source: Id,
    escape: bool,
    overlay: bool,
    close_button: bool,
    anchor: ModalAnchor,
    width: Option<f32>,
    max_height: Option<f32>,
    return_focus: Option<Id>,
    style: ModalStyle,
    bodyless: bool,
    label: Option<String>,
    role: crate::AccessRole,
    described: bool,
}

impl Modal {
    pub fn new(source: impl Hash) -> Self {
        Self {
            source: Id::new(source),
            escape: true,
            overlay: true,
            close_button: true,
            anchor: ModalAnchor::Center,
            width: None,
            max_height: None,
            return_focus: None,
            style: Default::default(),
            bodyless: false,
            label: None,
            role: crate::AccessRole::Dialog,
            described: false,
        }
    }
    /// No scrolling body: only header and actions (confirmations).
    pub(super) fn bodyless(mut self) -> Self {
        self.bodyless = true;
        self
    }
    /// What the dialog is to a screen reader, and whether the second text of its header
    /// describes it.
    pub(super) fn access(mut self, role: crate::AccessRole, described: bool) -> Self {
        (self.role, self.described) = (role, described);
        self
    }
    /// The name a screen reader announces when the modal opens. Without it the modal is
    /// named after the first text of its header ([`Self::show_parts`]); a modal that has
    /// neither is reported as [`DiagnosticKind::MissingAccessibleName`](crate::DiagnosticKind).
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    /// Escape closes the modal (default). When false Escape is still consumed.
    pub fn dismiss_on_escape(mut self, close: bool) -> Self {
        self.escape = close;
        self
    }
    /// A click on the overlay closes the modal (default). When false it is consumed.
    pub fn dismiss_on_overlay(mut self, close: bool) -> Self {
        self.overlay = close;
        self
    }
    /// Corner close button (default). It reports [`CloseReason::CloseButton`].
    pub fn close_button(mut self, show: bool) -> Self {
        self.close_button = show;
        self
    }
    pub fn anchor(mut self, anchor: ModalAnchor) -> Self {
        self.anchor = anchor;
        self
    }
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = super::sanitize::positive("Modal::width", width).or(self.width);
        self
    }
    #[track_caller]
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height =
            super::sanitize::positive("Modal::max_height", height).or(self.max_height);
        self
    }
    /// Widget to focus on close instead of the one focused when it opened.
    pub fn return_focus(mut self, id: Id) -> Self {
        self.return_focus = Some(id);
        self
    }
    pub fn style(mut self, style: ModalStyle) -> Self {
        self.style = style;
        self
    }

    /// Show `build` as the scrolling body. Returns `None` once fully closed.
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        open: &mut bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> Option<ModalOutput<R>> {
        let content = Content {
            header: None,
            body: build,
            footer: None,
        };
        show::run(self, ui, open, content)
    }

    /// Like [`Self::show`] with a header and an action row outside the scrolling body.
    pub fn show_parts<R>(
        self,
        ui: &mut Ui<'_>,
        open: &mut bool,
        header: impl FnOnce(&mut Ui<'_>),
        body: impl FnOnce(&mut Ui<'_>) -> R,
        footer: impl FnOnce(&mut Ui<'_>),
    ) -> Option<ModalOutput<R>> {
        let content = Content {
            header: Some(Box::new(header)),
            body,
            footer: Some(Box::new(footer)),
        };
        show::run(self, ui, open, content)
    }
}

impl Ui<'_> {
    /// Close the innermost modal being built, reported as [`CloseReason::Action`].
    pub fn close_modal(&mut self) {
        self.context.modals.close_requested = self.context.modals.building.last().copied();
        self.context.request_repaint();
    }
    /// Enter was pressed while focus was not in a control that handles Enter.
    /// True for the whole pass, for the innermost modal being built.
    pub fn default_action_pressed(&self) -> bool {
        self.context.modals.entered.is_some()
            && self.context.modals.entered == self.context.modals.building.last().copied()
    }
    /// Focus `response` when the enclosing modal opens, instead of its first
    /// focusable control. Call it every pass; it acts only on the opening one.
    pub fn modal_initial_focus(&mut self, response: &super::Response) {
        let opening = self
            .context
            .modals
            .building
            .last()
            .is_some_and(|id| self.context.modal_opening(*id));
        if opening && response.enabled {
            self.context.request_focus(response.id);
        }
    }
}
