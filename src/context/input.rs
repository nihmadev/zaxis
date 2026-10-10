//! Input accumulated between UI passes and event response types.

use crate::Vec2;
use std::collections::HashSet;
use winit::keyboard::{KeyCode, ModifiersState};

/// Input accumulated until the next UI pass. Pointer coordinates are logical pixels.
#[derive(Clone, Debug, Default)]
pub struct InputState {
    pub pointer: Option<Vec2>,
    pub primary_down: bool,
    pub middle_down: bool,
    pub secondary_down: bool,
    pub secondary_pressed: bool,
    pub secondary_released: bool,
    pub primary_pressed: bool,
    pub primary_released: bool,
    pub scroll_delta: Vec2,
    pub keys_down: HashSet<KeyCode>,
    pub keys_pressed: HashSet<KeyCode>,
    pub keys_released: HashSet<KeyCode>,
    pub modifiers: ModifiersState,
    pub text: String,
    pub focused: bool,
}

impl InputState {
    /// Note a key routed to a control in the pass's key sets.
    pub(super) fn record_key(&mut self, code: KeyCode, pressed: bool) {
        if pressed {
            self.keys_down.insert(code);
            self.keys_pressed.insert(code);
        } else {
            self.keys_down.remove(&code);
            self.keys_released.insert(code);
        }
    }

    /// A key the dispatcher itself acted on (group navigation moved focus): it stays held,
    /// but the control that now has focus must not read the same press and act on it too.
    pub(super) fn unpress(&mut self, code: KeyCode) {
        self.keys_pressed.remove(&code);
    }

    pub(super) fn finish_frame(&mut self) {
        self.primary_pressed = false;
        self.secondary_pressed = false;
        self.secondary_released = false;
        self.primary_released = false;
        self.scroll_delta = Vec2::ZERO;
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.text.clear();
    }
}

/// Result of event handling. Hosts should request a redraw when `repaint` is true.
#[derive(Clone, Copy, Debug, Default)]
pub struct EventResponse {
    pub consumed: bool,
    pub repaint: bool,
}

/// Counters of keys addressed to widgets and of focus groups, for profiling and tests.
/// Every figure is bounded: the queue, the held keys and the declarations do not grow
/// with the time since the last pass or with the number of widgets ever seen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputStats {
    /// Widgets whose key claims the last pass published.
    pub key_claims: usize,
    /// Events queued for their owners until the next pass.
    pub key_events_pending: usize,
    /// Keys held down whose release an owner will take.
    pub keys_owned: usize,
    /// Events dropped because the queue was full, since the context was made.
    pub key_events_dropped: u64,
    /// Focus groups the last pass published.
    pub focus_groups: usize,
    /// Regions and nested groups that belong to those groups.
    pub focus_group_members: usize,
}

impl super::Context {
    pub fn input_stats(&self) -> InputStats {
        InputStats {
            key_claims: self.keys.declared(),
            key_events_pending: self.keys.pending(),
            keys_owned: self.keys.owned(),
            key_events_dropped: self.keys.dropped(),
            focus_groups: self.focus_groups.len(),
            focus_group_members: self.focus_groups.members(),
        }
    }
}
