//! Retained window bounds and layer ordering.

use super::{Context, Id};
use crate::{Rect, Vec2};

pub(crate) struct WindowState {
    pub root: bool,
    pub rect: Rect,
    pub displayed_rect: Rect,
    pub min_size: Vec2,
    pub last_frame: u64,
    /// Stays above windows that are not `on_top`, however they were raised.
    pub on_top: bool,
}

impl Context {
    pub(crate) fn root_state(&mut self, id: Id) {
        let rect = self.viewport();
        self.windows.insert(
            id,
            WindowState {
                root: true,
                rect,
                displayed_rect: rect,
                min_size: Vec2::ZERO,
                last_frame: self.frame,
                on_top: false,
            },
        );
        self.layers.retain(|layer| *layer != id);
        self.layers.insert(0, id);
    }

    pub(crate) fn raise_window(&mut self, id: Id) {
        if self.windows.get(&id).is_some_and(|state| state.root) {
            return;
        }
        self.layers.retain(|layer| *layer != id);
        self.layers.push(id);
        self.stack_on_top();
    }

    /// Keep `on_top` windows after all others, each group in its own order.
    fn stack_on_top(&mut self) {
        let windows = &self.windows;
        let top = |id: &Id| windows.get(id).is_some_and(|window| window.on_top);
        self.layers.sort_by_key(|id| top(id));
    }

    pub(crate) fn set_window_on_top(&mut self, id: Id, on_top: bool) {
        if let Some(state) = self
            .windows
            .get_mut(&id)
            .filter(|state| state.on_top != on_top)
        {
            state.on_top = on_top;
            self.stack_on_top();
        }
    }

    pub(crate) fn front_window(&self) -> Option<Id> {
        self.layers.iter().rev().copied().find(|id| {
            self.windows.get(id).is_some_and(|window| {
                self.visible_windows.contains(id) || window.last_frame == self.frame
            })
        })
    }

    pub(crate) fn window_state(&mut self, id: Id, initial: Rect, min_size: Vec2) -> Rect {
        if !self.windows.contains_key(&id) {
            self.layers.push(id);
            self.stack_on_top();
        }
        let state = self.windows.entry(id).or_insert(WindowState {
            root: false,
            rect: initial,
            displayed_rect: initial,
            min_size,
            last_frame: self.frame,
            on_top: false,
        });
        state.min_size = min_size;
        let size = state.rect.size().max(min_size);
        let max_position = (self.logical_size - Vec2::new(48.0, 32.0)).max(Vec2::ZERO);
        let position = state.rect.min.clamp(Vec2::ZERO, max_position);
        state.rect = Rect::from_min_size(position, size);
        state.last_frame = self.frame;
        state.rect
    }

    pub(super) fn top_window(&self, pointer: Vec2) -> Option<Id> {
        if let Some(popup) = &self.popup {
            if self.viewport().contains(pointer) {
                if let Some((layer, _)) =
                    popup.extra.iter().rev().find(|(_, r)| r.contains(pointer))
                {
                    return Some(*layer);
                }
                return Some(popup.id);
            }
        }
        if let Some(modal) = self.modals.stack.last() {
            if self.viewport().contains(pointer) {
                return Some(modal.id);
            }
        }
        self.layers.iter().rev().copied().find(|id| {
            self.windows.get(id).is_some_and(|w| {
                (self.visible_windows.contains(id) || w.last_frame == self.frame)
                    && w.displayed_rect.contains(pointer)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winit::{
        dpi::PhysicalSize,
        event::{DeviceId, ElementState, MouseButton, WindowEvent},
    };
    use crate::{vec2, Window};
    use crate::{Padding, Root};

    #[test]
    fn root_tracks_viewport_and_reuses_unchanged_geometry() {
        let mut context = Context::new();
        for (size, scale) in [
            (PhysicalSize::new(640, 480), 1.0),
            (PhysicalSize::new(1600, 1200), 2.0),
        ] {
            context.set_viewport(size, scale);
            let mut clip = context.viewport();
            context.run(|context| {
                let result = Root::new().padding(Padding::all(0.0)).show(context, |ui| {
                    clip = ui.clip_rect();
                    ui.label("Root content");
                    42
                });
                assert_eq!(result, 42);
            });
            assert_eq!(clip, context.viewport());
            assert_eq!(context.windows[&Id::new("zaxis-root")].displayed_rect, clip);
            let revision = context.draw_data().revision;
            context.run(|context| {
                Root::new().padding(Padding::all(0.0)).show(context, |ui| {
                    ui.label("Root content");
                });
            });
            assert_eq!(context.draw_data().revision, revision);
        }
    }

    #[test]
    fn clicking_root_keeps_floating_windows_above_it() {
        let mut context = Context::new();
        context.set_viewport(PhysicalSize::new(640, 480), 1.0);
        let panel = Id::new("floating");
        // Build the root last to also verify that callback order cannot cover panels.
        context.run(|context| {
            Window::new("Floating").id(panel).show(context, |ui| {
                ui.button("Panel");
            });
            Root::new().show(context, |ui| {
                ui.button("Root");
            });
        });
        assert_eq!(context.top_window(vec2(100.0, 100.0)), Some(panel));
        context.on_window_event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: crate::winit::dpi::PhysicalPosition::new(600.0, 400.0),
        });
        context.on_window_event(&WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Left,
        });
        assert_eq!(context.top_window(vec2(100.0, 100.0)), Some(panel));
    }
}
