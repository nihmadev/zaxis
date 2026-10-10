//! Context menu rows bound to actions; see [`super::ContextMenuItem::action`].

use super::ContextMenuItem;
use crate::{components::Ui, context::ActionSource, Id};

pub(super) fn any(items: &[ContextMenuItem]) -> bool {
    items.iter().any(|item| item.action)
}

/// The rows with every action row replaced by what the registry says now.
pub(super) fn resolve(ui: &mut Ui<'_>, items: &[ContextMenuItem]) -> Vec<ContextMenuItem> {
    items
        .iter()
        .map(|item| {
            let mut item = item.clone();
            if !item.action {
                return item;
            }
            match item.id.and_then(|id| ui.context.action_view(id)) {
                Some(view) => {
                    item.text = view.title;
                    if item.right_text.is_empty() {
                        item.right_text = view.shortcut;
                    }
                    item.enabled &= view.enabled;
                    item.checked = view.checked.unwrap_or(item.checked);
                }
                None => item.enabled = false,
            }
            item
        })
        .collect()
}

/// A chosen action row raises its event instead of being reported as `selected`.
pub(super) fn raise(ui: &mut Ui<'_>, items: &[ContextMenuItem], selected: &mut Option<Id>) {
    let chosen = selected.filter(|id| items.iter().any(|i| i.action && i.id == Some(*id)));
    if let Some(id) = chosen {
        ui.context.fire_action(id, ActionSource::ContextMenu);
        *selected = None;
    }
}
