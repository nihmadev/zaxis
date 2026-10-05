use std::hash::Hash;

use crate::{components::theme::SegmentedStyle, CornerRadius, Id, ImageSource};

use super::super::SemanticStatus;

/// One choice of a [`SegmentedControl`](super::SegmentedControl). Its identity is the hash of
/// `value`, never its position or label, so reordering options keeps focus and animation.
pub struct SegmentOption<T> {
    pub(super) value: T,
    pub(super) label: String,
    pub(super) icon: Option<ImageSource>,
    pub(super) enabled: bool,
    pub(super) tooltip: Option<String>,
}

impl<T> SegmentOption<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            icon: None,
            enabled: true,
            tooltip: None,
        }
    }
    /// An icon left of the label, tinted with the label color. Either part may be empty.
    pub fn icon(mut self, icon: impl Into<ImageSource>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    /// A disabled option is dimmed, ignores input and is skipped by the keyboard. When it is
    /// the selected one it stays selected.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Shown on hover; defaults to the label when it is truncated or hidden.
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

impl<T, L: Into<String>> From<(T, L)> for SegmentOption<T> {
    fn from((value, label): (T, L)) -> Self {
        Self::new(value, label)
    }
}

impl<T, L: Into<String>, I: Into<ImageSource>> From<(T, L, I)> for SegmentOption<T> {
    fn from((value, label, icon): (T, L, I)) -> Self {
        Self::new(value, label).icon(icon)
    }
}

/// Overall look. Both variants share sizing, keyboard and animation behaviour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SegmentedVariant {
    /// A grey plate with a raised thumb that slides to the selected segment.
    #[default]
    Raised,
    /// Material 3 style: a pill with a hairline outline and dividers; the selected
    /// segment gets a tonal fill and a check mark, and fills fade in place.
    Outlined,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SegmentedOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Height preset; the height scales `Style::control_height`, text and icons follow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SegmentedSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl SegmentedSize {
    pub(super) fn factor(self) -> f32 {
        match self {
            Self::Small => 0.8,
            Self::Medium => 1.0,
            Self::Large => 1.25,
        }
    }
}

/// How wide the segments are. When the row does not fit its container, every mode shrinks
/// the segments uniformly and truncates labels with an ellipsis; if labels of segments with
/// icons would still be unreadable, those labels are hidden and the icons remain.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SegmentWidth {
    /// All segments as wide as the widest content.
    #[default]
    Equal,
    /// Each segment fits its own content.
    Content,
    /// All segments take this width (logical pixels).
    Fixed(f32),
    /// Equal segments that fill the available width.
    Fill,
}

/// Everything the builders can set, resolved once per frame.
pub(super) struct Config {
    pub id: Option<Id>,
    pub enabled: bool,
    pub status: SemanticStatus,
    pub orientation: SegmentedOrientation,
    pub size: SegmentedSize,
    pub variant: SegmentedVariant,
    pub width: SegmentWidth,
    pub min_width: Option<f32>,
    pub icon_only: bool,
    pub follow_focus: bool,
    pub style: SegmentedStyle,
    pub rounding: Option<CornerRadius>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            id: None,
            enabled: true,
            status: SemanticStatus::Normal,
            orientation: SegmentedOrientation::Horizontal,
            size: SegmentedSize::Medium,
            variant: SegmentedVariant::Raised,
            width: SegmentWidth::Equal,
            min_width: None,
            icon_only: false,
            follow_focus: true,
            style: SegmentedStyle::default(),
            rounding: None,
        }
    }
}
