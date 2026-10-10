//! Bridge from input events, native or not, to logical UI input.

use super::{
    platform_input::{ImeEvent, InputEvent, KeyInput, WheelDelta},
    Context, EventResponse,
};
use crate::{files::PickedFile, Vec2};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
};

/// A wheel notch scrolls three lines of text at a line height of 1.5 font sizes.
const LINE_STEP: f32 = 4.5;

impl Context {
    /// Handle a winit event immediately against retained hit regions. This preserves
    /// press/release pairs even when several events arrive before the next redraw. The
    /// event is converted with [`InputEvent::from_window_event`] and handled by
    /// [`on_input`](Self::on_input); events the context does not use are ignored.
    pub fn on_window_event(&mut self, event: &WindowEvent) -> EventResponse {
        match InputEvent::from_window_event(event) {
            Some(event) => self.on_input(event),
            None => EventResponse::default(),
        }
    }

    /// Handle one input event immediately against retained hit regions. The only entry
    /// point for input: the winit runner and hosts without a window (an overlay in another
    /// process, an engine with its own events) take this path. `consumed` says whether
    /// something in the interface took the event, so the host can withhold it from the
    /// application underneath; `repaint` says whether another pass is needed.
    ///
    /// Pointer positions are physical pixels of the viewport set by
    /// [`set_viewport`](Self::set_viewport) or by [`InputEvent::Resized`].
    pub fn on_input(&mut self, event: InputEvent) -> EventResponse {
        let mut consumed = false;
        let repaint = match event {
            InputEvent::PointerMoved { x, y } => {
                let pointer = Vec2::new(x as f32, y as f32) / self.scale;
                consumed = self.interaction.capture.is_some()
                    || self.scrolling.auto.is_some()
                    || self.hit_test(pointer).is_some();
                let changed = self.input.pointer != Some(pointer);
                self.move_pointer(pointer);
                changed
            }
            InputEvent::PointerLeft => {
                self.input.pointer = None;
                self.drag_cursor_left();
                self.stop_auto_scroll();
                true
            }
            InputEvent::Button { button, state } => {
                if button == MouseButton::Middle {
                    consumed = self.middle_button(state);
                } else if state == ElementState::Pressed && self.stop_auto_scroll() {
                    consumed = true;
                } else if button == MouseButton::Left {
                    consumed = self.primary_button(state);
                } else if button == MouseButton::Right {
                    consumed = self.secondary_button(state);
                }
                consumed |= self.popups.is_active() || self.modal_active();
                true
            }
            InputEvent::Wheel(delta) => {
                self.stop_auto_scroll();
                let delta = match delta {
                    WheelDelta::Lines(lines) => lines * self.style.font_size * LINE_STEP,
                    WheelDelta::Pixels(pixels) => pixels / self.scale,
                };
                if !delta.is_finite() {
                    self.report(super::DiagnosticKind::InvalidValue, None, None, || {
                        "wheel input must be finite; event ignored".into()
                    });
                    return EventResponse::default();
                }
                let delta = if self.input.modifiers.shift_key() && delta.x == 0.0 {
                    Vec2::new(delta.y, 0.0)
                } else {
                    delta
                };
                self.input.scroll_delta += delta;
                self.scrolling.wheel = true;
                consumed = self.scroll_wheel(-delta);
                self.scrolling.wheel = false;
                true
            }
            InputEvent::Key(KeyInput {
                physical,
                logical,
                state,
                repeat,
                text,
            }) => {
                let mut claimed = false;
                if let Some(code) = navigation_code(physical, &logical) {
                    let layout = crate::actions::latin_letter(&logical).unwrap_or(code);
                    self.keys.set_raw(physical, &logical, &text);
                    consumed = self.on_key_layout(code, layout, state, repeat).consumed;
                    claimed = self.keys.finish_key();
                }
                if state == ElementState::Pressed {
                    if let Some(text) = &text {
                        // A key a widget claimed is a command for it, not text.
                        if !self.text_fields.composing
                            && !claimed
                            && !self.input.modifiers.control_key()
                            && !self.input.modifiers.super_key()
                        {
                            consumed |= self.on_text_event(text).consumed;
                        }
                    }
                }
                true
            }
            InputEvent::Modifiers(modifiers) => {
                self.input.modifiers = modifiers;
                true
            }
            InputEvent::Ime(ImeEvent::Commit(text)) => {
                self.text_fields.composing = false;
                consumed = self.on_committed_text(&text, true).consumed;
                true
            }
            InputEvent::Ime(ImeEvent::Preedit(text, cursor)) => {
                consumed = self.ime_preedit(&text, cursor);
                true
            }
            InputEvent::Ime(ImeEvent::Disabled) => {
                self.ime_disabled();
                true
            }
            InputEvent::Focus(focused) => {
                self.input.focused = focused;
                if !focused {
                    self.focus_lost();
                }
                true
            }
            InputEvent::Resized { width, height } => {
                self.cancel_camera_input();
                self.drag_cancel(crate::components::drag_drop::DragReason::Cancelled);
                self.set_viewport(PhysicalSize::new(width, height), f64::from(self.scale));
                true
            }
            InputEvent::ScaleFactor(scale_factor) => {
                self.cancel_camera_input();
                self.drag_cancel(crate::components::drag_drop::DragReason::Cancelled);
                let physical = PhysicalSize::new(
                    (self.logical_size.x * self.scale).round() as u32,
                    (self.logical_size.y * self.scale).round() as u32,
                );
                self.set_viewport(physical, scale_factor);
                true
            }
            InputEvent::FileHovered(path) => self.file_hovered(PickedFile::from_path(path)),
            InputEvent::FileHoverCancelled => self.file_hover_cancelled(),
            InputEvent::FileDropped(path) => self.file_dropped(PickedFile::from_path(path)),
        };
        if repaint {
            self.request_repaint();
        }
        EventResponse { consumed, repaint }
    }
}

