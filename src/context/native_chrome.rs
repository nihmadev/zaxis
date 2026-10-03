use super::{Context, Id};
use crate::{Rect, Vec2};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    window::{ResizeDirection, Window},
};

pub(crate) struct NativeChrome {
    pub owner: Id,
    pub drag: Rect,
    pub resize_border: f32,
}

impl Context {
    pub(crate) fn native_chrome_press(&self, event: &WindowEvent, window: &Window) -> bool {
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
        if self.top_window(pointer) != Some(chrome.owner) || self.capture.is_some() {
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
            eprintln!("Native window interaction: {error}");
        }
        true
    }
}

fn resize_direction(rect: Rect, pointer: Vec2, border: f32) -> Option<ResizeDirection> {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_os = "macos"))]
    use crate::{vec2, Padding};
    use crate::{Root, TitleBar};
    use winit::dpi::PhysicalSize;
    #[cfg(not(target_os = "macos"))]
    use winit::{dpi::PhysicalPosition, event::DeviceId};

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn full_caption_has_no_padding_gaps_and_buttons_keep_press_release_pairs() {
        for scale in [1.0, 2.0] {
            let mut context = Context::new();
            context.set_viewport(
                PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
                scale,
            );
            context.run(|context| {
                Root::new().padding(Padding::all(0.0)).show(context, |_| {});
                TitleBar::new("Test").show(context);
            });
            let chrome = context.native_chrome.as_ref().unwrap();
            for point in [vec2(10.0, 8.0), vec2(300.0, 44.0), vec2(660.0, 23.0)] {
                assert!(chrome.drag.contains(point));
            }
            assert!(!chrome.drag.contains(vec2(680.0, 23.0)));
            assert_eq!(
                resize_direction(context.viewport(), vec2(2.0, 2.0), 5.0),
                Some(ResizeDirection::NorthWest)
            );
            for (index, x) in [(0, 685.0), (1, 731.0), (2, 777.0)] {
                context.on_window_event(&WindowEvent::CursorMoved {
                    device_id: DeviceId::dummy(),
                    position: PhysicalPosition::new(x * scale, 23.0 * scale),
                });
                // No redraw between press and release, matching rapid OS event delivery.
                for state in [ElementState::Pressed, ElementState::Released] {
                    context.on_window_event(&WindowEvent::MouseInput {
                        device_id: DeviceId::dummy(),
                        state,
                        button: MouseButton::Left,
                    });
                }
                context.run(|context| {
                    Root::new().show(context, |_| {});
                    let response = TitleBar::new("Test").show(context);
                    assert_eq!(
                        [response.minimize, response.maximize, response.close],
                        [index == 0, index == 1, index == 2]
                    );
                });
            }
            context.run(|context| {
                Root::new().show(context, |_| {});
            });
            assert!(context.native_chrome.is_none());
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn system_caption_has_no_custom_controls_or_reserved_client_area() {
        let mut context = Context::new();
        context.set_viewport(PhysicalSize::new(800, 600), 2.0);
        assert_eq!(TitleBar::HEIGHT, 0.0);
        context.run(|context| {
            Root::new().show(context, |_| {});
            let hits = context.hits.len();
            let elements = context.elements.len();
            let response = TitleBar::new("Test").show(context);
            assert!(!response.close && !response.minimize && !response.maximize);
            assert!(context.native_chrome.is_none());
            assert_eq!(context.hits.len(), hits);
            assert_eq!(context.elements.len(), elements);
        });
    }
}
