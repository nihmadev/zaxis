//! Modal surfaces: dialogs and edge sheets that own input until they close.
//!
//! A modal is a popup-class layer: it shares the popup paint order and the one
//! hit-test path (`Context::top_window`), so everything under it stops
//! receiving pointer, wheel, keys, text and shortcuts while it is open.
mod access;
mod dialog;
mod geometry;
mod parts;

pub use dialog::{Confirm, Confirmation, Dialog, DialogAction, DialogOutput};
pub(crate) use geometry::Measure;
pub use geometry::ModalAnchor;

use std::hash::Hash;

use super::{appearance::Appearance, theme::ModalStyle, Sense, Ui};
use crate::{
    context::{HitAction, HitRegion, Paint},
    layout::LayoutCursor,
    Id, Layout, Padding, Rect, Shape, Vec2,
};
use geometry::{geometry, pose, Metrics};

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

type Part<'a> = Box<dyn FnOnce(&mut Ui<'_>) + 'a>;

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
        self.run(ui, open, None, build, None)
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
        self.run(
            ui,
            open,
            Some(Box::new(header)),
            body,
            Some(Box::new(footer)),
        )
    }

    fn run<'a, R>(
        self,
        ui: &mut Ui<'_>,
        open: &mut bool,
        header: Option<Part<'a>>,
        body: impl FnOnce(&mut Ui<'_>) -> R,
        footer: Option<Part<'a>>,
    ) -> Option<ModalOutput<R>> {
        let id = ui.scope.with(("modal", self.source));
        let anim = id.with("presence");
        let live = ui.context.modal_is_open(id);
        if !*open && live {
            ui.context.end_modal(id);
        }
        if !*open && ui.context.animation_status(anim).is_none() {
            return None;
        }
        let viewport = ui.context.viewport();
        if viewport.is_empty() {
            // A minimized window has no room; keep `open` for when it returns.
            return None;
        }
        // `Style` is large: copy out only what is needed instead of cloning it.
        let look = parts::Look::of(ui.style());
        let mut component = look.modal;
        component.merge(self.style);
        let motion = super::sanitize::forward("Modal motion", look.presence.clone());
        let target = if *open { 1.0_f32 } else { 0.0 };
        let t = ui
            .context
            .transition_visible(anim, Some(0.0), target, motion, true)
            .value
            .clamp(0.0, 1.0);
        if !*open && t <= 0.0 {
            ui.context.remove_animation(anim);
            return None;
        }

        ui.context.begin_modal(id, self.return_focus, *open);
        ui.context.push_popup_layer(id);
        ui.context.modals.entered = None;
        let (escape, enter) = if *open {
            ui.context.take_modal_keys(id)
        } else {
            (false, false)
        };
        if enter {
            ui.context.modals.entered = Some(id);
        }
        let mut body_look = Appearance::new(look.window_fill, look.border, look.text_color);
        body_look.rounding = look.rounding;
        body_look.blur = look.blur_radius;
        body_look.shadow = look.elevation;
        body_look.opacity = look.opacity;
        body_look.apply(component.surface);
        let mut padding = component.padding.unwrap_or(look.window_padding);
        let mut gap = component.gap.unwrap_or(look.spacing).max(0.0);
        let mut margin = component.margin.unwrap_or(16.0);
        // Very small windows trade breathing room for keeping everything on screen.
        if viewport.size().y < 320.0 || viewport.size().x < 360.0 {
            let tight = |v: f32, max: f32| v.min(max);
            padding = Padding {
                left: tight(padding.left, 12.0),
                right: tight(padding.right, 12.0),
                top: tight(padding.top, 12.0),
                bottom: tight(padding.bottom, 12.0),
            };
            gap = tight(gap, 8.0);
            margin = tight(margin, 4.0);
        }
        let measure = ui
            .context
            .modals
            .measures
            .get(&id)
            .copied()
            .unwrap_or_default();
        let metrics = Metrics {
            anchor: self.anchor,
            width: self.width,
            max_height: self.max_height,
            min_width: component.min_width.unwrap_or(280.0),
            max_width: component.max_width.unwrap_or(520.0),
            margin,
            padding,
            gap,
        };
        let geo = geometry(viewport, &metrics, &measure, body_look.rounding);
        let rect = geo.surface;

        // Overlay: dims and blurs everything below, and is the click target
        // for dismissal. Registered first so the surface and content win hits.
        let local_style = local_style(ui, body_look.text_color);
        let mut root = Ui {
            flow: None,
            context: &mut *ui.context,
            window: id,
            scope: id,
            sequence: 0,
            clip: viewport,
            layout: LayoutCursor::new(viewport, Layout::Vertical, 0.0),
            enabled: *open,
            backdrop_blur: 0.0,
            hover_style: ui.hover_style,
            local_style: Some(local_style),
            local_style_revision: ui.local_style_revision,
        };
        // Stacked modals share the dimming of the lowest one instead of darkening again.
        let dims = root.context.modals.stack.first().is_none_or(|m| m.id == id);
        let mut dim = component
            .overlay
            .unwrap_or(crate::Color::rgba(0, 0, 0, 140));
        dim.0[3] = if dims {
            (f32::from(dim.0[3]) * t).round() as u8
        } else {
            0
        };
        root.context.paint(
            id.with("overlay"),
            id,
            viewport,
            vec![Paint::Shape(Shape::rect(viewport, dim).into())],
        );
        let overlay_blur = if dims {
            component.overlay_blur.unwrap_or(0.0) * t
        } else {
            0.0
        };
        root.context.paint_blur(
            id.with("overlay-blur"),
            id,
            viewport,
            crate::Blur::new(viewport).radius(overlay_blur),
        );
        let overlay_clicked = *open && root.interact(viewport, "overlay", Sense::CLICK).clicked;

        let measured = measure.last_frame != 0;
        let transform = pose(
            self.anchor,
            rect,
            t,
            component.enter_scale.unwrap_or(1.0).max(0.01),
            component.enter_offset.unwrap_or(0.0),
        );
        let scroll = look.scroll;
        let close_size = component.close_size.unwrap_or(look.control_height - 4.0);
        let reserve = if self.close_button {
            close_size + 4.0
        } else {
            0.0
        };
        let (is_open, body_height) = (*open, geo.body_height);
        let mut next = Measure {
            parts: u8::from(!self.bodyless)
                + u8::from(header.is_some())
                + u8::from(footer.is_some()),
            last_frame: root.context.frame,
            ..Default::default()
        };
        let mut close_clicked = false;
        let mut inner = None;
        // The dialog node keeps the settled bounds of the surface while it animates.
        let (node, mut titles) = (root.context.a11y_len(), 0..0);
        let dialog = root.context.a11y_begin_layer(id, id, self.role, |node| {
            access::dialog(node, self.label.as_deref());
        });
        root.layout = LayoutCursor::new(rect, Layout::Vertical, 0.0);
        root.visual("surface", transform, if measured { t } else { 0.0 }, |s| {
            // The effect measures this child: report the surface so it counts as visible.
            s.allocate_space(rect.size());
            parts::paint_surface(s, id, rect, viewport, body_look, geo.rounding);
            if is_open {
                s.context.register_hit(HitRegion {
                    id: id.with("block"),
                    window: id,
                    rect,
                    clip: viewport,
                    action: HitAction::Block,
                });
            }
            let content = padding.inset(rect);
            let width = content.size().x;
            // Parts are measured at their natural height; the geometry fits them to the surface.
            // The body's scroll padding hangs outside the content column so that
            // controls line up with the header and the actions.
            let bleed = scroll.padding.left.max(0.0);
            let tall = Rect::from_min_size(
                content.min - Vec2::new(bleed, 0.0),
                Vec2::new(width + 2.0 * bleed, 1.0e5),
            );
            parts::region(s, tall, gap, id.with("content"), |c| {
                if let Some(header) = header {
                    let start = c.context.a11y_len();
                    next.header =
                        parts::measure_item(c, |u| indented(u, bleed, width - reserve, header));
                    titles = start..c.context.a11y_len();
                }
                if self.bodyless {
                    inner = Some(body(c));
                } else {
                    let area = super::ScrollArea::vertical()
                        .id_source("body")
                        .max_height(body_height.max(0.0))
                        .show(c, |u| inner = Some(body(u)));
                    // The bar gutter is always reserved beside the viewport.
                    let gutter = scroll.bar_width.max(0.0) + scroll.bar_margin.max(0.0);
                    next.width = area.content_size.x + gutter;
                    next.body = area.content_size.y + scroll.padding.size().y;
                }
                if let Some(footer) = footer {
                    next.footer = parts::measure_item(c, |u| indented(u, bleed, width, footer));
                }
            });
            if self.close_button {
                let corner = Rect::from_min_size(
                    Vec2::new(rect.max.x - 8.0 - close_size, rect.min.y + 8.0),
                    Vec2::splat(close_size),
                );
                close_clicked = parts::close_button(s, corner);
            }
        });
        next.width = next.width.max(0.0);
        let ctx = &mut *root.context;
        access::name(ctx, node, titles, self.described);
        ctx.a11y_end(dialog, Some((rect, viewport)));
        if !is_open {
            ctx.drop_layer_semantics(id);
        }
        let changed = (next.width - measure.width).abs() > 0.5
            || (next.header - measure.header).abs() > 0.5
            || (next.body - measure.body).abs() > 0.5
            || (next.footer - measure.footer).abs() > 0.5
            || measure.last_frame == 0;
        ctx.modals.measures.insert(id, next);
        if changed {
            ctx.request_repaint();
        }

        let mut closed = None;
        if is_open {
            closed = if escape && self.escape {
                Some(CloseReason::Escape)
            } else if overlay_clicked && self.overlay {
                Some(CloseReason::Overlay)
            } else if close_clicked {
                Some(CloseReason::CloseButton)
            } else if ctx.modals.close_requested == Some(id) {
                Some(CloseReason::Action)
            } else {
                None
            };
            if closed.is_some() {
                *open = false;
                ctx.end_modal(id);
            }
        }
        ctx.end_modal_build();
        Some(ModalOutput {
            inner: inner.expect("modal body runs once per pass"),
            closed,
            default_action: enter && is_open,
            rect,
        })
    }
}

/// The surface's text color applies to everything inside it, like in a popup.
fn local_style(ui: &Ui<'_>, text_color: crate::Color) -> std::sync::Arc<super::Style> {
    let mut style = ui.style().clone();
    style.text_color = text_color;
    std::sync::Arc::new(style)
}

/// A header or action row aligned with the body's content column.
fn indented(ui: &mut Ui<'_>, bleed: f32, width: f32, part: Part<'_>) {
    ui.horizontal(|row| {
        row.add_space(bleed);
        row.with_width(width.max(0.0), |column| {
            column.vertical(|stack| part(stack))
        });
    });
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
