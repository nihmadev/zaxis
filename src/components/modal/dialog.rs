//! Ready-made forms on top of [`Modal`]: a dialog with a title, description and
//! actions, and a confirmation whose result is a plain value.
use std::hash::Hash;

use super::{CloseReason, Modal, ModalOutput};
use crate::{
    components::{theme::ModalStyle, Button, ButtonVariant, Text, Ui},
    Align, SemanticStatus, TypographyRole,
};

/// A button in a dialog's action row.
#[derive(Clone, Debug)]
pub struct DialogAction {
    label: String,
    variant: ButtonVariant,
    status: SemanticStatus,
    default: bool,
}

impl DialogAction {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Surface,
            status: SemanticStatus::Normal,
            default: false,
        }
    }
    /// Accent-filled emphasis for the main action.
    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Solid;
        self
    }
    /// Destructive action: error fill.
    pub fn danger(mut self) -> Self {
        self.variant = ButtonVariant::Surface;
        self.status = SemanticStatus::Error;
        self
    }
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    /// Chosen by Enter when focus is not in a control that handles Enter.
    /// Without any default the last action is used.
    pub fn default_action(mut self) -> Self {
        self.default = true;
        self
    }
}

pub struct DialogOutput<R> {
    pub inner: R,
    /// Index of the chosen action on the one pass in which it closed the dialog.
    pub action: Option<usize>,
    /// Set on the one pass in which the dialog closed.
    pub closed: Option<CloseReason>,
}

/// Title, optional description, a scrolling body and a right-aligned action row.
/// Choosing an action closes the dialog; the index is in [`DialogOutput::action`].
pub struct Dialog {
    modal: Modal,
    title: String,
    description: Option<String>,
    actions: Vec<DialogAction>,
    style: ModalStyle,
}

impl Dialog {
    pub fn new(source: impl Hash, title: impl Into<String>) -> Self {
        Self {
            modal: Modal::new(source),
            title: title.into(),
            description: None,
            actions: Vec::new(),
            style: Default::default(),
        }
    }
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }
    pub fn action(mut self, action: DialogAction) -> Self {
        self.actions.push(action);
        self
    }
    /// Adjust the underlying modal: close conditions, width, anchor.
    pub fn modal(mut self, configure: impl FnOnce(Modal) -> Modal) -> Self {
        self.modal = configure(self.modal);
        self
    }
    pub fn style(mut self, style: ModalStyle) -> Self {
        self.style = style;
        self
    }

    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        open: &mut bool,
        body: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> Option<DialogOutput<R>> {
        let Self {
            modal,
            title,
            description,
            actions,
            style,
        } = self;
        let theme = ui.style().modal;
        let mut merged = theme;
        merged.merge(style);
        let (title_role, text_role) = (
            merged.title.unwrap_or(TypographyRole::Heading),
            merged.description.unwrap_or(TypographyRole::Body),
        );
        let spacing = merged.spacing.unwrap_or(8.0);
        let default = actions
            .iter()
            .position(|a| a.default)
            .unwrap_or(actions.len().saturating_sub(1));
        let mut chosen = None;
        let described = description.is_some();
        let modal = modal.access(crate::AccessRole::Dialog, described);
        let output = modal.style(style).show_parts(
            ui,
            open,
            |ui| {
                ui.layout.spacing = spacing;
                ui.add(Text::new(title).typography(title_role));
                if let Some(text) = description {
                    ui.add(Text::new(text).typography(text_role).muted());
                }
            },
            body,
            |ui| chosen = action_row(ui, &actions, default, spacing, None),
        )?;
        Some(finish(output, chosen))
    }
}

fn finish<R>(output: ModalOutput<R>, chosen: Option<usize>) -> DialogOutput<R> {
    DialogOutput {
        inner: output.inner,
        action: chosen.filter(|_| output.closed == Some(CloseReason::Action)),
        closed: output.closed,
    }
}

