//! Field validation state and the compact `Field` layout: label, control, message.
//!
//! The library validates nothing and stores no rules. The application computes a
//! [`Validation`] every pass and hands it to a [`Field`] (or marks one control
//! with `.status(..)`); a changing message never requests frames by itself.

use std::{fmt::Display, hash::Hash};

use super::{SemanticStatus, Text, TypographyRole, Ui};
use crate::{Color, Id, Vec2};

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
                if !self.label.is_empty() {
                    ui.add(
                        Text::new(self.label.clone())
                            .typography(TypographyRole::Small)
                            .color(style.text_color),
                    );
                }
                let previous = std::mem::replace(&mut ui.context.field_status, status);
                let result = build(ui);
                ui.context.field_status = previous;
                if !message.is_empty() {
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
                result
            })
        })
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
