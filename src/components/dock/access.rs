use crate::{AccessRole, Id, Rect, Ui};

pub(super) fn root(ui: &mut Ui<'_>, id: Id, name: &str) -> crate::accessibility::Scope {
    ui.a11y_begin(id, AccessRole::Group, |node| {
        node.label(name);
    })
}
pub(super) fn panel(
    ui: &mut Ui<'_>,
    id: Id,
    rect: Rect,
    title: &str,
    selected: bool,
) -> crate::accessibility::Scope {
    let scope = ui.a11y_begin(id, AccessRole::Group, |node| {
        node.label(title).selected(selected);
    });
    // Ended by the caller after building its controls; the ring has no semantic node.
    let _ = rect;
    scope
}
