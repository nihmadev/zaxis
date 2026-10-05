//! Field validation state and the compact `Field` layout: label, control, message.
//!
//! The library validates nothing and stores no rules. The application computes a
//! [`Validation`] every pass and hands it to a [`Field`] (or marks one control
//! with `.status(..)`); a changing message never requests frames by itself.

use std::{fmt::Display, hash::Hash};

use super::{SemanticStatus, Text, TypographyRole, Ui};
use crate::{AccessLive, AccessNode, AccessRole, Color, Id, Vec2};

/// Outcome of the application's validation of one value: a status and the text
/// to show under the control. The default is "nothing to say".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Validation {
    pub status: SemanticStatus,
    pub message: String,
}

impl Validation {
    /// No status and no message.
    pub const fn ok() -> Self {
        Self {
            status: SemanticStatus::Normal,
            message: String::new(),
        }
    }
    pub fn error(message: impl Into<String>) -> Self {
        Self::with(SemanticStatus::Error, message)
    }
    pub fn warning(message: impl Into<String>) -> Self {
        Self::with(SemanticStatus::Warning, message)
    }
    pub fn success(message: impl Into<String>) -> Self {
        Self::with(SemanticStatus::Success, message)
    }
    pub fn with(status: SemanticStatus, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

/// A status without a message.
impl From<SemanticStatus> for Validation {
    fn from(status: SemanticStatus) -> Self {
        Self::with(status, String::new())
    }
}

/// `Err` becomes an error carrying the error's text; `Ok` is [`Validation::ok`].
impl<T, E: Display> From<&Result<T, E>> for Validation {
    fn from(result: &Result<T, E>) -> Self {
        match result {
            Ok(_) => Self::ok(),
            Err(error) => Self::error(error.to_string()),
        }
    }
}

/// A label, one or more controls, and a hint or validation message below them.
///
/// Every field-like control inside (text edit, numbers, combo box, checkbox,
/// switch, slider) takes the field's status unless it sets its own. The message
/// line exists only while there is something to say, so it adds its height and
/// nothing more to the layout; [`Field::reserve_message`] keeps the line instead.
pub struct Field {
    label: String,
    hint: String,
    validation: Validation,
    id: Option<Id>,
    reserve: bool,
}

impl Field {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            hint: String::new(),
            validation: Validation::ok(),
            id: None,
            reserve: false,
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    /// Shown while the validation has no message.
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.hint = text.into();
        self
    }
    /// The result of the application's own check for this pass.
    pub fn validation(mut self, validation: impl Into<Validation>) -> Self {
        self.validation = validation.into();
        self
    }
    /// Always keep one line free under the control, so a message appearing or
    /// disappearing moves nothing.
    pub fn reserve_message(mut self, reserve: bool) -> Self {
        self.reserve = reserve;
        self
    }

    pub fn show<R>(self, ui: &mut Ui<'_>, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        let id = self.id.unwrap_or_else(|| Id::new(&self.label));
        let style = ui.style().clone();
        let gap = (style.spacing * 0.5).max(0.0);
        let message = if self.validation.message.is_empty() {
            self.hint.as_str()
        } else {
            self.validation.message.as_str()
        };
        let color = status_text(
            &style,
            self.validation.status,
            !self.validation.message.is_empty(),
        );
        let status = self.validation.status;
        ui.vertical(|ui| {
            // The layout spacing is the gap between label, control and message.
            ui.layout.spacing = gap;
            ui.push_id(("field", id), |ui| {
                // Where this pass's nodes of the label, the controls and the message begin.
                let mut nodes = [None; 3];
                if !self.label.is_empty() {
                    nodes[0] = Some(ui.context.a11y_len());
                    ui.add(
                        Text::new(self.label.clone())
                            .typography(TypographyRole::Small)
                            .color(style.text_color),
                    );
                }
                let previous = std::mem::replace(&mut ui.context.field_status, status);
                nodes[1] = Some(ui.context.a11y_len());
                let result = build(ui);
                ui.context.field_status = previous;
                if !message.is_empty() {
                    nodes[2] = Some(ui.context.a11y_len());
                    ui.add(
                        Text::new(message.to_owned())
                            .typography(TypographyRole::Small)
                            .color(color),
                    );
                } else if self.reserve {
                    let line = ui
                        .context
                        .measure_text(
                            "Ag",
                            style.typography.small,
                            style.typography.weights.small,
                            f32::INFINITY,
                        )
                        .y;
                    ui.allocate_space(Vec2::new(0.0, line));
                }
                if ui.context.a11y_on() {
                    let validated = !self.validation.message.is_empty();
                    describe(ui, nodes, message, status, validated);
                }
                result
            })
        })
    }
}

/// Roles a field's label names: the first node of one of them built inside the field is
/// its control.
fn is_control(role: AccessRole) -> bool {
    matches!(
        role,
        AccessRole::Button
            | AccessRole::CheckBox
            | AccessRole::Switch
            | AccessRole::RadioGroup
            | AccessRole::RadioButton
            | AccessRole::Slider
            | AccessRole::SpinButton
            | AccessRole::TextInput
            | AccessRole::MultilineTextInput
            | AccessRole::ComboBox
            | AccessRole::ListBox
            | AccessRole::ColorWell
            | AccessRole::Tree
            | AccessRole::Table
            | AccessRole::Grid
    )
}

/// Tie the field together for assistive technology: its first control is named by the label
/// and described by the message, which is the control's error when the status says so. A
/// validation message is a live region, so it is spoken when it appears or changes.
fn describe(
    ui: &mut Ui<'_>,
    [label, controls, text]: [Option<usize>; 3],
    message: &str,
    status: SemanticStatus,
    validated: bool,
) {
    let mut id_of = |index: Option<usize>| Some(ui.context.a11y_node_mut(index?)?.id);
    let (label, message_id) = (id_of(label), id_of(text));
    let error = status == SemanticStatus::Error;
    if let Some(node) = text.and_then(|index| ui.context.a11y_node_mut(index)) {
        if validated {
            node.live(if error {
                AccessLive::Assertive
            } else {
                AccessLive::Polite
            });
        }
    }
    let control = controls.and_then(|start| ui.context.a11y_find(start, |n| is_control(n.role)));
    let Some(control) = control else {
        return;
    };
    let id = control.id;
    if let Some(label) = label {
        control.labelled_by(label);
    }
    if error {
        control.invalid(true);
    }
    if let Some(text) = message_id {
        control.described_by(text);
        if error && validated {
            control.error_message(text);
        }
        // Not every platform follows the relation; all of them speak a description.
        if control.description.is_none() {
            control.description(message);
        }
    }
    // A part named after its control (the text field of a number input) is not named
    // through the control's own relation: it takes the label as well.
    let Some(label) = label else {
        return;
    };
    for index in controls.unwrap_or(0)..ui.context.a11y_len() {
        let named = |node: &&mut AccessNode| {
            let more = node.more.as_ref();
            more.is_some_and(|more| more.labelled_by.contains(&id))
        };
        if let Some(node) = ui.context.a11y_node_mut(index).filter(named) {
            node.labelled_by(label);
        }
    }
}

/// Hints are muted; a status message takes its status color.
fn status_text(style: &crate::Style, status: SemanticStatus, has_message: bool) -> Color {
    match status {
        _ if !has_message => style.muted_text,
        SemanticStatus::Normal => style.muted_text,
        SemanticStatus::Success => style.success,
        SemanticStatus::Warning => style.warning,
        SemanticStatus::Error => style.error,
    }
}
