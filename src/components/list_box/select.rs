//! Selection rules shared by the pointer and the keyboard. The set belongs to the application;
//! every operation reports a change at most once.
use super::{model::ListModel, state::ListState, ListEvent, ListMode};
use crate::Id;
use std::collections::HashSet;

#[derive(Clone, Copy, Default)]
pub(super) struct Mods {
    pub shift: bool,
    /// Ctrl, or Cmd on macOS.
    pub toggle: bool,
}

pub(super) struct Selector<'a, M> {
    pub model: &'a M,
    pub mode: ListMode,
    pub set: &'a mut HashSet<Id>,
    pub state: &'a mut ListState,
    pub events: &'a mut Vec<ListEvent>,
}

impl<M: ListModel> Selector<'_, M> {
    fn replace(&mut self, keys: impl IntoIterator<Item = Id>) {
        let next: HashSet<Id> = keys.into_iter().collect();
        if next != *self.set {
            *self.set = next;
            self.events.push(ListEvent::SelectionChanged);
        }
    }
    fn extend(&mut self, keys: impl IntoIterator<Item = Id>) {
        let before = self.set.len();
        self.set.extend(keys);
        if self.set.len() != before {
            self.events.push(ListEvent::SelectionChanged);
        }
    }
    fn toggle(&mut self, key: Id) {
        if !self.set.remove(&key) {
            self.set.insert(key);
        }
        self.events.push(ListEvent::SelectionChanged);
    }
    pub fn clear(&mut self) {
        if !self.set.is_empty() {
            self.set.clear();
            self.events.push(ListEvent::SelectionChanged);
        }
    }
    /// Selectable keys between two indices, inclusive.
    fn span(&self, a: usize, b: usize) -> Vec<Id> {
        (a.min(b)..=a.max(b))
            .map(|i| self.model.entry(i))
            .filter(|e| e.selectable())
            .map(|e| e.key)
            .collect()
    }
    fn anchor_index(&self, fallback: usize) -> usize {
        self.state
            .anchor
            .and_then(|k| ListState::find(self.model, k, self.state.anchor_hint))
            .unwrap_or(fallback)
    }
    fn set_anchor(&mut self, index: usize) {
        self.state.anchor = Some(self.model.entry(index).key);
        self.state.anchor_hint = index;
    }
    /// A pointer click on a selectable row.
    pub fn click(&mut self, index: usize, mods: Mods) {
        let key = self.model.entry(index).key;
        self.state.set_active(index, self.model);
        self.state.reveal = false;
        match self.mode {
            ListMode::None => {}
            ListMode::Single => self.replace([key]),
            ListMode::Multiple if mods.shift => {
                let span = self.span(self.anchor_index(index), index);
                if mods.toggle {
                    self.extend(span);
                } else {
                    self.replace(span);
                }
            }
            ListMode::Multiple if mods.toggle => {
                self.toggle(key);
                self.set_anchor(index);
            }
            ListMode::Multiple => {
                self.replace([key]);
                self.set_anchor(index);
            }
            ListMode::Checks if mods.shift => {
                let span = self.span(self.anchor_index(index), index);
                self.extend(span);
            }
            ListMode::Checks => {
                self.toggle(key);
                self.set_anchor(index);
            }
        }
    }
    /// Keyboard movement onto `index`: `select` replaces the selection, `extend` makes the
    /// range from the anchor the selection (Shift+arrows).
    pub fn moved(&mut self, from: Option<usize>, index: usize, select: bool, extend: bool) {
        let key = self.model.entry(index).key;
        self.state.set_active(index, self.model);
        match self.mode {
            ListMode::Multiple if extend => {
                let anchor = self.anchor_index(from.unwrap_or(index));
                if self.state.anchor.is_none() {
                    self.set_anchor(anchor);
                }
                let span = self.span(anchor, index);
                self.replace(span);
            }
            ListMode::Single | ListMode::Multiple if select => {
                self.replace([key]);
                self.set_anchor(index);
            }
            _ => {}
        }
    }
    /// Space on the active row.
    pub fn toggle_active(&mut self, index: usize) {
        let key = self.model.entry(index).key;
        match self.mode {
            ListMode::None => {}
            ListMode::Single => self.replace([key]),
            ListMode::Multiple | ListMode::Checks => self.toggle(key),
        }
        self.set_anchor(index);
    }
    pub fn select_all(&mut self) {
        if matches!(self.mode, ListMode::Multiple | ListMode::Checks) {
            let all: Vec<Id> = (0..self.model.len())
                .map(|i| self.model.entry(i))
                .filter(|e| e.selectable())
                .map(|e| e.key)
                .collect();
            self.extend(all);
        }
    }
}
