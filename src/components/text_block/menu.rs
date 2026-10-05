//! The context menu of a block: Copy and Select all on the text, Copy link address and Copy
//! text on a link. Opened by a secondary click; its items act on the same selection and
//! clipboard as the keyboard.

use super::{
    links::LinkRun,
    state::{BlockState, MenuTarget},
};
use crate::{
    components::{ContextMenu, ContextMenuItem, Response, Ui},
    Id,
};

const COPY: &str = "copy";
const SELECT_ALL: &str = "select-all";
const COPY_ADDRESS: &str = "copy-address";
const COPY_TEXT: &str = "copy-text";

/// `text` is the response of the block's own region when it is selectable; `anchor` is any
/// response of the block, which the menu hangs on.
pub(crate) fn show(
    ui: &mut Ui<'_>,
    state: &mut BlockState,
    id: Id,
    scope: Id,
    text: Option<Response>,
    links: &[LinkRun],
    anchor: Response,
) {
    let mut open_at = None;
    if let Some(r) = text.filter(|r| r.secondary_clicked()) {
        open_at = ui.context.gestures.secondary_position(r.id);
        state.menu = Some(MenuTarget::Text);
    }
    for link in links {
        if link.response.is_some_and(|r| r.secondary_clicked()) {
            open_at = ui.context.gestures.secondary_position(link.id);
            state.menu = Some(MenuTarget::Link(link.index));
        }
    }
    let Some(target) = state.menu else {
        return;
    };
    let items = match target {
        MenuTarget::Text => {
            let selected = ui.context.selected_text().is_some_and(|t| !t.is_empty());
            vec![
                ContextMenuItem::new(COPY, "Copy").enabled(selected),
                ContextMenuItem::new(SELECT_ALL, "Select all").enabled(!state.text.is_empty()),
            ]
        }
        MenuTarget::Link(index) => {
            let url = state.links.get(index).is_some_and(|l| l.url.is_some());
            vec![
                ContextMenuItem::new(COPY_ADDRESS, "Copy link address").enabled(url),
                ContextMenuItem::new(COPY_TEXT, "Copy text"),
            ]
        }
    };
    let mut menu = ContextMenu::new((id, "menu"), &items);
    if let Some(position) = open_at {
        menu = menu.open_at(position);
    }
    let output = menu.show(ui, anchor);
    if let Some(chosen) = output.selected {
        let link = match target {
            MenuTarget::Link(i) => state.links.get(i).cloned(),
            MenuTarget::Text => None,
        };
        if chosen == Id::new(COPY) {
            ui.context.selection_copy();
        } else if chosen == Id::new(SELECT_ALL) {
            ui.context.selection_select_all(scope);
            ui.context.request_focus(id);
        } else if chosen == Id::new(COPY_ADDRESS) {
            if let Some(url) = link.and_then(|l| l.url) {
                let _ = ui.context.copy_text(url);
            }
        } else if chosen == Id::new(COPY_TEXT) {
            if let Some(link) = link {
                let _ = ui.context.copy_text(state.copy_range(link.range, false));
            }
        }
    }
    if !output.open {
        state.menu = None;
    }
}
