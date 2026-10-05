//! Pointer selection of a block: press, double and triple click, shift-click, and the
//! drag that follows. The state it changes lives in the context
//! ([`SelectionState`](crate::context)), shared by every block of the window.

use super::state::BlockState;
use crate::{
    components::edit_buffer::{para_end, para_start, word_at},
    components::Ui,
    context::selection::Endpoint,
    Id, Rect, Vec2,
};

/// Where a block sits for this pass.
#[derive(Clone, Copy)]
pub(crate) struct Place {
    pub id: Id,
    pub scope: Id,
    pub ord: usize,
    pub rect: Rect,
}

/// Source byte under `local` (relative to the block's origin).
pub(crate) fn byte_at(ui: &mut Ui<'_>, state: &mut BlockState, local: Vec2) -> usize {
    let BlockState {
        doc, text, display, ..
    } = &mut *state;
    let shown: &str = display.as_deref().unwrap_or(text);
    let (pos, _) = doc.hit_point(ui.context, shown, local);
    let byte = state.to_source(pos.byte);
    state.clamp(byte)
}

/// React to a press on the block or one of its links, and feed an ongoing drag.
/// `targets` are the focus targets of the block: its own ID and its links'.
pub(crate) fn handle(
    ui: &mut Ui<'_>,
    state: &mut BlockState,
    place: Place,
    targets: impl Iterator<Item = Id>,
) {
    let Place {
        id,
        scope,
        ord,
        rect,
    } = place;
    let origin = rect.min;
    ui.context.selection_touch(id, ord);
    for target in targets {
        let Some((point, mods)) = ui.context.press_point(target) else {
            continue;
        };
        let byte = byte_at(ui, state, point - origin);
        let shift = mods.shift_key();
        let count = ui.context.selection_press(target, point, shift);
        let at = Endpoint {
            item: id,
            byte,
            ord,
        };
        let extends = shift
            && ui
                .context
                .selection
                .selection
                .is_some_and(|s| s.scope == scope);
        if extends {
            ui.context.selection_extend(scope, at);
        } else if count == 2 && target == id {
            let range = word_at(&state.text, byte);
            ui.context.selection_set(scope, id, ord, range);
        } else if count == 3 && target == id {
            let range = para_start(&state.text, byte)..para_end(&state.text, byte);
            ui.context.selection_set(scope, id, ord, range);
        } else {
            ui.context.selection_begin(scope, at);
        }
        let multi = count > 1 && !extends;
        ui.context.selection_drag_begin(target, scope, point, multi);
        break;
    }
    if ui.context.selection_dragging(scope) {
        let pointer = ui.context.text_edit_pointer(id);
        if let Some(pointer) = pointer.filter(|p| p.y >= rect.min.y) {
            let byte = if pointer.y > rect.max.y {
                state.text.len()
            } else {
                byte_at(ui, state, pointer - origin)
            };
            ui.context.selection_offer(Endpoint {
                item: id,
                byte,
                ord,
            });
        }
    }
}
