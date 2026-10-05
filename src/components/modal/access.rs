//! What a modal tells assistive technology: a dialog named by its title.

use crate::{accessibility::Scope, AccessNode, AccessRole, Context, Id, Rect};
use std::ops::Range;

/// The dialog layer, opened before the surface is built.
pub(super) struct Opened {
    node: usize,
    scope: Scope,
}

/// Open the dialog node of modal `id` as a layer; what the surface builds goes in it.
pub(super) fn open(context: &mut Context, id: Id, role: AccessRole, label: Option<&str>) -> Opened {
    let node = context.a11y_len();
    let scope = context.a11y_begin_layer(id, id, role, |node| dialog(node, label));
    Opened { node, scope }
}

/// Name the dialog after its `header` nodes and close it at the settled `bounds`
/// (surface, clip) of the surface, also while it animates. A closing layer is dropped
/// from the tree with everything in it.
pub(super) fn close(
    context: &mut Context,
    opened: Opened,
    header: Range<usize>,
    described: bool,
    bounds: (Rect, Rect),
    closing: Option<Id>,
) {
    name(context, opened.node, header, described);
    context.a11y_end(opened.scope, Some(bounds));
    if let Some(id) = closing {
        context.drop_layer_semantics(id);
    }
}

/// The dialog node as it is opened; [`name`] completes it once the header was built.
fn dialog(node: &mut AccessNode, label: Option<&str>) {
    node.modal();
    if let Some(label) = label {
        node.label(label);
    }
}

/// Name the dialog node `dialog` after the first text of its header (unless it was given
/// a name) and, when `described`, describe it with the second one. `header` is the range
/// of nodes the header added.
fn name(context: &mut Context, dialog: usize, header: Range<usize>, described: bool) {
    if !context.a11y_on() {
        return;
    }
    let (mut title, mut description) = (None, None);
    for index in header {
        let Some(node) = context.a11y_node_mut(index) else {
            break;
        };
        let text = node.value.as_deref().filter(|text| !text.trim().is_empty());
        match text.filter(|_| node.role.is_text()) {
            Some(_) if title.is_none() => title = Some(node.id),
            Some(text) => {
                description = Some(text.to_owned());
                break;
            }
            None => {}
        }
    }
    let Some(node) = context.a11y_node_mut(dialog) else {
        return;
    };
    if let Some(title) = title.filter(|_| node.label.is_none()) {
        node.labelled_by(title);
    }
    if let Some(description) = description.filter(|_| described) {
        node.description(description);
    }
}
