//! Keyboard handling for the focused list: movement, selection keys and type-ahead.
use super::{
    model::ListModel,
    select::{Mods, Selector},
    ListEvent, ListMode,
};
use crate::time::Instant;
use std::{collections::HashSet, time::Duration};
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
use winit::keyboard::{KeyCode, ModifiersState};

pub(super) struct KeyInput<'a> {
    pub keys: &'a HashSet<KeyCode>,
    pub mods: ModifiersState,
    pub text: &'a str,
    pub now: Instant,
    pub timeout: Duration,
    pub follows: bool,
}

pub(super) fn mods(state: ModifiersState) -> Mods {
    Mods {
        shift: state.shift_key(),
        toggle: state.control_key() || state.super_key(),
    }
}

fn selectable(model: &impl ListModel, i: usize) -> bool {
    model.entry(i).selectable()
}
/// The next selectable row after (or before) `from`; `None` starts at the first (last) one.
fn step(model: &impl ListModel, from: Option<usize>, down: bool) -> Option<usize> {
    let len = model.len();
    match (from, down) {
        (Some(i), true) => (i + 1..len).find(|i| selectable(model, *i)),
        (Some(i), false) => (0..i.min(len)).rev().find(|i| selectable(model, *i)),
        (None, true) => (0..len).find(|i| selectable(model, *i)),
        (None, false) => (0..len).rev().find(|i| selectable(model, *i)),
    }
}
/// The selectable row nearest `at`, preferring the travel direction.
fn nearest(model: &impl ListModel, at: usize, down: bool) -> Option<usize> {
    let at = at.min(model.len().saturating_sub(1));
    if selectable(model, at) {
        return Some(at);
    }
    step(model, Some(at), down).or_else(|| step(model, Some(at), !down))
}

/// Case-insensitive prefix test that only accepts a match ending on a grapheme boundary,
/// so a typed `e` does not match the `e` of a decomposed `é`.
pub(super) fn prefix_matches(text: &str, needle: &[char]) -> bool {
    let mut matched = 0;
    for (at, c) in text.char_indices() {
        for lower in c.to_lowercase() {
            if matched >= needle.len() || needle[matched] != lower {
                return false;
            }
            matched += 1;
        }
        if matched == needle.len() {
            let end = at + c.len_utf8();
            return GraphemeCursor::new(end, text.len(), true)
                .is_boundary(text, 0)
                .unwrap_or(false);
        }
    }
    false
}

impl<M: ListModel> Selector<'_, M> {
    /// Returns true when the keys were consumed. `page` maps (row, down) to the row a page
    /// away.
    pub fn keyboard(&mut self, input: &KeyInput<'_>, page: impl Fn(usize, bool) -> usize) -> bool {
        let model = self.model;
        let keys = input.keys;
        let m = mods(input.mods);
        let active = self.state.active_index(model);
        if m.toggle && keys.contains(&KeyCode::KeyA) {
            self.select_all();
            return true;
        }
        if keys.contains(&KeyCode::Escape) {
            self.state.typed.clear();
            self.clear();
            return true;
        }
        if keys.contains(&KeyCode::Enter) || keys.contains(&KeyCode::NumpadEnter) {
            if let Some(i) = active.filter(|i| selectable(model, *i)) {
                self.events.push(ListEvent::Activated(model.entry(i).key));
            }
            return true;
        }
        let down = keys.contains(&KeyCode::ArrowDown);
        let up = keys.contains(&KeyCode::ArrowUp);
        let target = if down != up {
            step(model, active, down)
                .or_else(|| active.filter(|i| selectable(model, *i)))
                .or_else(|| step(model, active, !down))
        } else if keys.contains(&KeyCode::Home) {
            step(model, None, true)
        } else if keys.contains(&KeyCode::End) {
            step(model, None, false)
        } else {
            let (page_down, page_up) = (
                keys.contains(&KeyCode::PageDown),
                keys.contains(&KeyCode::PageUp),
            );
            let from = active.unwrap_or(0);
            (page_down != page_up)
                .then(|| nearest(model, page(from, page_down), page_down))
                .flatten()
        };
        let navigates = down != up
            || [
                KeyCode::Home,
                KeyCode::End,
                KeyCode::PageUp,
                KeyCode::PageDown,
            ]
            .iter()
            .any(|k| keys.contains(k));
        if navigates {
            self.state.typed.clear();
            if let Some(t) = target {
                let select = input.follows && !m.toggle && self.mode != ListMode::Checks;
                self.moved(active, t, select, m.shift);
            }
            return true;
        }
        let typing = !self.state.typed.is_empty() && input.text.contains(' ');
        if keys.contains(&KeyCode::Space) && !typing {
            if let Some(i) = active.filter(|i| selectable(model, *i)) {
                self.toggle_active(i);
            }
            return true;
        }
        self.type_ahead(input, active)
    }

    fn type_ahead(&mut self, input: &KeyInput<'_>, active: Option<usize>) -> bool {
        if input.mods.control_key() || input.mods.alt_key() || input.mods.super_key() {
            return false;
        }
        let typed: String = input.text.chars().filter(|c| !c.is_control()).collect();
        if typed.is_empty() {
            return false;
        }
        let state = &mut *self.state;
        if state
            .typed_at
            .is_none_or(|at| input.now.saturating_duration_since(at) > input.timeout)
        {
            state.typed.clear();
        }
        state.typed.push_str(&typed);
        state.typed_at = Some(input.now);
        let graphemes: Vec<&str> = state.typed.graphemes(true).collect();
        // The same key pressed repeatedly cycles through the rows starting with it.
        let cycle = graphemes.len() > 1 && graphemes.iter().all(|g| *g == graphemes[0]);
        let needle: Vec<char> = if cycle { graphemes[0] } else { &state.typed }
            .chars()
            .flat_map(char::to_lowercase)
            .collect();
        let len = self.model.len();
        let from = match (active, cycle || graphemes.len() == 1) {
            (Some(i), true) => i + 1,
            (Some(i), false) => i,
            (None, _) => 0,
        };
        let hit = (from..len).chain(0..from.min(len)).find(|i| {
            let e = self.model.entry(*i);
            e.selectable() && prefix_matches(e.text, &needle)
        });
        if let Some(i) = hit {
            let select =
                input.follows && matches!(self.mode, ListMode::Single | ListMode::Multiple);
            self.moved(active, i, select, false);
        }
        true
    }
}
