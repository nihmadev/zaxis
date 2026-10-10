//! What widgets share about actions: `ui.actions()`, the shortcut and description a node
//! tells assistive technology, and the tooltip and event of a button bound to an action.

use super::{Response, Tooltip, Ui};
use crate::{
    context::{actions::ActionView, ActionSource, ActionsHandle},
    AccessNode, Id,
};

impl Ui<'_> {
    /// The actions of this context: set enabled and checked state for the pass, take the
    /// events menus, buttons and keys raised. See [`ActionsHandle`].
    pub fn actions(&mut self) -> ActionsHandle<'_> {
        self.context.actions()
    }
}

/// The shortcut and description of an action on its node.
pub(crate) fn describe(node: &mut AccessNode, view: &ActionView) {
    if !view.shortcut.is_empty() {
        node.keyboard_shortcut(view.shortcut.as_str());
    }
    if !view.description.is_empty() {
        node.description(view.description.as_str());
    }
}

/// After a button bound to an action was laid out: a click raises the action and the
/// tooltip names the shortcut.
pub(crate) fn finish_button(
    ui: &mut Ui<'_>,
    action: Id,
    view: &ActionView,
    response: Response,
    enabled: bool,
) {
    if enabled && response.clicked() {
        ui.context.fire_action(action, ActionSource::Button);
    }
    let mut text = view.title.clone();
    if !view.shortcut.is_empty() {
        text.push_str(&format!(" ({})", view.shortcut));
    }
    if !view.description.is_empty() {
        text.push('\n');
        text.push_str(&view.description);
    }
    Tooltip::new(text).show(ui, response);
}
