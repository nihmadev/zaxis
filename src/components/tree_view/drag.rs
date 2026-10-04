//! Node dragging. Rows become sources and targets through the shared drag
//! primitives; the tree only validates the move and reports it.
use super::{state::Row, TreeEvent, TreeModel};
use crate::{DragSource, DropTarget, DropZones, Id, Insertion, Response, TreeNodeDrag, Ui};
use std::collections::HashMap;

pub(super) struct Dragged<'a> {
    pub tree: Id,
    pub node: Id,
    pub rows: &'a [Row],
    pub index: &'a HashMap<Id, usize>,
    pub response: Response,
    pub indent: f32,
    pub branch: bool,
    /// The tree has focus and this is its focused row.
    pub focused: bool,
}

/// True if `node` is `ancestor` or lies below it in the visible rows. Every
/// ancestor of a visible row is itself a visible, open row, so the walk needs no model.
fn within(rows: &[Row], index: &HashMap<Id, usize>, node: Id, ancestor: Id) -> bool {
    let mut current = Some(node);
    for _ in 0..=rows.len() {
        match current {
            Some(id) if id == ancestor => return true,
            Some(id) => current = index.get(&id).and_then(|&i| rows[i].parent),
            None => return false,
        }
    }
    false
}

pub(super) fn attach(ui: &mut Ui<'_>, row: Dragged<'_>, events: &mut Vec<TreeEvent>) {
    let Dragged {
        tree,
        node,
        rows,
        index,
        response,
        indent,
        branch,
        focused,
    } = row;
    DragSource::new(node, TreeNodeDrag { tree, node })
        .keyboard(focused)
        .focus_owner(tree)
        .attach(ui, response);
    let out = DropTarget::new(node, |dragged: &TreeNodeDrag| {
        dragged.tree == tree && !within(rows, index, node, dragged.node)
    })
    .zones(DropZones::tree().inside(branch))
    .line_indent(indent)
    .attach(ui, response);
    if let Some(drop) = out.dropped {
        events.push(TreeEvent::Moved {
            node: drop.payload.node,
            target: node,
            position: drop.insertion.unwrap_or(Insertion::After),
        });
    }
}

/// A node scrolled out of the virtual window is still being dragged if the
/// model still has it.
pub(super) fn keep_alive(ui: &mut Ui<'_>, tree: Id, model: &impl TreeModel) {
    let dragged = ui
        .context
        .drag
        .session
        .as_ref()
        .and_then(|session| session.payload.as_ref())
        .and_then(|payload| payload.get::<TreeNodeDrag>().copied())
        .filter(|dragged| dragged.tree == tree && model.node(dragged.node).is_some());
    if let Some(dragged) = dragged {
        ui.keep_drag_source(dragged.node);
    }
}
