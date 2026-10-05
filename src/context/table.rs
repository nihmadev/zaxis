//! Tables: column resize drags routed through the pointer capture, and the retained state
//! of every table. Unlike other containers, a table keeps its state while it is not
//! built: user widths, sorting and selection survive hiding and reordering it.
use super::{Context, Id};
use crate::components::table::TableState;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Tables {
    pub(crate) states: HashMap<Id, TableState>,
    /// Widths set by resize drags, waiting for the table's next pass.
    resized: HashMap<(Id, Id), f32>,
}

impl Context {
    /// A press on the resize handle of `column` starts from its current width.
    pub(super) fn column_resize_press(&mut self, table: Id, column: Id) {
        if let Some(state) = self.tables.states.get_mut(&table) {
            if let Some(&width) = state.current_widths.get(&column) {
                state.drag = Some((column, width));
            }
        }
    }

    /// The captured handle of `column` moved `delta` from where it was pressed.
    pub(super) fn column_resize_move(&mut self, table: Id, column: Id, delta: f32) {
        let Some((dragged, width)) = self.tables.states.get(&table).and_then(|s| s.drag) else {
            return;
        };
        if dragged == column {
            self.tables
                .resized
                .insert((table, column), (width + delta).max(0.0));
        }
    }

    /// The drag ends without a release (a modal opened over it); widths stay as they are.
    pub(super) fn cancel_column_resize(&mut self, table: Id) {
        if let Some(state) = self.tables.states.get_mut(&table) {
            state.drag = None;
        }
    }

    /// A resize drag keeps its capture while the pointer is off the handle, as long as the
    /// table was built in this pass with that column resizable, in a visible window.
    pub(super) fn column_resize_holds(&self, table: Id, column: Id, window: Id) -> bool {
        self.tables.states.get(&table).is_some_and(|state| {
            state.last_frame == self.frame && state.resize_columns.contains(&column)
        }) && self.visible_windows.contains(&window)
    }

    pub(crate) fn take_column_resize(&mut self, table: Id, column: Id) -> Option<f32> {
        self.tables.resized.remove(&(table, column))
    }
}
