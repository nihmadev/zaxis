//! The table for assistive technology: a `Table` that scrolls, a header row of column
//! headers, then the built rows with their cells. Counts and indices describe the whole
//! data set, also the rows a virtualized body did not build.
use super::{Table, Ui};
use crate::{accessibility::Scope, AccessRole, Context, Id, Rect};

impl Table {
    /// The name a screen reader speaks for the table ("Open orders"). The table draws no
    /// caption of its own, so without this it is announced as an unnamed table.
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

/// The table's node and its header row. The header is painted after the body, which
/// decides the horizontal offset, but comes first in the tree: its row is created up front
/// and filled when the header is painted.
pub(super) struct Nodes {
    table: Scope,
    header: Option<u32>,
}

pub(super) fn begin(ui: &mut Ui<'_>, id: Id, label: &str, columns: usize) -> Nodes {
    let table = ui.a11y_begin(id, AccessRole::Table, |node| {
        node.label(label);
        node.table().column_count = Some(columns as u32);
    });
    let row = ui.a11y_begin(id.with("header-row"), AccessRole::Row, |node| {
        node.table().row = Some(0);
    });
    let header = row.0;
    ui.a11y_end(row, None);
    Nodes { table, header }
}

impl Nodes {
    /// Column headers described from here on belong to the header row.
    pub(super) fn open_header(&self, context: &mut Context) {
        if let Some(header) = self.header {
            context.a11y.stack.push(header);
        }
    }

    /// Close the header row and the table. `rows` is the number of data rows in the whole
    /// set and `body` the scroll area the rows live in.
    pub(super) fn finish(self, ui: &mut Ui<'_>, header: Rect, rect: Rect, rows: usize, body: Id) {
        ui.a11y_end(Scope(self.header), Some(header));
        if let Some(node) = self
            .table
            .0
            .and_then(|at| ui.context.a11y_node_mut(at as usize))
        {
            node.table().row_count = Some(rows.saturating_add(1).min(u32::MAX as usize) as u32);
        }
        ui.context.a11y_scroll(&self.table, body);
        ui.a11y_end(self.table, Some(rect));
    }
}
