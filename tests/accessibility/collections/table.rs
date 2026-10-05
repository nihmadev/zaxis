use super::*;
use zaxis::accesskit::{Point, SortDirection as Sorted};

const ROW: f32 = 28.0;

fn columns() -> [Column; 3] {
    [
        Column::fixed("name", 140.0).title("Name").sortable(true),
        Column::fixed("status", 100.0).title("Status"),
        Column::remainder("action").min_width(80.0).title("Action"),
    ]
}

struct Sheet {
    harness: Harness,
    /// `None` builds every row; a number virtualizes that many.
    rows: Option<usize>,
    selectable: bool,
    enabled: bool,
    sorts: Vec<SortRequest>,
    clicked: Vec<Id>,
    opened: Vec<usize>,
    selected: Option<Id>,
}

impl Sheet {
    fn new(rows: Option<usize>) -> Self {
        let mut sheet = Self {
            harness: Harness::new(),
            rows,
            selectable: true,
            enabled: true,
            sorts: Vec::new(),
            clicked: Vec::new(),
            opened: Vec::new(),
            selected: None,
        };
        sheet.pass();
        sheet
    }
    fn pass(&mut self) -> Option<usize> {
        let (rows, selectable, enabled) = (self.rows, self.selectable, self.enabled);
        let (sorts, clicked, opened, selected) = (
            &mut self.sorts,
            &mut self.clicked,
            &mut self.opened,
            &mut self.selected,
        );
        self.harness.pass(|ctx| {
            Root::new().show(ctx, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    let table = Table::new("orders")
                        .accessible_label("Orders")
                        .columns(columns())
                        .selectable(selectable)
                        .max_height(240.0);
                    let mut line = |body: &mut TableBody<'_, '_>, n: usize| {
                        body.row(n, |row| {
                            row.cell(|ui| {
                                ui.label(format!("Order {n}"));
                            });
                            row.cell(|ui| {
                                ui.label("Ready");
                            });
                            row.cell(|ui| {
                                if ui.button("Open").clicked() {
                                    opened.push(n);
                                }
                            });
                        });
                    };
                    let (sort, click, chosen) = match rows {
                        Some(total) => {
                            let out = table.show_rows(ui, ROW, total, |body, n| line(body, n));
                            (out.sort_request, out.row_clicked, out.selected_row)
                        }
                        None => {
                            let out = table.show(ui, |body| (0..4).for_each(|n| line(body, n)));
                            (out.sort_request, out.row_clicked, out.selected_row)
                        }
                    };
                    sorts.extend(sort);
                    clicked.extend(click);
                    *selected = chosen;
                });
            });
        })
    }
    fn settle(&mut self) {
        for _ in 0..4 {
            self.pass();
        }
    }
    fn table(&self) -> NodeId {
        self.harness.tree.expect(Role::Table, "Orders")
    }
    /// The row whose first cell says `name`.
    fn row(&self, name: &str) -> NodeId {
        let tree = &self.harness.tree;
        let cell = tree.parent(tree.expect(Role::Label, name)).expect("a cell");
        assert_eq!(tree.node(cell).role(), Role::Cell);
        tree.parent(cell).expect("a row")
    }
}