/// Right-aligned row. Returns the chosen action and closes the modal on a choice.
fn action_row(
    ui: &mut Ui<'_>,
    actions: &[DialogAction],
    default: usize,
    spacing: f32,
    focus: Option<usize>,
) -> Option<usize> {
    let mut chosen = None;
    ui.vertical_aligned(Align::End, |column| {
        column.horizontal(|row| {
            row.layout.spacing = spacing;
            for (index, action) in actions.iter().enumerate() {
                let response = row.add(
                    Button::new(action.label.as_str())
                        .id_source(("action", index))
                        .variant(action.variant)
                        .status(action.status),
                );
                if focus == Some(index) {
                    row.modal_initial_focus(&response);
                }
                if response.clicked {
                    chosen = Some(index);
                }
            }
        });
    });
    if chosen.is_none() && !actions.is_empty() && ui.default_action_pressed() {
        chosen = Some(default);
    }
    if chosen.is_some() {
        ui.close_modal();
    }
    chosen
}

/// What a [`Confirm`] resolved to. Returned once, on the pass it was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirmation {
    Confirmed,
    /// The cancel button gives [`CloseReason::Action`]; Escape and overlay give their own.
    Cancelled(CloseReason),
}

/// Confirmation with alert-dialog semantics: no corner button, the overlay
/// never closes it, and a dangerous action closes only by an explicit choice
/// (Escape included) with focus on the safe button.
pub struct Confirm {
    source: crate::Id,
    title: String,
    description: Option<String>,
    confirm: String,
    cancel: String,
    danger: bool,
    escape: Option<bool>,
    overlay: bool,
}

impl Confirm {
    pub fn new(source: impl Hash) -> Self {
        Self {
            source: crate::Id::new(source),
            title: String::new(),
            description: None,
            confirm: "OK".into(),
            cancel: "Cancel".into(),
            danger: false,
            escape: None,
            overlay: false,
        }
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }
    pub fn confirm_label(mut self, label: impl Into<String>) -> Self {
        self.confirm = label.into();
        self
    }
    pub fn cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel = label.into();
        self
    }
    /// The confirm button is destructive; initial focus and Enter go to Cancel,
    /// and Escape no longer closes unless re-enabled.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
    pub fn dismiss_on_escape(mut self, close: bool) -> Self {
        self.escape = Some(close);
        self
    }
    pub fn dismiss_on_overlay(mut self, close: bool) -> Self {
        self.overlay = close;
        self
    }

    pub fn show(self, ui: &mut Ui<'_>, open: &mut bool) -> Option<Confirmation> {
        let danger = self.danger;
        let actions = [
            DialogAction::new(self.cancel),
            if danger {
                DialogAction::new(self.confirm).danger()
            } else {
                DialogAction::new(self.confirm).primary()
            },
        ];
        // Safe button: Cancel when dangerous, otherwise the confirming one.
        let safe = usize::from(!danger);
        let (escape, overlay) = (self.escape.unwrap_or(!danger), self.overlay);
        let spacing = ui.style().modal.spacing.unwrap_or(8.0);
        let (title_role, text_role) = (
            ui.style().modal.title.unwrap_or(TypographyRole::Heading),
            ui.style().modal.description.unwrap_or(TypographyRole::Body),
        );
        let mut chosen = None;
        let (title, description) = (self.title, self.description);
        let output = Modal::new(self.source)
            .close_button(false)
            .bodyless()
            .access(crate::AccessRole::AlertDialog, description.is_some())
            .dismiss_on_escape(escape)
            .dismiss_on_overlay(overlay)
            .show_parts(
                ui,
                open,
                |ui| {
                    ui.layout.spacing = spacing;
                    ui.add(Text::new(title).typography(title_role));
                    if let Some(text) = description {
                        ui.add(Text::new(text).typography(text_role).muted());
                    }
                },
                |_| {},
                |ui| chosen = action_row(ui, &actions, safe, spacing, Some(safe)),
            )?;
        match output.closed? {
            CloseReason::Action => Some(if chosen == Some(1) {
                Confirmation::Confirmed
            } else {
                Confirmation::Cancelled(CloseReason::Action)
            }),
            reason => Some(Confirmation::Cancelled(reason)),
        }
    }
}
