//! What a combo box tells assistive technology: the trigger and what it owns while open.

use crate::{AccessActionKind as Kind, AccessNode, Id};

/// The closed description of the trigger; its value and open state follow once the pass's
/// input has been applied.
pub(super) fn trigger(node: &mut AccessNode, id: Id, label: &str, enabled: bool) {
    node.label(label)
        .disabled(!enabled)
        .has_popup()
        .clicks(id)
        .action(Kind::Expand)
        .action(Kind::Collapse)
        .action(Kind::SetValue);
}

/// The trigger once the pass's input is applied: the chosen label or the placeholder,
/// whether the list is open and whether the value is invalid.
pub(super) fn trigger_state(
    node: &mut AccessNode,
    value: Option<&str>,
    placeholder: &str,
    open: bool,
    invalid: bool,
) {
    match value {
        Some(label) => node.value(label),
        None => node.placeholder(placeholder),
    };
    node.expanded(open).invalid(invalid);
}

/// The node that holds focus while the list is open (the trigger, or the filter field)
/// points at the list and at the highlighted option.
pub(super) fn popup_owner(node: &mut AccessNode, combo: Id, list: Id, active: Option<Id>) {
    node.controls(list);
    if let Some(option) = active {
        node.active_descendant(combo.with(("option", option)));
    }
}
