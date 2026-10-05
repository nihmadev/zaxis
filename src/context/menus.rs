//! Menus anchored to a popup: combo boxes, context menus and menu bars. Navigation keys go
//! to the open popup's key target, or to a focused combo box that is still closed, and
//! wait there for its next pass. Their retained state lives here as well.

use super::{Context, HitAction, Id};
use crate::components::{
    combo_box::ComboBoxState, context_menu::MenuState, menu_bar::MenuBarState,
};
use std::collections::HashMap;
use winit::{event::ElementState, keyboard::KeyCode};

#[derive(Default)]
pub(crate) struct Menus {
    pub(crate) combo_boxes: HashMap<Id, ComboBoxState>,
    pub(crate) context_menus: HashMap<Id, MenuState>,
    pub(crate) menu_bars: HashMap<Id, MenuBarState>,
    /// Navigation keys waiting for each menu's next pass.
    keys: HashMap<Id, Vec<KeyCode>>,
}

impl Menus {
    /// Menus not built in pass `frame` lose their state.
    pub(super) fn retire(&mut self, frame: u64) {
        self.combo_boxes
            .retain(|_, state| state.last_frame == frame);
        self.context_menus
            .retain(|_, state| state.last_frame == frame);
        self.menu_bars.retain(|_, state| state.last_frame == frame);
    }

    /// Keys nobody took this pass are dropped.
    pub(super) fn clear_keys(&mut self) {
        self.keys.clear();
    }
}

impl Context {
    /// A navigation key for the open popup's key target or a focused combo box. Space,
    /// Home and End stay text editing keys while a filter field has focus. Returns whether
    /// the key was consumed.
    pub(super) fn menu_key(&mut self, code: KeyCode, state: ElementState) -> bool {
        let popup_target = self
            .popups
            .current
            .as_ref()
            .and_then(|popup| popup.key_target);
        let Some(id) = popup_target.or_else(|| self.interaction.focused_as(HitAction::ComboBox))
        else {
            return false;
        };
        let navigation = matches!(
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
            && popup_target.is_some());
        if !navigation {
            return false;
        }
        let editing = self.interaction.focused_as(HitAction::TextEdit).is_some();
        if editing
            && !matches!(
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
            return false;
        }
        let pressed = state == ElementState::Pressed;
        self.input.record_key(code, pressed);
        if pressed {
            self.menus.keys.entry(id).or_default().push(code);
        }
        true
    }

    /// Navigation keys of menu `id` for this pass.
    pub(crate) fn take_menu_keys(&mut self, id: Id) -> Vec<KeyCode> {
        self.menus.keys.remove(&id).unwrap_or_default()
    }
}
