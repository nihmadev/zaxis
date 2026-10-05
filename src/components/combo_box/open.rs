//! Opening and closing: what reached the trigger this pass, applied to the retained state.

use super::{trigger::Trigger, ComboBoxOption, ComboBoxState, Ui};
use crate::{AccessAction, Id};
use winit::keyboard::KeyCode;

/// Where this pass's input leaves the popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Open {
    /// Closed; a closing animation may still show it.
    Closed,
    /// Opened by this pass: the list starts over.
    Opening,
    /// Open since an earlier pass.
    Kept,
}

/// What reached the trigger since its last pass.
pub(super) struct Input {
    /// Closed from outside: an outside press, Escape, lost focus.
    dismissed: bool,
    clicked: bool,
    access: Vec<AccessAction>,
    keys: Vec<KeyCode>,
}

/// The input, applied.
pub(super) struct Requests {
    pub(super) open: Open,
    /// Keys for the open list; keys that only opened it are gone.
    pub(super) keys: Vec<KeyCode>,
    /// The option assistive technology chose by its label.
    pub(super) value: Option<usize>,
}

impl Input {
    pub(super) fn take(ui: &mut Ui<'_>, trigger: &Trigger, clicked: bool) -> Self {
        Self {
            dismissed: ui.context.take_popup_dismissal(trigger.popup),
            clicked,
            access: ui.context.take_access_actions(trigger.id),
            keys: ui.context.take_menu_keys(trigger.id),
        }
    }
}

impl ComboBoxState {
    /// The retained state of combo box `id` for this pass (a new one is open when
    /// `default_open`), and whether its popup was open.
    pub(super) fn load(ui: &mut Ui<'_>, id: Id, default_open: bool) -> (Self, bool) {
        let existing = ui.context.menus.combo_boxes.remove(&id);
        let was_open = existing.as_ref().is_some_and(|state| state.open);
        let mut state = existing.unwrap_or_else(|| Self {
            open: default_open,
            ..Default::default()
        });
        state.last_frame = ui.context.frame;
        (state, was_open)
    }

    /// Applies the input in order: a dismissal closes; a trigger that is not usable is
    /// closed; the click toggles; requests of assistive technology; then keys, of which
    /// the first opens a closed list (arrows, Enter and Space only open it).
    pub(super) fn apply<T>(
        &mut self,
        mut was_open: bool,
        input: Input,
        trigger: &Trigger,
        options: &[ComboBoxOption<T>],
    ) -> Requests {
        if input.dismissed {
            self.open = false;
            was_open = false;
        }
        if !trigger.usable {
            self.open = false;
        }
        if trigger.usable && input.clicked {
            self.open = !self.open;
        }
        // Opening and closing are the click's toggle; a value is the option of that name
        // chosen from the list.
        let mut value = None;
        for request in input.access {
            match request {
                AccessAction::Expand if trigger.usable => self.open = true,
                AccessAction::Collapse => self.open = false,
                AccessAction::SetValue(text) if trigger.enabled => {
                    value = options
                        .iter()
                        .position(|option| option.enabled && option.label == text);
                }
                _ => {}
            }
        }
        let mut keys = Vec::new();
        for key in input.keys {
            if !trigger.usable {
                break;
            }
            if !self.open {
                self.open = true;
                // Opening keys start at the selected item (or the first enabled item).
                if matches!(
                    key,
                    KeyCode::ArrowDown | KeyCode::ArrowUp | KeyCode::Enter | KeyCode::Space
                ) {
                    continue;
                }
            }
            keys.push(key);
        }
        let open = match (self.open, was_open) {
            (false, _) => Open::Closed,
            (true, false) => Open::Opening,
            (true, true) => Open::Kept,
        };
        Requests { open, keys, value }
    }

    /// A list opened by this pass starts on the selected option (or the first enabled
    /// one) with an empty query, and its filter takes focus once shown. Without a filter
    /// there is no query.
    pub(super) fn start<T: PartialEq>(
        &mut self,
        open: Open,
        options: &[ComboBoxOption<T>],
        selected: Option<&T>,
        filterable: bool,
    ) {
        if open == Open::Opening {
            self.list.focus_filter = filterable;
            self.query.clear();
            self.active = options
                .iter()
                .find(|o| o.enabled && Some(&o.value) == selected)
                .or_else(|| options.iter().find(|o| o.enabled))
                .map(|o| o.id);
        }
        if !filterable {
            self.query.clear();
        }
    }
}
