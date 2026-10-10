use super::{Context, HitAction, Id};
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
        if self.interaction.capture.is_some() {
            return false;
        }
        // Tab strips of a window without system decorations hand their empty space to the
        // window; the topmost region under the pointer decides, so popups and windows above
        // the strip keep their input.
        let strip = self
            .hit_test(pointer)
            .is_some_and(|hit| hit.action == HitAction::NativeDrag);
        let top = self.top_window(pointer);
        let direction = match &self.native_chrome {
            Some(chrome)
                if (top == Some(chrome.owner) || top.is_some_and(|id| self.is_modal_layer(id)))
                    && window.is_resizable()
                    && !window.is_maximized() =>
            {
                resize_direction(self.viewport(), pointer, chrome.resize_border)
            }
            _ => None,
        };
        let in_title = self.native_chrome.as_ref().is_some_and(|chrome| {
            (top == Some(chrome.owner) || top.is_some_and(|id| self.is_modal_layer(id)))
                && chrome.drag.contains(pointer)
        });
        // Launch on MouseInput, before press/release can be batched into a redraw.
        let result = if let Some(direction) = direction {
            window.drag_resize_window(direction)
        } else if in_title || strip {
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
