//! Keyboard focus traversal, activation, and slider input.

use super::{Context, EventResponse, HitAction, SliderInput, TextEditInput};
use winit::{event::ElementState, keyboard::KeyCode};

fn slider_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ArrowLeft
            | KeyCode::ArrowRight
            | KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
    )
}

impl Context {
    /// Deliver committed text from a custom input bridge (also used by native events).
    pub fn on_text_event(&mut self, text: &str) -> EventResponse {
        self.on_committed_text(text, false)
    }

    pub(super) fn on_committed_text(&mut self, text: &str, ime: bool) -> EventResponse {
        self.input.text.push_str(text);
        let target = self.focused_widget.filter(|id| {
            self.previous_hits
                .iter()
                .any(|hit| hit.id == *id && hit.action == HitAction::TextEdit)
        });
        if let Some(id) = target {
            self.text_edit_input.entry(id).or_default().push(if ime {
                TextEditInput::Commit(text.to_owned())
            } else {
                TextEditInput::Text(text.to_owned())
            });
        }
        self.request_repaint();
        EventResponse {
            consumed: target.is_some(),
            repaint: true,
        }
    }

    /// Process a physical key without constructing a platform-owned winit `KeyEvent`.
    /// Useful for custom input bridges and deterministic interaction benchmarks.
    /// Text and modifiers are delivered separately through `on_window_event`.
    pub fn on_key_event(
        &mut self,
        code: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> EventResponse {
        let consumed = self.key(code, state, repeat);
        self.request_repaint();
        EventResponse {
            consumed,
            repaint: true,
        }
    }

    pub(super) fn key(&mut self, code: KeyCode, state: ElementState, repeat: bool) -> bool {
        if state == ElementState::Pressed {
            self.focus_visible = true;
        }
        if self.drag_key(code, state, repeat) {
            return true;
        }
        if self.popup.is_some() && matches!(code, KeyCode::Escape | KeyCode::Tab) {
            if state == ElementState::Pressed {
                self.dismiss_popup(true);
            }
            if code == KeyCode::Escape {
                return true;
            }
        }
        if self.popup.is_none() && self.modal_escape(code, state, repeat) {
            return true;
        }
        let combo = self
            .popup
            .as_ref()
            .and_then(|popup| popup.key_target)
            .or_else(|| {
                self.focused_widget.filter(|id| {
                    self.previous_hits
                        .iter()
                        .any(|hit| hit.id == *id && hit.action == HitAction::ComboBox)
                })
            });
        if let Some(id) = combo.filter(|_| {
            matches!(
                code,
                KeyCode::ArrowUp
                    | KeyCode::ArrowDown
                    | KeyCode::Enter
                    | KeyCode::Space
                    | KeyCode::Home
                    | KeyCode::End
                    | KeyCode::PageUp
                    | KeyCode::PageDown
            ) || (matches!(code, KeyCode::ArrowLeft | KeyCode::ArrowRight)
                && self.popup.as_ref().is_some_and(|p| p.key_target.is_some()))
        }) {
            // Space/Home/End remain text editing keys when a filter has focus.
            let editing = self.focused_widget.is_some_and(|focus| {
                self.previous_hits
                    .iter()
                    .any(|h| h.id == focus && h.action == HitAction::TextEdit)
            });
            if !editing
                || matches!(
                    code,
                    KeyCode::ArrowUp
                        | KeyCode::ArrowDown
                        | KeyCode::Enter
                        | KeyCode::PageUp
                        | KeyCode::PageDown
                        | KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                )
            {
                if state == ElementState::Pressed {
                    self.input.keys_down.insert(code);
                    self.input.keys_pressed.insert(code);
                    self.combo_input.entry(id).or_default().push(code);
                } else {
                    self.input.keys_down.remove(&code);
                    self.input.keys_released.insert(code);
                }
                return true;
            }
        }
        if code == KeyCode::Escape && state == ElementState::Pressed && self.stop_auto_scroll() {
            return true;
        }
        if state == ElementState::Pressed {
            self.focus_visible = true;
        }
        if self.split_key(code, state == ElementState::Pressed) {
            return true;
        }
        if self.tree_key(code, state, repeat) {
            return true;
        }
        let focused_action = self.focused_widget.and_then(|id| {
            self.previous_hits
                .iter()
                .find(|hit| hit.id == id)
                .map(|hit| hit.action)
        });
        if focused_action == Some(HitAction::DragValue)
            && (slider_key(code)
                || matches!(
                    code,
                    KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::F2 | KeyCode::Escape
                ))
        {
            if state == ElementState::Pressed {
                self.input.keys_down.insert(code);
                self.input.keys_pressed.insert(code);
                self.number_input
                    .entry(self.focused_widget.unwrap())
                    .or_default()
                    .push(super::NumberInputEvent::Key(code, self.input.modifiers));
            } else {
                self.input.keys_down.remove(&code);
                self.input.keys_released.insert(code);
            }
            return true;
        }
        let tab_input = self
            .focused_widget
            .is_some_and(|id| self.text_edit_tabs_previous.contains(&id))
            && !self.input.modifiers.shift_key();
        if focused_action == Some(HitAction::TextEdit) && (code != KeyCode::Tab || tab_input) {
            match state {
                ElementState::Pressed => {
                    self.input.keys_down.insert(code);
                    self.input.keys_pressed.insert(code);
                    self.text_edit_input
                        .entry(self.focused_widget.unwrap())
                        .or_default()
                        .push(TextEditInput::Key(code, self.input.modifiers));
                }
                ElementState::Released => {
                    self.input.keys_down.remove(&code);
                    self.input.keys_released.insert(code);
                }
            }
            return true;
        }
        if focused_action == Some(HitAction::Slider) && slider_key(code) {
            match state {
                ElementState::Pressed => {
                    self.input.keys_down.insert(code);
                    self.input.keys_pressed.insert(code);
                    self.slider_input
                        .entry(self.focused_widget.unwrap())
                        .or_default()
                        .push(SliderInput::Key(code));
                }
                ElementState::Released => {
                    self.input.keys_down.remove(&code);
                    self.input.keys_released.insert(code);
                }
            }
            return true;
        }
        match state {
            ElementState::Pressed => {
                self.input.keys_down.insert(code);
                self.input.keys_pressed.insert(code);
                if code == KeyCode::Tab && !repeat {
                    let buttons: Vec<_> = self
                        .previous_hits
                        .iter()
                        .filter(|h| {
                            h.action.focusable()
                                && !self.tree_tab_action(h.id)
                                && self.modal_tab_scope(h)
                                && !h.rect.intersect(h.clip).is_empty()
                        })
                        .map(|h| h.id)
                        .collect();
                    if !buttons.is_empty() {
                        let current = self.focused_widget.and_then(|id| {
                            buttons
                                .iter()
                                .position(|button| *button == self.tree_tab_owner(id))
                        });
                        let next = if self.input.modifiers.shift_key() {
                            current.map_or(buttons.len() - 1, |i| {
                                (i + buttons.len() - 1) % buttons.len()
                            })
                        } else {
                            current.map_or(0, |i| (i + 1) % buttons.len())
                        };
                        self.set_focus(Some(buttons[next]));
                        self.keyboard_active = None;
                    }
                    return !buttons.is_empty();
                }
                if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter)
                    && !repeat
                    && !focused_action.is_some_and(|action| {
                        action == HitAction::Activate
                            || matches!(action, HitAction::Interact(sense) if sense.click())
                    })
                    && self.modal_default_action()
                {
                    return true;
                }
                if matches!(code, KeyCode::Enter | KeyCode::Space)
                    && !repeat
                    && focused_action.is_some_and(|action| {
                        action == HitAction::Activate
                            || matches!(action, HitAction::Interact(sense) if sense.click())
                    })
                {
                    if let Some(id) = self.focused_widget {
                        self.keyboard_active = Some((id, code));
                        return true;
                    }
                }
            }
            ElementState::Released => {
                self.input.keys_down.remove(&code);
                self.input.keys_released.insert(code);
                if let Some((id, active_code)) = self.keyboard_active {
                    if active_code == code {
                        self.keyboard_active = None;
                        if self.focused_widget == Some(id) {
                            self.clicked.insert(id);
                        }
                        return true;
                    }
                }
            }
        }
        false
    }
}
