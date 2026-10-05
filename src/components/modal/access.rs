//! What a modal tells assistive technology: a dialog named by its title.

use crate::{AccessNode, Context};
use std::ops::Range;

/// The dialog node as it is opened; [`name`] completes it once the header was built.
pub(super) fn dialog(node: &mut AccessNode, label: Option<&str>) {
    node.modal();
    if let Some(label) = label {
        node.label(label);
    }
}

/// Name the dialog node `dialog` after the first text of its header (unless it was given
/// a name) and, when `described`, describe it with the second one. `header` is the range
/// of nodes the header added.
pub(super) fn name(context: &mut Context, dialog: usize, header: Range<usize>, described: bool) {
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