#[test]
fn a_table_publishes_headers_rows_and_cells_with_their_indices() {
    let mut sheet = Sheet::new(None);
    let (tree, table) = (&sheet.harness.tree, sheet.table());
    let node = tree.node(table);
    assert_eq!(
        (node.row_count(), node.column_count()),
        (Some(5), Some(3)),
        "header included"
    );
    assert!(
        tree.all(Role::ScrollView).is_empty(),
        "the table is the scroll container"
    );
    let rows = children(&sheet.harness, table, Role::Row);
    assert_eq!(rows.len(), 5);
    assert_eq!(
        tree.node(table).children()[0],
        rows[0],
        "the header row comes first"
    );
    let headers = children(&sheet.harness, rows[0], Role::ColumnHeader);
    let names: Vec<_> = headers.iter().map(|id| tree.name(*id)).collect();
    assert_eq!(names, ["Name", "Status", "Action"]);
    for (column, id) in headers.iter().enumerate() {
        let header = tree.node(*id);
        assert_eq!(
            (header.row_index(), header.column_index()),
            (Some(0), Some(column))
        );
        assert_eq!(
            header.supports_action(Action::Click),
            column == 0,
            "only Name sorts"
        );
        assert_eq!(header.sort_direction(), None);
    }
    for (index, row) in rows.iter().enumerate().skip(1) {
        assert_eq!(tree.node(*row).row_index(), Some(index));
        assert_eq!(tree.node(*row).is_selected(), Some(false));
        let cells = children(&sheet.harness, *row, Role::Cell);
        assert_eq!(cells.len(), 3);
        for (column, cell) in cells.iter().enumerate() {
            let node = tree.node(*cell);
            assert_eq!(
                (node.row_index(), node.column_index()),
                (Some(index), Some(column))
            );
        }
        // What a cell shows is inside the cell.
        let text = children(&sheet.harness, cells[0], Role::Label);
        assert_eq!(
            tree.node(text[0]).value(),
            Some(format!("Order {}", index - 1).as_str())
        );
        assert_eq!(children(&sheet.harness, cells[2], Role::Button).len(), 1);
        // Cells tile the row, left to right, under their headers.
        let (cell, header) = (
            logical(&sheet.harness, cells[1]),
            logical(&sheet.harness, headers[1]),
        );
        assert!((cell.min.x - header.min.x).abs() < 1.0 && (cell.max.x - header.max.x).abs() < 1.0);
        let row = logical(&sheet.harness, *row);
        assert!(cell.min.y >= row.min.y - 0.5 && cell.max.y <= row.max.y + 0.5);
    }
    assert_eq!(sheet.pass(), None, "an idle pass publishes nothing");
}

#[test]
fn a_click_on_a_sortable_header_sorts_once_and_publishes_the_direction() {
    let mut sheet = Sheet::new(None);
    let name = sheet.harness.tree.expect(Role::ColumnHeader, "Name");
    let column = Id::new("name");
    assert!(sheet.harness.act(name, Action::Click));
    sheet.settle();
    let ascending = SortRequest {
        column,
        direction: SortDirection::Ascending,
    };
    assert_eq!(sheet.sorts, [ascending], "one request, one sort");
    assert_eq!(
        sheet.harness.tree.node(name).sort_direction(),
        Some(Sorted::Ascending)
    );
    assert!(sheet.harness.act(name, Action::Click));
    sheet.settle();
    let descending = SortRequest {
        column,
        direction: SortDirection::Descending,
    };
    assert_eq!(sheet.sorts, [ascending, descending]);
    assert_eq!(
        sheet.harness.tree.node(name).sort_direction(),
        Some(Sorted::Descending)
    );
    // The pointer does the same.
    let at = logical(&sheet.harness, name).center();
    click(&mut sheet.harness.context, at);
    sheet.settle();
    assert_eq!(sheet.sorts, [ascending, descending, ascending]);
    // A header that does not sort refuses the click.
    let status = sheet.harness.tree.expect(Role::ColumnHeader, "Status");
    assert!(!sheet.harness.act(status, Action::Click));
    sheet.settle();
    assert_eq!(sheet.sorts.len(), 3);
    assert_eq!(sheet.pass(), None);
}

#[test]
fn a_click_on_a_row_selects_it_and_a_button_in_a_cell_stays_a_button() {
    let mut sheet = Sheet::new(None);
    let row = sheet.row("Order 2");
    assert!(sheet.harness.act(row, Action::Click));
    sheet.settle();
    assert_eq!(sheet.clicked, [Id::new(2usize)], "one request, one click");
    assert_eq!(sheet.selected, Some(Id::new(2usize)));
    assert_eq!(sheet.harness.tree.node(row).is_selected(), Some(true));
    assert_eq!(
        sheet.harness.tree.node(sheet.row("Order 1")).is_selected(),
        Some(false)
    );

    let button = children(&sheet.harness, sheet.row("Order 0"), Role::Cell)[2];
    let button = children(&sheet.harness, button, Role::Button)[0];
    assert!(sheet.harness.act(button, Action::Click));
    sheet.settle();
    assert_eq!(sheet.opened, [0]);
    assert_eq!(
        sheet.clicked.len(),
        1,
        "the row under the button was not clicked"
    );
    assert_eq!(sheet.pass(), None);

    // Without selection rows say nothing about it and take no clicks.
    sheet.selectable = false;
    sheet.settle();
    let row = sheet.row("Order 1");
    assert_eq!(sheet.harness.tree.node(row).is_selected(), None);
    assert!(!sheet.harness.act(row, Action::Click));
    sheet.settle();
    assert_eq!(sheet.clicked.len(), 1);
}

