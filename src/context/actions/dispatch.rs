//! Keys to actions. Runs inside `Context::key`, after everything that owns a key outright
//! (drag, popups, modal Escape, menu navigation, split and tree handles, selection copy)
//! and before the focused control and focus traversal, so a shortcut works while a button
//! or a list has focus yet never takes a key a text field edits with.

use super::{ActionSource, Pending};
use crate::{
    actions::{is_modifier, Lookup, Mods, Stroke},
    context::{Context, HitAction, Id},
    KeyBinding,
};
use winit::keyboard::KeyCode;

impl Context {
    /// Escape cancels a chord that waits for its next stroke, before any other handler.
    pub(in crate::context) fn chord_escape(&mut self, code: KeyCode, pressed: bool) -> bool {
        if code != KeyCode::Escape || self.actions.chord.is_none() {
            return false;
        }
        if pressed {
            self.actions.chord = None;
            self.actions.swallowed.insert(code);
            self.request_repaint();
            return true;
        }
        self.actions.swallowed.remove(&code)
    }

    /// The release or repeat of a key an action took.
    pub(in crate::context) fn action_key_tail(
        &mut self,
        code: KeyCode,
        pressed: bool,
        repeat: bool,
    ) -> bool {
        if pressed {
            repeat && self.actions.swallowed.contains(&code)
        } else {
            self.actions.release_swallowed(code)
        }
    }

    /// A key press that may run an action or continue a chord. `layout` is the key it counts
    /// as for shortcuts. Returns whether the key was taken.
    pub(in crate::context) fn action_key(&mut self, code: KeyCode, layout: KeyCode) -> bool {
        let mods = Mods::from_state(self.input.modifiers);
        self.actions.pressed.insert(code, (layout, mods));
        if self.actions.registry.is_empty()
            || is_modifier(code)
            || self.key_capture_active()
            || self.popups.is_active()
        {
            return false;
        }
        let pending = self.actions.chord.take();
        if pending.is_none() && !self.text_focus_lets_through(mods, layout) {
            return false;
        }
        let mut strokes: Vec<Stroke> = pending
            .as_ref()
            .map_or_else(Vec::new, |p| p.strokes.clone());
        strokes.push(Stroke::new(mods, KeyBinding::Key(layout)));
        let keymap = self.actions.registry.keymap();
        let modal = self.modal_active();
        let usable = |id: Id| {
            self.action_enabled_for_keys(id)
                && (!modal || self.actions.registry.action(id).is_some_and(|a| a.modal))
        };
        let found = keymap.lookup(&self.actions.published_context, &strokes, &usable);
        match found {
            Lookup::Full(id) => {
                self.fire_action_from_keys(id);
                self.actions.swallowed.insert(code);
                true
            }
            Lookup::Prefix => {
                let delay = self.chord_timeout();
                self.actions.chord = self
                    .frame_time
                    .checked_add(delay)
                    .map(|deadline| Pending { strokes, deadline });
                self.request_repaint_after(delay);
                self.request_repaint();
                self.actions.swallowed.insert(code);
                true
            }
            Lookup::None if pending.is_some() => {
                // A key that does not continue the chord ends it and is not used again.
                self.request_repaint();
                self.actions.swallowed.insert(code);
                true
            }
            Lookup::None => false,
        }
    }

    /// Keys between passes read what the last pass showed.
    fn action_enabled_for_keys(&self, id: Id) -> bool {
        self.actions
            .published
            .get(&id)
            .and_then(|flags| flags.enabled)
            .unwrap_or(true)
    }

    fn fire_action_from_keys(&mut self, id: Id) {
        let queued = self.frame;
        self.actions.events.push(super::Event {
            id,
            source: ActionSource::Keyboard,
            queued,
        });
        self.request_repaint();
    }

    /// While a text field has focus it keeps printable keys and its editing shortcuts;
    /// Ctrl+S, function keys and the like still reach the actions.
    fn text_focus_lets_through(&self, mods: Mods, code: KeyCode) -> bool {
        if self.interaction.focused_as(HitAction::TextEdit).is_none() {
            return true;
        }
        let command = mods.contains(Mods::CTRL) || mods.contains(Mods::META);
        if command {
            // Ctrl and Alt together is AltGr on Windows: it types characters.
            return !(mods.contains(Mods::ALT) && !mods.contains(Mods::META)) && !edits_text(code);
        }
        if mods.contains(Mods::ALT) {
            return !is_printable(code);
        }
        is_function_key(code)
    }
}

fn edits_text(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::KeyA
            | KeyCode::KeyC
            | KeyCode::KeyX
            | KeyCode::KeyV
            | KeyCode::KeyZ
            | KeyCode::KeyY
            | KeyCode::Insert
            | KeyCode::Delete
            | KeyCode::Backspace
            | KeyCode::ArrowLeft
            | KeyCode::ArrowRight
            | KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
    )
}

fn is_function_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::F1
            | KeyCode::F2
            | KeyCode::F3
            | KeyCode::F4
            | KeyCode::F5
            | KeyCode::F6
            | KeyCode::F7
            | KeyCode::F8
            | KeyCode::F9
            | KeyCode::F10
            | KeyCode::F11
            | KeyCode::F12
            | KeyCode::F13
            | KeyCode::F14
            | KeyCode::F15
            | KeyCode::F16
            | KeyCode::F17
            | KeyCode::F18
            | KeyCode::F19
            | KeyCode::F20
            | KeyCode::F21
            | KeyCode::F22
            | KeyCode::F23
            | KeyCode::F24
    )
}

fn is_printable(code: KeyCode) -> bool {
    crate::actions::key_name(code).is_some_and(|name| {
        name.len() == 1
            || matches!(
                name,
                "space"
                    | "comma"
                    | "period"
                    | "slash"
                    | "backslash"
                    | "semicolon"
                    | "quote"
                    | "bracketleft"
                    | "bracketright"
                    | "minus"
                    | "equal"
                    | "backquote"
            )
    })
}
