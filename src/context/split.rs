//! Split boundaries participate in the existing capture/focus/transform routing.
use super::{Context, HitAction, Id};
use crate::{components::split_pane::SplitInput, Vec2};
use winit::keyboard::KeyCode;

impl Context {
    pub(super) fn split_pointer(&mut self, id: Id, pointer: Vec2, phase: u8) {
        let event = match phase {
            1 => {
                let now = crate::time::Instant::now();
                let double = self.split_click.as_ref().is_some_and(|last| {
                    last.id == id
                        && now.duration_since(last.time).as_millis() <= 500
                        && (last.position - pointer).length_squared() <= 16.0
                        && last.count == 1
                });
                self.split_click = Some(super::interaction::ClickSequence {
                    id,
                    position: pointer,
                    time: now,
                    count: if double { 2 } else { 1 },
                });
                SplitInput::Begin(pointer, double)
            }
            2 => SplitInput::End(pointer),
            _ => {
                if self
                    .split_click
                    .as_ref()
                    .is_some_and(|last| (last.position - pointer).length_squared() > 16.0)
                {
                    self.split_click = None;
                }
                SplitInput::Move(pointer)
            }
        };
        self.split_input.entry(id).or_default().push(event);
    }
    pub(crate) fn take_split_input(&mut self, id: Id) -> Vec<SplitInput> {
        let inverse = self
            .input_transforms
            .get(&id)
            .copied()
            .unwrap_or_default()
            .inverse();
        self.split_input
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
        if self.capture.is_some_and(|c| {
            c.hit.id == id && matches!(c.hit.action, HitAction::SplitResize { .. })
        }) {
            self.capture = None;
        }
    }
    pub(super) fn split_key(&mut self, code: KeyCode, pressed: bool) -> bool {
        let Some(id) = self.focused_widget else {
            return false;
        };
        let Some(vertical) = self.previous_hits.iter().find_map(|h| {
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
        if pressed {
            self.input.keys_down.insert(code);
            self.input.keys_pressed.insert(code);
            if code == KeyCode::Escape {
                self.cancel_split_capture(id);
            } else {
                self.split_input
                    .entry(id)
                    .or_default()
                    .push(SplitInput::Key(code, self.input.modifiers));
            }
        } else {
            self.input.keys_down.remove(&code);
            self.input.keys_released.insert(code);
        }
        true
    }
}
