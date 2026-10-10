//! Group navigation on the keyboard path and publishing at the end of a pass.

use super::nav::Nav;
use crate::context::{key_routing::Owner, Context, DiagnosticKind};
use winit::keyboard::KeyCode;

impl Context {
    /// An arrow key, Home or End on a focused member of a group: focus moves to the
    /// neighbour the group names. Runs after everything that owns the key (a control that
    /// uses it, Actions, claims), just before Tab traversal. Without modifiers, in a layer
    /// that may take keys, and not while a `KeyBox` captures. The key is owned until
    /// release, and its autorepeat moves focus on, whatever has focus after the first
    /// step. The press is not left in `InputState::keys_pressed`: a control that reads
    /// keys there (as controls written before groups did) would otherwise act on a key
    /// that moved focus onto it. Returns whether a group took the key.
    pub(in crate::context) fn group_key(&mut self, code: KeyCode) -> bool {
        let mods = self.input.modifiers;
        if mods.control_key()
            || mods.alt_key()
            || mods.super_key()
            || mods.shift_key()
            || self.key_capture_active()
        {
            return false;
        }
        let Some(focused) = self.interaction.focused else {
            return false;
        };
        let nav = self.focus_groups.navigate(focused, code);
        if nav == Nav::Unhandled {
            return false;
        }
        let in_open_layer = self
            .interaction
            .previous_hits
            .iter()
            .find(|hit| hit.id == focused && hit.action.focusable())
            .is_some_and(|hit| self.key_layer_open(hit.window));
        if !in_open_layer {
            return false;
        }
        match nav {
            Nav::Unhandled | Nav::Stay => {}
            Nav::Move { target, group } => {
                self.set_focus(Some(target));
                self.interaction.keyboard_active = None;
                self.focus_groups.moved.insert(group, target);
            }
        }
        self.input.unpress(code);
        self.keys.owners.insert(code, Owner::Navigation);
        true
    }

    /// End of pass: the groups built in it route the next input.
    pub(crate) fn publish_focus_groups(&mut self) {
        if self.focus_groups.is_open() {
            self.report(DiagnosticKind::UnbalancedScope, None, None, || {
                "focus group left open at the end of the pass; its closure did not return".into()
            });
        }
        self.focus_groups
            .publish(&self.interaction.previous_hits, self.interaction.focused);
    }
}
