//! Bridge from native winit events to logical UI input.

use super::{Context, EventResponse};
use crate::Vec2;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::PhysicalKey,
};

impl Context {
    /// Handle an event immediately against retained hit regions. This preserves
    /// press/release pairs even when several events arrive before the next redraw.
    pub fn on_window_event(&mut self, event: &WindowEvent) -> EventResponse {
        let mut consumed = false;
        let repaint = match event {
            WindowEvent::CursorMoved { position, .. } => {
                let pointer = Vec2::new(position.x as f32, position.y as f32) / self.scale;
                consumed = self.capture.is_some()
                    || self.scrolling.auto.is_some()
                    || self.hit_test(pointer).is_some();
                let changed = self.input.pointer != Some(pointer);
                self.move_pointer(pointer);
                changed
            }
            WindowEvent::CursorLeft { .. } => {
                self.input.pointer = None;
                self.stop_auto_scroll();
                true
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Middle {
                    consumed = self.middle_button(*state);
                } else if *state == ElementState::Pressed && self.stop_auto_scroll() {
                    consumed = true;
                } else if *button == MouseButton::Left {
                    consumed = self.primary_button(*state);
                }
                consumed |= self.popup.is_some();
                true
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.stop_auto_scroll();
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(*x, *y) * self.style.font_size,
                    MouseScrollDelta::PixelDelta(p) => {
                        Vec2::new(p.x as f32, p.y as f32) / self.scale
                    }
                };
                let delta = if self.input.modifiers.shift_key() && delta.x == 0.0 {
                    Vec2::new(delta.y, 0.0)
                } else {
                    delta
                };
                self.input.scroll_delta += delta;
                consumed = self.scroll_wheel(-delta);
                true
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    consumed = self.on_key_event(code, event.state, event.repeat).consumed;
                }
                if event.state == ElementState::Pressed {
                    if let Some(text) = &event.text {
                        if !self.ime_composing
                            && !self.input.modifiers.control_key()
                            && !self.input.modifiers.super_key()
                        {
                            consumed |= self.on_text_event(text).consumed;
                        }
                    }
                }
                true
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.input.modifiers = modifiers.state();
                true
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                self.ime_composing = false;
                consumed = self.on_committed_text(text, true).consumed;
                true
            }
            WindowEvent::Ime(winit::event::Ime::Preedit(text, cursor)) => {
                self.ime_composing = !text.is_empty();
                if let Some(id) = self.focused_widget.filter(|id| {
                    self.previous_hits
                        .iter()
                        .any(|h| h.id == *id && h.action == super::HitAction::TextEdit)
                }) {
                    self.text_edit_input
                        .entry(id)
                        .or_default()
                        .push(super::TextEditInput::Preedit(text.clone(), *cursor));
                    consumed = true;
                }
                true
            }
            WindowEvent::Ime(winit::event::Ime::Disabled) => {
                self.ime_composing = false;
                if let Some(id) = self.focused_widget {
                    self.text_edit_input
                        .entry(id)
                        .or_default()
                        .push(super::TextEditInput::Preedit(String::new(), None));
                }
                true
            }
            WindowEvent::Focused(focused) => {
                self.input.focused = *focused;
                if !focused {
                    self.dismiss_popup(false);
                    self.input.primary_down = false;
                    self.input.middle_down = false;
                    self.stop_auto_scroll();
                    self.input.keys_down.clear();
                    self.capture = None;
                    self.text_click = None;
                    self.keyboard_active = None;
                    self.set_focus(None);
                    self.input.pointer = None;
                }
                true
            }
            WindowEvent::Resized(size) => {
                self.set_viewport(*size, f64::from(self.scale));
                true
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let physical = PhysicalSize::new(
                    (self.logical_size.x * self.scale).round() as u32,
                    (self.logical_size.y * self.scale).round() as u32,
                );
                self.set_viewport(physical, *scale_factor);
                true
            }
            _ => false,
        };
        if repaint {
            self.request_repaint();
        }
        EventResponse { consumed, repaint }
    }
}
