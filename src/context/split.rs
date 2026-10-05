//! Split boundaries: pointer drags (a double press resets), keys of the focused boundary
//! and the retained state of every split pane, through the existing capture, focus and
//! transform routing.
use super::{gesture::ClickCounter, Context, HitAction, Id};
use crate::{
    components::split_pane::{SplitInput, SplitState},
    Vec2,
};
use std::collections::HashMap;
use winit::keyboard::KeyCode;

#[derive(Default)]
pub(crate) struct Splits {
    pub(crate) states: HashMap<Id, SplitState>,
    /// Input waiting for each boundary's next pass.
    input: HashMap<Id, Vec<SplitInput>>,
    /// Presses on one boundary: a second press in place is a double press.
    clicks: ClickCounter,
}

impl Splits {
    pub(super) fn retire(&mut self, frame: u64) {
        self.states.retain(|_, state| state.last_frame == frame);
    }

    pub(super) fn clear_input(&mut self) {
        self.input.clear();
    }

    pub(super) fn pending(&self) -> usize {
        self.input.len()
    }
}

/// Where a captured boundary drag is.
#[derive(Clone, Copy)]
pub(super) enum Phase {
    Press,
    Move,
    Release,
}

impl Context {
    pub(super) fn split_pointer(&mut self, id: Id, pointer: Vec2, phase: Phase) {
        let event = match phase {
            Phase::Press => {
                let now = crate::time::Instant::now();
                let double = self.splits.clicks.count(id, pointer, now, 2, true) == 2;
                SplitInput::Begin(pointer, double)
            }
            Phase::Release => SplitInput::End(pointer),
            Phase::Move => {
                self.splits.clicks.moved(pointer);
                SplitInput::Move(pointer)
            }
        };
        self.splits.input.entry(id).or_default().push(event);
    }
    pub(crate) fn take_split_input(&mut self, id: Id) -> Vec<SplitInput> {
        let inverse = self.visuals.to_local(id);
        self.splits
            .input
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|event| match event {
                SplitInput::Begin(p, double) => SplitInput::Begin(inverse.point(p), double),
                SplitInput::Move(p) => SplitInput::Move(inverse.point(p)),
                SplitInput::End(p) => SplitInput::End(inverse.point(p)),
                event => event,
            })
            .collect()
    }
    pub(crate) fn cancel_split_capture(&mut self, id: Id) {
        if self.interaction.capture.is_some_and(|c| {
            c.hit.id == id && matches!(c.hit.action, HitAction::SplitResize { .. })
        }) {
            self.interaction.capture = None;
        }
    }
    pub(super) fn split_key(&mut self, code: KeyCode, pressed: bool) -> bool {
        let Some(id) = self.interaction.focused else {
            return false;
        };
        let Some(vertical) = self.interaction.previous_hits.iter().find_map(|h| {
            if h.id != id {
                return None;
            }
            match h.action {
                HitAction::SplitResize { vertical } => Some(vertical),
                _ => None,
            }
        }) else {
            return false;
        };
        let supported = matches!(code, KeyCode::Home | KeyCode::Backspace | KeyCode::Escape)
            || if vertical {
                matches!(code, KeyCode::ArrowUp | KeyCode::ArrowDown)
            } else {
                matches!(code, KeyCode::ArrowLeft | KeyCode::ArrowRight)
            };
        if !supported {
            return false;
        }
        self.input.record_key(code, pressed);
        if pressed {
            if code == KeyCode::Escape {
                self.cancel_split_capture(id);
            } else {
                let event = SplitInput::Key(code, self.input.modifiers);
                self.splits.input.entry(id).or_default().push(event);
            }
        }
        true
    }
}
