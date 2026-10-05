use std::hash::Hash;

use super::super::SemanticStatus;
use crate::{components::theme::RadioStyle, Id};

/// One choice of a [`RadioGroup`](super::RadioGroup). Its identity is the hash of `value`,
/// never its position or label, so reordering or removing options keeps focus and animation.
pub struct RadioOption<T> {
    pub(super) value: T,
    pub(super) label: String,
    pub(super) description: String,
    pub(super) enabled: bool,
    pub(super) tooltip: Option<String>,
}

impl<T> RadioOption<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            description: String::new(),
            enabled: true,
            tooltip: None,
        }
    }
    /// A second, muted line below the label; wraps like the label.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = text.into();
        self
    }
    /// A disabled option is faded, ignores input and is skipped by the keyboard. When it is
    /// the selected one it stays selected.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Shown while the pointer rests on the row.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
    pub(super) fn key(&self) -> Id
    where
        T: Hash,
    {
        Id::new(&self.value)
    }
}

impl<T, L: Into<String>> From<(T, L)> for RadioOption<T> {
    fn from((value, label): (T, L)) -> Self {
        Self::new(value, label)
    }
}

impl<T, L: Into<String>, D: Into<String>> From<(T, L, D)> for RadioOption<T> {
    fn from((value, label, description): (T, L, D)) -> Self {
        Self::new(value, label).description(description)
    }
}

/// How the rows are arranged. Arrow keys follow it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadioLayout {
    /// One column; Up/Down move the focus.
    #[default]
    Vertical,
    /// One row; Left/Right move the focus. Labels wrap when the row does not fit.
    Horizontal,
    /// Row-major grid with this many columns (at least 1); all four arrows move the focus.
    Grid(usize),
}

/// Indicator and text size preset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadioSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl RadioSize {
    pub(super) fn factor(self) -> f32 {
        match self {
            Self::Small => 0.8,
            Self::Medium => 1.0,
            Self::Large => 1.25,
        }
    }
}

/// Everything the builders can set, resolved once per frame.
pub(super) struct Config {
    pub id: Option<Id>,
    pub enabled: bool,
    pub status: SemanticStatus,
    pub layout: RadioLayout,
    pub size: RadioSize,
    pub follow_focus: bool,
    pub row_gap: Option<f32>,
    pub column_gap: Option<f32>,
    pub label_gap: Option<f32>,
    pub min_row_height: Option<f32>,
    pub style: RadioStyle,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            id: None,
            enabled: true,
            status: SemanticStatus::Normal,
            layout: RadioLayout::Vertical,
            size: RadioSize::Medium,
            follow_focus: true,
            row_gap: None,
            column_gap: None,
            label_gap: None,
            min_row_height: None,
            style: RadioStyle::default(),
        }
    }
}
