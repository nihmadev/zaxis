//! A button bound to a registered action.

use super::Button;
use crate::{context::actions::ActionView, Id, Ui};
use std::hash::Hash;

impl<F: Fn(&mut crate::Painter<'_>, crate::ControlPaint)> Button<F> {
    /// Run an action when clicked while keeping this button's own caption. The action's
    /// enabled state, checked state, shortcut tooltip and event apply as in
    /// [`Button::action`].
    pub fn triggers(mut self, id: impl Hash) -> Self {
        self.action = Some(crate::actions::action_id(id));
        self
    }

    /// Take caption, icon, enabled and checked state from the action, if the button has one.
    pub(super) fn apply_action(&mut self, ui: &mut Ui<'_>) -> Option<(Id, ActionView)> {
        let id = self.action?;
        let view = ui.context.action_view(id)?;
        self.enabled &= view.enabled;
        if self.text.is_empty() {
            // The caption may change with the language; the identity must not.
            self.text.clone_from(&view.title);
            self.id.get_or_insert(id);
        }
        if let Some(checked) = view.checked {
            self.selected = checked;
            self.toggle = true;
        }
        if self.icon.is_none() {
            self.icon.clone_from(&view.icon);
        }
        Some((id, view))
    }
}
