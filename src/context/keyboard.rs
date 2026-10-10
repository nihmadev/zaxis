//! Keyboard dispatch. A key goes to the first handler that consumes it, in this order:
//! the release or repeat of a key a widget or a group took (it stays with its owner), a
//! drag in progress, Escape on a pending chord, the release or repeat of a key an action
//! took, the leaf popup (Escape and Tab close it), the top modal (Escape), menu
//! navigation, middle-button autoscroll (Escape), Dock navigation (an application action
//! claiming its chord has priority), the focused split boundary, tree, selectable text,
//! the keys the focused widget declared with a `KeyInterest` (explicit ownership: they beat
//! the keymap), the keymap (actions and chords; a focused text field keeps printable keys
//! and its editing shortcuts), then drag value, text field or slider, then the arrow keys,
//! Home and End of a focus group, and finally focus traversal, the modal default action and
//! activation of the focused widget. Each handler returns whether it consumed the key; a
//! consumed key still updates `InputState`'s key sets the way its handler decides.
//!
//! The keys a widget or group owns are decided here, in `claimed_key` and `group_key`,
//! from the declarations of the last finished pass; nothing waits for the next pass.

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
        self.on_key_layout(code, code, state, repeat)
    }

    /// Like [`on_key_event`](Self::on_key_event) for a key that counts as another one for
    /// shortcuts: `layout` is the letter key its Latin letter names, so a shortcut follows
    /// the printed letter. Input events pick it from the logical key.
    pub(super) fn on_key_layout(
        &mut self,
        code: KeyCode,
        layout: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> EventResponse {
        let consumed = self.key_as(code, layout, state, repeat);
        self.request_repaint();
        EventResponse {
            consumed,
            repaint: true,
        }
    }

    pub(super) fn key(&mut self, code: KeyCode, state: ElementState, repeat: bool) -> bool {
        self.key_as(code, code, state, repeat)
    }

    fn key_as(
        &mut self,
        code: KeyCode,
        layout: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> bool {
        let pressed = state == ElementState::Pressed;
        if pressed {
            self.interaction.focus_visible = true;
        }
        if self.owned_key_tail(code, layout, state, repeat) {
            return true;
        }
        if self.drag_key(code, state, repeat) {
            return true;
        }
        if self.chord_escape(code, pressed) || self.action_key_tail(code, pressed, repeat) {
            return true;
        }
        if self.popup_key(code, pressed, repeat) {
            return true;
        }
        if !self.popups.is_active() && self.modal_escape(code, state, repeat) {
            return true;
        }
        if self.menu_key(code, state) {
            return true;
        }
        if code == KeyCode::Escape && pressed && self.stop_auto_scroll() {
            return true;
        }
        if self.dock_navigation_key(code) {
            if pressed && !repeat && self.action_key(code, layout) {
                return true;
            }
            self.input.record_key(code, pressed);
            return true;
        }
        if self.split_key(code, pressed) || self.tree_key(code, state, repeat) {
            return true;
        }
        if pressed && !repeat && self.selection_key(code) {
            self.input.keys_down.insert(code);
            return true;
        }
        if self.claimed_key(code, layout, state, repeat) {
            return true;
        }
        if pressed && !repeat && self.action_key(code, layout) {
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

    /// Escape closes the leaf popup and Tab closes it too, going on to move focus in what
    /// remains. One Escape press closes one level: its autorepeat and release stay with it
    /// and never reach the level below, or a modal.
    fn popup_key(&mut self, code: KeyCode, pressed: bool, repeat: bool) -> bool {
        match code {
            KeyCode::Escape if pressed && !repeat => {
                self.popups.escape_held = self.popups.is_active();
                if self.popups.escape_held {
                    self.close_popup();
                }
                self.popups.escape_held
            }
            KeyCode::Escape if pressed => self.popups.escape_held,
            KeyCode::Escape => std::mem::take(&mut self.popups.escape_held),
            KeyCode::Tab if pressed && self.popups.is_active() => {
                self.close_popup();
                false
            }
            _ => false,
        }
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
        if code != KeyCode::Tab && self.group_key(code) {
            return true;
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
    /// within the top modal, skipping tree row actions and regions clipped away. A focus
    /// group counts as one region, its Tab stop. Returns whether there was one.
    fn traverse_focus(&mut self) -> bool {
        let trees = &self.trees;
        let (groups, focused) = (&self.focus_groups, self.interaction.focused);
        let mut group_stops = std::collections::HashMap::new();
        let mut stops: Vec<Id> = self
            .interaction
            .previous_hits
            .iter()
            .filter(|h| {
                h.action.focusable()
                    && !trees.is_row_action(h.id)
                    && self.modal_tab_scope(h)
                    && !h.rect.intersect(h.clip).is_empty()
                    && groups.is_tab_stop(h.id, focused, &mut group_stops)
            })
            .map(|h| h.id)
            .collect();
        // Fragments of one wrapped link are one focus stop.
        stops.dedup();
        if stops.is_empty() {
            return false;
        }
        let current = self.interaction.focused.and_then(|id| {
            let owner = groups.tab_representative(self.trees.tab_owner(id), focused);
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