#[test]
fn a_disabled_table_refuses_requests() {
    let mut sheet = Sheet::new(None);
    sheet.enabled = false;
    sheet.settle();
    let (row, name) = (
        sheet.row("Order 1"),
        sheet.harness.tree.expect(Role::ColumnHeader, "Name"),
    );
    assert!(sheet.harness.tree.node(sheet.table()).is_disabled());
    assert!(!sheet.harness.act(row, Action::Click));
    assert!(!sheet.harness.act(name, Action::Click));
    sheet.settle();
    assert!(sheet.clicked.is_empty() && sheet.sorts.is_empty());
}

#[test]
fn a_virtualized_table_describes_the_whole_set_from_the_rows_it_built() {
    let mut sheet = Sheet::new(Some(5000));
    let table = sheet.table();
    let node = sheet.harness.tree.node(table);
    assert_eq!(
        (node.row_count(), node.column_count()),
        (Some(5001), Some(3))
    );
    let built = children(&sheet.harness, table, Role::Row).len();
    assert!(built < 16, "header and the rows near the viewport: {built}");
    assert!(sheet.harness.context.accessibility_stats().nodes < 16 * 8);
    assert_eq!(
        sheet.harness.tree.node(sheet.row("Order 3")).row_index(),
        Some(4)
    );
    assert_eq!(sheet.pass(), None);

    let far = ActionData::SetScrollOffset(Point::new(0.0, f64::from(4000.0 * ROW)));
    assert!(sheet.harness.act_with(table, Action::SetScrollOffset, far));
    sheet.settle();
    assert_eq!(
        sheet.harness.tree.node(table).scroll_y(),
        Some(f64::from(4000.0 * ROW))
    );
    let row = sheet.row("Order 4000");
    assert_eq!(sheet.harness.tree.node(row).row_index(), Some(4001));
    let cells = children(&sheet.harness, row, Role::Cell);
    assert_eq!(sheet.harness.tree.node(cells[2]).row_index(), Some(4001));
    assert!(children(&sheet.harness, table, Role::Row).len() < 16);
    assert!(sheet.harness.tree.find(Role::Label, "Order 3").is_none());
    // The header stays, first, above the rows.
    let header = children(&sheet.harness, table, Role::Row)[0];
    assert_eq!(
        children(&sheet.harness, header, Role::ColumnHeader).len(),
        3
    );
    assert!(logical(&sheet.harness, header).max.y <= logical(&sheet.harness, row).min.y + 0.5);

    assert!(sheet.harness.act(row, Action::Click));
    sheet.settle();
    assert_eq!(sheet.clicked, [Id::new(4000usize)]);
    assert_eq!(sheet.harness.tree.node(row).is_selected(), Some(true));
    // A row below the viewport edge is revealed by scrolling the table.
    let below = sheet.row("Order 4006");
    assert!(sheet.harness.act(below, Action::ScrollIntoView));
    sheet.settle();
    let (rect, port) = (
        logical(&sheet.harness, below),
        logical(&sheet.harness, table),
    );
    assert!(rect.max.y <= port.max.y + 0.5, "{rect:?} in {port:?}");
    assert_eq!(sheet.pass(), None);
}

#[test]
fn a_layout_grid_adds_no_nodes_of_its_own() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        Root::new().show(ctx, |ui| {
            ui.grid(
                "form",
                [Column::content("label"), Column::remainder("field")],
                |grid| {
                    grid.row("name", |row| {
                        row.cell(|ui| {
                            ui.label("Name");
                        });
                        row.cell(|ui| {
                            ui.button("Change");
                        });
                    });
                },
            );
        });
    };
    harness.pass(build);
    for role in [Role::Table, Role::Grid, Role::Row, Role::Cell] {
        assert!(harness.tree.all(role).is_empty(), "{role:?}");
    }
    // Its widgets are in the tree as if the grid were not there.
    let (label, button) = (
        harness.tree.expect(Role::Label, "Name"),
        harness.tree.expect(Role::Button, "Change"),
    );
    assert_eq!(harness.tree.parent(label), harness.tree.parent(button));
    assert!(logical(&harness, label).max.x <= logical(&harness, button).min.x + 0.5);
    assert_eq!(harness.pass(build), None);
}
