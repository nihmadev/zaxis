//! Menu items bound to actions: caption, shortcut, enabled and checked state come from the
//! registry on every pass, and choosing one raises the action's event.

use super::{Kind, MenuItem};
use crate::{components::Ui, context::ActionSource, Id};

pub(super) fn any(items: &[MenuItem]) -> bool {
    items
        .iter()
        .any(|item| item.action || item.children().is_some_and(any))
}

/// The items with every action item replaced by what the registry says now.
pub(super) fn resolve(ui: &mut Ui<'_>, items: &[MenuItem]) -> Vec<MenuItem> {
    items
        .iter()
        .map(|item| {
            let mut item = item.clone();
            if let Kind::Submenu(children) = &item.kind {
                item.kind = Kind::Submenu(resolve(ui, children));
            }
            let Kind::Action(id) = item.kind.clone() else {
                return item;
            };
            if !item.action {
                return item;
            }
            match ui.context.action_view(id) {
                Some(view) => {
                    item.text = view.title;
                    if item.shortcut.is_empty() {
                        item.shortcut = view.shortcut;
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

fn is_action(items: &[MenuItem], id: Id) -> bool {
    items.iter().any(|item| {
        item.action && matches!(item.kind, Kind::Action(own) if own == id)
            || item.children().is_some_and(|c| is_action(c, id))
    })
}

/// A chosen action item raises its event instead of being reported as `selected`, so one
/// choice has one effect however many places handle the action.
pub(super) fn raise(ui: &mut Ui<'_>, items: &[MenuItem], selected: &mut Option<Id>) {
    if let Some(id) = selected.filter(|id| is_action(items, *id)) {
        ui.context.fire_action(id, ActionSource::Menu);
        *selected = None;
    }
}
