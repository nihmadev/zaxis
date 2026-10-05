use super::{heights::HeightIndex, model::ListModel};
use crate::time::Instant;
use crate::Id;
use std::collections::HashSet;

/// Retained per list in the `Context`; dropped on the first pass the list is not shown.
#[derive(Default)]
pub struct ListState {
    pub last_frame: u64,
    pub heights: HeightIndex,
    pub active: Option<Id>,
    pub active_hint: usize,
    /// Shift ranges extend from here.
    pub anchor: Option<Id>,
    pub anchor_hint: usize,
    pub typed: String,
    pub typed_at: Option<Instant>,
    /// Scroll the active row into view on the next pass.
    pub reveal: bool,
    /// Selection when the application does not pass one.
    pub owned: HashSet<Id>,
    pub viewport_height: f32,
    /// First visible row, by key, and how far its top is above the viewport top.
    pub scroll_anchor: Option<(Id, f32)>,
    pub scroll_hint: usize,
    /// A reveal that missed (rows were measured after it was computed) and is retried.
    pub retarget: Option<(Id, u8)>,
    pub loaded_len: Option<usize>,
    pub owner_focused: bool,
    /// The focus ring was showing on the previous pass.
    pub ring: bool,
    pub drag_hint: usize,
}

impl ListState {
    /// Where `key` is now. The remembered `hint` is tried first, then the nearest entries
    /// outward, so a few insertions or removals nearby cost a few comparisons.
    pub fn find(model: &impl ListModel, key: Id, hint: usize) -> Option<usize> {
        let len = model.len();
        let hint = hint.min(len.saturating_sub(1));
        (0..len).find_map(|step| {
            [hint.checked_sub(step), hint.checked_add(step)]
                .into_iter()
                .flatten()
                .find(|i| *i < len && model.entry(*i).key == key)
        })
    }
    pub fn active_index(&self, model: &impl ListModel) -> Option<usize> {
        Self::find(model, self.active?, self.active_hint)
    }
    pub fn set_active(&mut self, index: usize, model: &impl ListModel) {
        self.active = Some(model.entry(index).key);
        self.active_hint = index;
        self.reveal = true;
    }
    /// After a model change keep the active row, or move to its nearest selectable
    /// neighbour when it vanished or became disabled.
    pub fn settle_active(&mut self, model: &impl ListModel) {
        let Some(key) = self.active else { return };
        let found = Self::find(model, key, self.active_hint);
        if let Some(i) = found.filter(|i| model.entry(*i).selectable()) {
            self.active_hint = i;
            return;
        }
        let from = found.unwrap_or(self.active_hint).min(model.len());
        let near = (from..model.len())
            .chain((0..from).rev())
            .find(|i| model.entry(*i).selectable());
        match near {
            Some(i) => {
                self.active = Some(model.entry(i).key);
                self.active_hint = i;
                self.reveal = true;
            }
            None => self.active = None,
        }
    }
}
