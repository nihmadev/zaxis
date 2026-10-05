//! Cost: the tree of a virtualized collection grows with the rows on screen, not with the
//! data, and nothing is published while nothing changes.
use super::*;

const ROWS: usize = 5000;

fn list(ctx: &mut Context) {
    Root::new().show(ctx, |ui| {
        ListBox::new("big")
            .accessible_label("Big list")
            .row_height(24.0)
            .max_height(240.0)
            .show_rows(
                ui,
                ROWS,
                |i| ListEntry::item(i, "Row"),
                |ui, row| {
                    ui.label(format!("Row {}", row.index));
                },
            );
    });
}

fn table(ctx: &mut Context) {
    Root::new().show(ctx, |ui| {
        Table::new("big")
            .accessible_label("Big table")
            .columns([
                Column::fixed("a", 120.0).title("A"),
                Column::remainder("b").title("B"),
            ])
            .max_height(240.0)
            .show_rows(ui, 24.0, ROWS, |body, n| {
                body.row(n, |row| {
                    row.cell(|ui| {
                        ui.label(format!("A{n}"));
                    });
                    row.cell(|ui| {
                        ui.label(format!("B{n}"));
                    });
                });
            });
    });
}

struct Flat(usize);
impl TreeModel for Flat {
    fn revision(&self) -> u64 {
        0
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        (0..self.0).map(Id::new)
    }
    fn children(&self, _: Id) -> impl Iterator<Item = Id> {
        std::iter::empty()
    }
    fn node(&self, _: Id) -> Option<TreeNode<'_>> {
        Some(TreeNode::leaf("Node"))
    }
}

fn tree(ctx: &mut Context) {
    Root::new().show(ctx, |ui| {
        TreeView::new("big")
            .accessible_label("Big tree")
            .max_height(240.0)
            .show(ui, &Flat(2000));
    });
}

/// Rows of `role` under the collection, nodes in the whole tree, and nodes sent by one idle pass.
fn measure(build: fn(&mut Context), container: Role, row: Role) -> (usize, u64, Option<usize>) {
    let mut harness = Harness::new();
    let first = harness.pass(build).expect("the first pass sends the tree");
    harness.settle(build);
    let stats = harness.context.accessibility_stats();
    assert_eq!(
        first as u64, stats.nodes_sent,
        "nothing was resent while idle"
    );
    let root = harness.tree.all(container)[0];
    let rows = children(&harness, root, row).len();
    let idle = harness.pass(build);
    (rows, stats.nodes, idle)
}

#[test]
fn a_list_of_five_thousand_rows_publishes_the_rows_it_built() {
    let (rows, nodes, idle) = measure(list, Role::ListBox, Role::ListBoxOption);
    assert!(
        (10..=12).contains(&rows),
        "ten rows fit, one more on each side: {rows}"
    );
    // Root, list, and an option with its text per built row.
    assert_eq!(nodes, 2 + 2 * rows as u64);
    assert_eq!(idle, None);
}

#[test]
fn a_tree_of_two_thousand_nodes_publishes_the_rows_it_built() {
    let (rows, nodes, idle) = measure(tree, Role::Tree, Role::TreeItem);
    assert!((5..=16).contains(&rows), "{rows}");
    assert_eq!(nodes, 2 + rows as u64);
    assert_eq!(idle, None);
}

#[test]
fn a_table_of_five_thousand_rows_publishes_the_rows_it_built() {
    let (rows, nodes, idle) = measure(table, Role::Table, Role::Row);
    assert!(
        (6..=12).contains(&rows),
        "the header and the rows in view: {rows}"
    );
    // Root, table, the header row with two headers, and per data row two cells with a label each.
    assert_eq!(nodes, 2 + 3 + 5 * (rows as u64 - 1));
    assert_eq!(idle, None);
}

#[test]
fn collections_cost_nothing_while_no_assistive_technology_is_connected() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    for build in [list as fn(&mut Context), tree, table] {
        context.run(build);
        context.run(build);
    }
    assert_eq!(context.accessibility_stats().passes, 0);
    assert!(context.take_accessibility_update().is_none());
}

#[test]
fn every_scale_factor_gives_the_same_logical_rows() {
    let mut reference: Option<Vec<Rect>> = None;
    for scale in [1.0, 1.5, 2.0] {
        let mut harness = Harness::with_scale(scale);
        harness.pass(list);
        let root = harness.tree.expect(Role::ListBox, "Big list");
        let rows = children(&harness, root, Role::ListBoxOption);
        let rects: Vec<Rect> = rows
            .iter()
            .take(8)
            .map(|row| logical(&harness, *row))
            .collect();
        let expected = reference.get_or_insert_with(|| rects.clone());
        for (got, want) in rects.iter().zip(expected.iter()) {
            assert!(
                (got.min - want.min).abs().max_element() <= 1.0,
                "scale {scale}: {got:?} {want:?}"
            );
            assert!(
                (got.max - want.max).abs().max_element() <= 1.0,
                "scale {scale}: {got:?} {want:?}"
            );
        }
    }
}