impl Context {
    /// The window lost focus: popups close without restoring focus, drags and gestures
    /// end, buttons and keys count as released, and nothing keeps focus or composition.
    fn focus_lost(&mut self) {
        self.dismiss_popup(false);
        self.drag_cancel(crate::components::drag_drop::DragReason::FocusLost);
        self.input.primary_down = false;
        self.input.middle_down = false;
        self.input.secondary_down = false;
        self.gesture_cancel();
        self.camera_routing.cancel();
        self.stop_auto_scroll();
        self.input.keys_down.clear();
        self.keys.reset();
        self.focus_groups.finish_frame();
        self.actions.focus_lost();
        self.interaction.capture = None;
        self.text_fields.clicks.reset();
        self.interaction.keyboard_active = None;
        self.set_focus(None);
        self.text_fields.composing = false;
        self.input.pointer = None;
    }
}

// Navigation follows key meaning: NumPad with NumLock off has a numeric physical
// scan code but an arrow/Home/End logical key. Letter shortcuts stay physical.
pub fn navigation_code(physical: PhysicalKey, logical: &Key) -> Option<KeyCode> {
    let code = match logical {
        Key::Named(NamedKey::ArrowUp) => KeyCode::ArrowUp,
        Key::Named(NamedKey::ArrowDown) => KeyCode::ArrowDown,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::ArrowLeft,
        Key::Named(NamedKey::ArrowRight) => KeyCode::ArrowRight,
        Key::Named(NamedKey::Home) => KeyCode::Home,
        Key::Named(NamedKey::End) => KeyCode::End,
        Key::Named(NamedKey::PageUp) => KeyCode::PageUp,
        Key::Named(NamedKey::PageDown) => KeyCode::PageDown,
        _ => {
            return match physical {
                PhysicalKey::Code(code) => Some(code),
                _ => None,
            }
        }
    };
    Some(code)
}
