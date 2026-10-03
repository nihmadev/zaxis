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
