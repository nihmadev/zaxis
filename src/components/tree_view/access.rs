//! The tree for assistive technology: one `Tree` node that scrolls and holds focus, its
//! built rows as a flat run of `TreeItem`s whose depth is their level.
use super::{state::Row, TreeInput, TreeState};
use crate::{
    accessibility::Scope, AccessAction, AccessActionKind as Kind, AccessRole, Id, Rect, Ui, Vec2,
};

/// The node (and the hit region) of the row that shows `node`.
pub(super) fn row_id(tree: Id, node: Id) -> Id {
    tree.with(("node", node))
}

/// Requests for rows, as the input the pointer and the keys produce: they run through
/// `TreeState::input` with everything else that arrived for this pass.
pub(super) fn requests(ui: &mut Ui<'_>, tree: Id, state: &TreeState, inputs: &mut Vec<TreeInput>) {
    if ui.context.a11y.pending.is_empty() {
        return;
    }
    for row in &state.rows {
        let node = row.id;
        for request in ui.context.take_access_actions(row_id(tree, node)) {
            inputs.push(match request {
                AccessAction::Click => TreeInput::Click {
                    node,
                    chevron: false,
                    double: false,
                },
                AccessAction::Expand => TreeInput::SetOpen { node, open: true },
                AccessAction::Collapse => TreeInput::SetOpen { node, open: false },
                _ => continue,
            });
        }
    }
}

/// Open the tree's node. Focus stays on it; the row the keys act on is its active descendant.
pub(super) fn begin(
    ui: &mut Ui<'_>,
    id: Id,
    label: &str,
    enabled: bool,
    state: &TreeState,
) -> Scope {
    ui.a11y_begin(id, AccessRole::Tree, |node| {
        node.label(label).disabled(!enabled);
        if let Some(cursor) = state.focused {
            node.active_descendant(row_id(id, cursor));
        }
    })
}

/// Open the node of one built row; its action widgets become its children.
pub(super) fn row(
    ui: &mut Ui<'_>,
    id: Id,
    row: &Row,
    label: &str,
    open: bool,
    selected: bool,
    enabled: bool,
) -> Scope {
    ui.a11y_begin(id, AccessRole::TreeItem, |node| {
        node.label(label)
            .level(row.depth as u32 + 1)
            .position_in_set(row.set.0 as usize, row.set.1 as usize)
            .selected(selected)
            .disabled(!(row.enabled && enabled))
            .action(Kind::Click);
        if row.children.expandable() {
            node.expanded(open)
                .action(Kind::Expand)
                .action(Kind::Collapse);
        }
    })
}

/// Close a row's node over the whole row: `activation` ends where the row's actions begin.
pub(super) fn end_row(ui: &mut Ui<'_>, scope: Scope, activation: Rect) {
    let right = ui.layout.bounds.max.x.max(activation.max.x);
    let rect = Rect::from_min_max(activation.min, Vec2::new(right, activation.max.y));
    ui.a11y_end(scope, Some(rect));
}
