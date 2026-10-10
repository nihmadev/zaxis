//! What the strip tells assistive technology: a tab list of tabs, each with its close
//! button as a button of its own. The active tab is the one that is selected, and the
//! pages built after the strip (`Ui::tab_pages`) find it by that.

use crate::{components::Ui, AccessOrientation, AccessRole, Id, Rect};

pub(super) struct Described<'a> {
    pub id: Id,
    pub rect: Rect,
    /// The name: the label, or for a tab of an icon alone its tooltip.
    pub name: &'a str,
    pub selected: bool,
    pub index: usize,
    pub count: usize,
    pub enabled: bool,
    /// The close button, when the tab has one: its id and hit area.
    pub close: Option<(Id, Rect)>,
}

pub(super) fn list(ui: &mut Ui<'_>, id: Id, enabled: bool) -> crate::accessibility::Scope {
    ui.a11y_begin(id, AccessRole::TabList, |node| {
        node.orientation(AccessOrientation::Horizontal)
            .disabled(!enabled);
    })
}

pub(super) fn tab(ui: &mut Ui<'_>, tab: &Described<'_>) {
    if !ui.context.a11y_on() {
        return;
    }
    let scope = ui.a11y_begin(tab.id, AccessRole::Tab, |node| {
        node.label(tab.name)
            .selected(tab.selected)
            .position_in_set(tab.index, tab.count)
            .disabled(!tab.enabled)
            .clicks(tab.id);
    });
    if let Some((id, rect)) = tab.close {
        ui.a11y(id, rect, AccessRole::Button, |node| {
            node.label(format!("Close {}", tab.name))
                .disabled(!tab.enabled)
                .clicks(id);
        });
    }
    ui.a11y_end(scope, Some(tab.rect));
}
