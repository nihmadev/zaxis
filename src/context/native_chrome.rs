use super::{Context, Id};
use crate::{Rect, Vec2};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    window::{ResizeDirection, Window},
};

pub struct NativeChrome {
    pub owner: Id,
    pub drag: Rect,
    pub resize_border: f32,
}

impl Context {
    pub(crate) fn native_chrome_press(&mut self, event: &WindowEvent, window: &Window) -> bool {
        if window.is_decorated() {
            return false;
        }
        if !matches!(
            event,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            }
        ) {
            return false;
        }
        let Some(pointer) = self.input.pointer else {
            return false;
        };
        let Some(chrome) = &self.native_chrome else {
            return false;
        };
        let top = self.top_window(pointer);
        if top != Some(chrome.owner) && !top.is_some_and(|id| self.is_modal_layer(id))
            || self.capture.is_some()
        {
            return false;
        }
        // Launch on MouseInput, before press/release can be batched into a redraw.
        let direction = if window.is_resizable() && !window.is_maximized() {
            resize_direction(self.viewport(), pointer, chrome.resize_border)
        } else {
            None
        };
        let result = if let Some(direction) = direction {
            window.drag_resize_window(direction)
        } else if chrome.drag.contains(pointer) {
            window.drag_window()
        } else {
            return false;
        };
        if let Err(error) = result {
            self.report(crate::DiagnosticKind::External, None, None, || {
                format!("native window interaction failed: {error}")
            });
        }
        true
    }
}

pub fn resize_direction(rect: Rect, pointer: Vec2, border: f32) -> Option<ResizeDirection> {
    if border <= 0.0 || !rect.contains(pointer) {
        return None;
    }
    let left = pointer.x < rect.min.x + border;
    let right = pointer.x >= rect.max.x - border;
    let top = pointer.y < rect.min.y + border;
    let bottom = pointer.y >= rect.max.y - border;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(ResizeDirection::NorthWest),
        (_, true, true, _) => Some(ResizeDirection::NorthEast),
        (true, _, _, true) => Some(ResizeDirection::SouthWest),
        (_, true, _, true) => Some(ResizeDirection::SouthEast),
        (true, _, _, _) => Some(ResizeDirection::West),
        (_, true, _, _) => Some(ResizeDirection::East),
        (_, _, true, _) => Some(ResizeDirection::North),
        (_, _, _, true) => Some(ResizeDirection::South),
        _ => None,
    }
}
