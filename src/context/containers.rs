//! Retained state of container widgets: grids, cards, wrapped flows, tab pages, carousels,
//! list boxes and collapsing headers. Each entry lives while its widget is built, except
//! that a collapsing header hidden by a collapsed ancestor lives as long as the ancestor.

use super::Id;
use crate::components::{
    card::CardState, carousel::CarouselState, collapsing_header::CollapsingState, grid::GridState,
    list_box::ListState, motion::TabPagesState, ui::FlowState,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct Containers {
    pub(crate) grids: HashMap<Id, GridState>,
    pub(crate) cards: HashMap<Id, CardState>,
    /// Measured rows of horizontal wrapping layouts.
    pub(crate) flows: HashMap<Id, FlowState>,
    pub(crate) tab_pages: HashMap<Id, TabPagesState>,
    pub(crate) carousels: HashMap<Id, CarouselState>,
    pub(crate) list_boxes: HashMap<Id, ListState>,
    pub(crate) collapsing_headers: HashMap<Id, CollapsingState>,
}

impl Containers {
    /// Containers not built in pass `frame` lose their state.
    pub(super) fn retire(&mut self, frame: u64) {
        self.grids.retain(|_, state| state.last_frame == frame);
        self.cards.retain(|_, state| state.last_frame == frame);
        self.flows.retain(|_, state| state.last_frame == frame);
        self.tab_pages.retain(|_, state| state.last_frame == frame);
        self.carousels.retain(|_, state| state.last_frame == frame);
        self.list_boxes.retain(|_, state| state.last_frame == frame);
        self.retire_collapsing_headers(frame);
    }

    /// Keep retained nested headers while an ancestor hides them. Removing the entire
    /// ancestor releases the group. No content callbacks run here.
    fn retire_collapsing_headers(&mut self, frame: u64) {
        let headers = &mut self.collapsing_headers;
        let mut live: Vec<_> = headers
            .iter()
            .filter(|(_, state)| state.last_frame == frame)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = HashSet::new();
        while let Some(id) = live.pop() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(state) = headers.get_mut(&id) {
                state.last_frame = frame;
                live.extend(state.descendants.iter().copied());
            }
        }
        headers.retain(|_, state| state.last_frame == frame);
    }
}
