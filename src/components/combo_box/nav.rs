//! Keyboard navigation: the highlight moves over the enabled options the filter lists.

use super::ComboBoxOption;
use crate::Id;
use winit::keyboard::KeyCode;

/// The enabled options among the listed ones, in list order: the only ones the keyboard
/// reaches.
pub(super) struct Navigable<'s, T> {
    options: &'s [ComboBoxOption<T>],
    indices: Vec<usize>,
}

/// What one pass's keys did.
pub(super) struct Navigation {
    /// The highlight moved and is scrolled into view.
    pub(super) moved: bool,
    /// Enter or Space chose the highlighted option.
    pub(super) chosen: Option<usize>,
}

impl<'s, T> Navigable<'s, T> {
    pub(super) fn new(options: &'s [ComboBoxOption<T>], listed: &[usize]) -> Self {
        let indices = listed
            .iter()
            .copied()
            .filter(|&i| options[i].enabled)
            .collect();
        Self { options, indices }
    }

    fn position(&self, active: Option<Id>) -> Option<usize> {
        self.indices
            .iter()
            .position(|&i| Some(self.options[i].id) == active)
    }

    /// A highlight that the filter or a change of the options took away goes to the
    /// first navigable option, or nowhere. Returns whether it moved.
    pub(super) fn restore(&self, active: &mut Option<Id>) -> bool {
        if self.position(*active).is_some() {
            return false;
        }
        *active = self.indices.first().map(|&i| self.options[i].id);
        true
    }

    /// Moves the highlight by `keys` in order, page keys by `page` rows. Enter or Space
    /// chooses the highlighted option and ends the pass's navigation.
    pub(super) fn navigate(
        &self,
        keys: &[KeyCode],
        page: usize,
        active: &mut Option<Id>,
    ) -> Navigation {
        let mut navigation = Navigation {
            moved: false,
            chosen: None,
        };
        for key in keys {
            if matches!(key, KeyCode::Enter | KeyCode::Space) {
                navigation.chosen = self.position(*active).map(|at| self.indices[at]);
                break;
            }
            let current = self.position(*active).unwrap_or(0);
            let last = self.indices.len().saturating_sub(1);
            let next = match key {
                KeyCode::ArrowDown => (current + 1).min(last),
                KeyCode::ArrowUp => current.saturating_sub(1),
                KeyCode::Home => 0,
                KeyCode::End => last,
                KeyCode::PageDown => (current + page).min(last),
                KeyCode::PageUp => current.saturating_sub(page),
                _ => current,
            };
            *active = self.indices.get(next).map(|&i| self.options[i].id);
            navigation.moved = true;
        }
        navigation
    }
}
