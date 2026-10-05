//! What menus tell assistive technology: the rows, and the request to open a context menu
//! on the widget it belongs to.

use super::ContextMenuItem;
use crate::{
    components::{Response, Ui},
    context::popup::Wrapped,
    AccessAction, AccessActionKind as Kind, AccessNode, Id, Vec2,
};

/// How a context menu finds the node of its target.
#[derive(Clone, Copy)]
pub(super) enum Target {
    /// By the target's id among the nodes described before the menu.
    Find,
    /// The first node of the widget the menu wraps, and whether the menu was asked for
    /// before that widget ran.
    Wrapped { node: usize, asked: bool },
    /// The wrapped widget described nothing.
    Silent,
}

fn show(request: &AccessAction) -> bool {
    matches!(request, AccessAction::ShowContextMenu)
}

/// Call before building the widget a menu wraps.
pub(super) fn before(ui: &Ui<'_>) -> Wrapped {
    ui.context.a11y_wrap(show)
}

/// Call after the wrapped widget was built.
pub(super) fn wrapped(ui: &mut Ui<'_>, before: Wrapped) -> Target {
    match ui.context.a11y_wrapped(before) {
        Some((node, requests)) => Target::Wrapped {
            node,
            asked: !requests.is_empty(),
        },
        None => Target::Silent,
    }
}

/// Let the target's node accept `ShowContextMenu`, and answer a request for it with the
/// point the menu opens at: the middle of what is visible of the target.
pub(super) fn asked(
    ui: &mut Ui<'_>,
    target: Target,
    response: Response,
    anchor: Id,
    usable: bool,
) -> Option<Vec2> {
    if !ui.context.a11y_on() || !usable || !ui.is_enabled() {
        return None;
    }
    let (node, asked) = match target {
        Target::Silent => return None,
        Target::Wrapped { node, asked } => (ui.context.a11y_node_mut(node), asked),
        Target::Find => {
            let requests = ui.context.a11y_requests(show);
            let asked = requests.iter().any(|(node, _)| *node == response.id);
            (ui.context.a11y_node_of(response.id), asked)
        }
    };
    node?.action(Kind::ShowContextMenu);
    if !asked {
        return None;
    }
    // Where the target was drawn, after layout, scrolling and transforms.
    let shown = ui
        .context
        .interaction
        .previous_hits
        .iter()
        .find(|hit| hit.id == anchor);
    let visible = shown
        .map(|hit| hit.rect.intersect(hit.clip))
        .filter(|rect| !rect.is_empty())
        .unwrap_or(response.rect);
    Some(visible.center())
}

/// One selectable row of a menu.
pub(super) fn row(node: &mut AccessNode, id: Id, item: &ContextMenuItem) {
    if item.left_text.is_empty() {
        node.label(item.text.as_str());
    } else {
        node.label(format!("{} {}", item.left_text, item.text));
    }
    if !item.right_text.is_empty() {
        node.keyboard_shortcut(item.right_text.as_str());
    }
    node.disabled(!item.enabled).clicks(id);
    if item.checked {
        node.toggled(true);
    }
    if item.submenu {
        node.has_popup()
            .expanded(false)
            .action(Kind::Expand)
            .action(Kind::Collapse);
    }
}
