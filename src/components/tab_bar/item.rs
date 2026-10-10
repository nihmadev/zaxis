use std::hash::Hash;

use crate::{Id, ImageSource};

/// One tab of a [`TabBar`](crate::TabBar). Its identity is the hash of `value`, never its
/// position or label, so reordering, renaming and moving between bars keep its focus,
/// animation and state.
pub struct TabItem<T> {
    pub(super) value: T,
    pub(super) label: String,
    pub(super) icon: Option<ImageSource>,
    pub(super) closable: Option<bool>,
    pub(super) enabled: bool,
    pub(super) tooltip: Option<String>,
}

impl<T> TabItem<T> {
    /// A tab for `value`. An empty label with an [`icon`](Self::icon) makes a tab of the
    /// icon alone, which needs a [`tooltip`](Self::tooltip): it names the tab for assistive
    /// technology and shows on hover.
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            icon: None,
            closable: None,
            enabled: true,
            tooltip: None,
        }
    }
    /// An icon left of the label, tinted with the label color.
    pub fn icon(mut self, icon: impl Into<ImageSource>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    /// Whether this tab has a close button, overriding [`TabBar::closable`](crate::TabBar::closable).
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = Some(closable);
        self
    }
    /// A disabled tab is dimmed, takes no input, cannot be closed or dragged, and the
    /// keyboard skips it. When it is the selected one it stays selected.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Shown on hover; the full label when it is cut short.
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

impl<T, L: Into<String>> From<(T, L)> for TabItem<T> {
    fn from((value, label): (T, L)) -> Self {
        Self::new(value, label)
    }
}

impl<T, L: Into<String>, I: Into<ImageSource>> From<(T, L, I)> for TabItem<T> {
    fn from((value, label, icon): (T, L, I)) -> Self {
        Self::new(value, label).icon(icon)
    }
}
