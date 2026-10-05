//! Keyboard dispatch. A key goes to the first handler that consumes it, in this order:
//! a drag in progress, the open popup (Escape and Tab dismiss it), the top modal
//! (Escape), menu navigation, middle-button autoscroll (Escape), the focused split
//! boundary, tree, selectable text, drag value, text field or slider, and finally focus
//! traversal, the modal default action and activation of the focused widget. Each handler
//! returns whether it consumed the key; a consumed key still updates `InputState`'s key
//! sets the way its handler decides.

use super::{Context, EventResponse, HitAction, Id};
use winit::{event::ElementState, keyboard::KeyCode};

impl Context {
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
        let pressed = state == ElementState::Pressed;
        if pressed {
            self.interaction.focus_visible = true;
        }
        if self.drag_key(code, state, repeat) || self.popup_key(code, pressed) {
            return true;
        }
        if self.popups.current.is_none() && self.modal_escape(code, state, repeat) {
            return true;
        }
        if self.menu_key(code, state) {
            return true;
        }
        if code == KeyCode::Escape && pressed && self.stop_auto_scroll() {
            return true;
        }
        if self.split_key(code, pressed) || self.tree_key(code, state, repeat) {
            return true;
        }
        if pressed && !repeat && self.selection_key(code) {
            self.input.keys_down.insert(code);
            return true;
        }
        let focused = self
            .interaction
            .focused
            .and_then(|id| Some((id, self.interaction.action_of(id)?)));
        if let Some((id, action)) = focused {
            let consumed = match action {
                HitAction::DragValue => self.drag_value_key(id, code, pressed),
                HitAction::TextEdit => self.text_key(id, code, pressed),
                HitAction::Slider => self.slider_key(id, code, pressed),
                _ => false,
            };
            if consumed {
                return true;
            }
        }
        self.navigation_key(code, pressed, repeat, focused.map(|(_, action)| action))
    }

    /// Escape and Tab dismiss the open popup when pressed. Escape is consumed; Tab goes
    /// on to move focus.
    fn popup_key(&mut self, code: KeyCode, pressed: bool) -> bool {
        if self.popups.current.is_none() || !matches!(code, KeyCode::Escape | KeyCode::Tab) {
            return false;
        }
        if pressed {
            self.dismiss_popup(true);
        }
        code == KeyCode::Escape
    }

    /// Keys no control took: Tab traversal, Enter as the modal default action, and Enter
    /// or Space held on a clickable widget, which clicks it on release.
    fn navigation_key(
        &mut self,
        code: KeyCode,
        pressed: bool,
        repeat: bool,
        focused: Option<HitAction>,
    ) -> bool {
        self.input.record_key(code, pressed);
        if !pressed {
            return self.release_activation(code);
        }
        if code == KeyCode::Tab && !repeat {
            return self.traverse_focus();
        }
        let activates = focused.is_some_and(|action| {
            action == HitAction::Activate
                || action == HitAction::Link
                || matches!(action, HitAction::Interact(sense) if sense.click())
        });
        if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter)
            && !repeat
            && !activates
            && self.modal_default_action()
        {
            return true;
        }
        if matches!(code, KeyCode::Enter | KeyCode::Space) && !repeat && activates {
            if let Some(id) = self.interaction.focused {
                self.interaction.keyboard_active = Some((id, code));
                return true;
            }
        }
        false
    }

    /// Move focus to the next (Shift: previous) focusable region in published order,
    /// within the top modal, skipping tree row actions and regions clipped away. Returns
    /// whether there was one.
    fn traverse_focus(&mut self) -> bool {
        let trees = &self.trees;
        let mut stops: Vec<Id> = self
            .interaction
            .previous_hits
            .iter()
            .filter(|h| {
                h.action.focusable()
                    && !trees.is_row_action(h.id)
                    && self.modal_tab_scope(h)
                    && !h.rect.intersect(h.clip).is_empty()
            })
            .map(|h| h.id)
            .collect();
        // Fragments of one wrapped link are one focus stop.
        stops.dedup();
        if stops.is_empty() {
            return false;
        }
        let current = self.interaction.focused.and_then(|id| {
            let owner = self.trees.tab_owner(id);
            stops.iter().position(|stop| *stop == owner)
        });
        let next = if self.input.modifiers.shift_key() {
            current.map_or(stops.len() - 1, |i| (i + stops.len() - 1) % stops.len())
        } else {
            current.map_or(0, |i| (i + 1) % stops.len())
        };
        self.set_focus(Some(stops[next]));
        self.interaction.keyboard_active = None;
        true
    }

    /// Releasing the key that holds a widget down clicks it, if it still has focus.
    fn release_activation(&mut self, code: KeyCode) -> bool {
        let Some((id, held)) = self.interaction.keyboard_active else {
            return false;
        };
        if held != code {
            return false;
        }
        self.interaction.keyboard_active = None;
        if self.interaction.focused == Some(id) {
            self.interaction.clicked.insert(id);
        }
        true
    }
}
