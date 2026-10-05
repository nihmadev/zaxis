//! TreeView, Table and Grid row dragging through their public builders.
use super::*;
use std::{collections::HashSet, ops::Range};
use zaxis::{
    Column, Grid, RowMove, Table, TableOutput, TreeChildren, TreeEvent, TreeModel, TreeNode,
    TreeOutput, TreeView,
};

fn n(value: u64) -> Id {
    Id::new(value)
}

/// 1 { 11, 12 }, 2 { 21 }, 3 (leaf)
struct Model {
    nodes: Vec<(u64, Option<u64>, TreeChildren)>,
    labels: Vec<String>,
    revision: u64,
}
impl Model {
    fn new() -> Self {
        let nodes = vec![
            (1, None, TreeChildren::Loaded),
            (11, Some(1), TreeChildren::Leaf),
            (12, Some(1), TreeChildren::Leaf),
            (2, None, TreeChildren::Loaded),
            (21, Some(2), TreeChildren::Leaf),
            (3, None, TreeChildren::Leaf),
        ];
        Self {
            labels: nodes.iter().map(|n| format!("Node {}", n.0)).collect(),
            nodes,
            revision: 0,
        }
    }
}
impl TreeModel for Model {
    fn revision(&self) -> u64 {
        self.revision
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        self.nodes
            .iter()
            .filter(|n| n.1.is_none())
            .map(|n| Id::new(n.0))
    }
    fn children(&self, id: Id) -> impl Iterator<Item = Id> {
        self.nodes
            .iter()
            .filter(move |n| n.1.map(Id::new) == Some(id))
            .map(|n| Id::new(n.0))
    }
    fn node(&self, id: Id) -> Option<TreeNode<'_>> {
        let i = self.nodes.iter().position(|n| Id::new(n.0) == id)?;
        Some(TreeNode::leaf(&self.labels[i]).children(self.nodes[i].2))
    }
    fn parent(&self, id: Id) -> Option<Id> {
        self.nodes
            .iter()
            .find(|n| Id::new(n.0) == id)
            .and_then(|n| n.1)
            .map(Id::new)
    }
}

struct Tree {
    context: Context,
    model: Model,
    open: HashSet<Id>,
    selected: Option<Id>,
    out: Option<TreeOutput>,
    events: Vec<TreeEvent>,
    step: u64,
    start: Instant,
}
impl Tree {
    fn new() -> Self {
        let mut context = Context::new();
        context.set_viewport(PhysicalSize::new(800, 600), 1.0);
        let mut tree = Self {
            context,
            model: Model::new(),
            open: [n(1), n(2)].into_iter().collect(),
            selected: Some(n(21)),
            out: None,
            events: Vec::new(),
            step: 0,
            start: Instant::now(),
        };
        tree.frame();
        tree.frame();
        tree
    }
    fn frame(&mut self) {
        self.step += 1;
        let now = self.start + Duration::from_millis(self.step * 16);
        let (model, open, selected) = (&self.model, &mut self.open, &mut self.selected);
        let mut out = None;
        self.context.run_at(now, |c| {
            Root::new().show(c, |ui| {
                out = Some(
                    TreeView::new("t")
                        .open(open)
                        .selected(selected)
                        .drag_nodes(true)
                        .max_height(200.0)
                        .show(ui, model),
                );
            });
        });
        let out = out.unwrap();
        self.events.extend(out.events.iter().copied());
        self.out = Some(out);
    }
    fn row(&self, node: u64) -> Rect {
        self.context
            .probe()
            .previous_hits
            .iter()
            .find(|h| {
                matches!(h.action, HitAction::TreeRow { node: x, chevron: false, .. } if x == n(node))
            })
            .unwrap()
            .rect
    }
    fn at(&self, node: u64, fraction: f32) -> Vec2 {
        let r = self.row(node);
        Vec2::new(r.min.x + 60.0, r.min.y + r.size().y * fraction)
    }
    fn drag(&mut self, node: u64) {
        let from = self.at(node, 0.5);
        self.context.move_pointer(from);
        self.context.primary_button(ElementState::Pressed);
        self.context.move_pointer(from + Vec2::new(0.0, 8.0));
        self.frame();
        self.frame();
    }
    fn moves(&self) -> Vec<TreeEvent> {
        self.events
            .iter()
            .copied()
            .filter(|e| matches!(e, TreeEvent::Moved { .. }))
            .collect()
    }
}

#[test]
fn tree_nodes_move_before_after_and_inside_without_touching_the_model() {
    for (target, fraction, expected) in [
        (2, 0.5, Insertion::Inside),
        (3, 0.1, Insertion::Before),
        (21, 0.9, Insertion::After),
    ] {
        let mut t = Tree::new();
        t.drag(11);
        t.context.move_pointer(t.at(target, fraction));
        t.frame();
        t.context.primary_button(ElementState::Released);
        t.frame();
        assert_eq!(
            t.moves(),
            vec![TreeEvent::Moved {
                node: n(11),
                target: n(target),
                position: expected
            }]
        );
        t.frame();
        t.frame();
        assert_eq!(t.moves().len(), 1, "reported once");
        // The library never edits the model; open and selected state is intact.
        assert_eq!(t.model.nodes.len(), 6);
        assert_eq!(t.selected, Some(n(21)));
        assert!(t.open.contains(&n(1)) && t.open.contains(&n(2)));
    }
}

#[test]
fn a_node_cannot_be_dropped_into_itself_or_its_own_subtree() {
    let mut t = Tree::new();
    for target in [1, 11, 12] {
        t.drag(1);
        t.context.move_pointer(t.at(target, 0.5));
        t.frame();
        t.context.primary_button(ElementState::Released);
        t.frame();
        t.frame();
        assert!(t.moves().is_empty(), "target {target}");
    }
    // Leaves never offer an "inside" zone.
    t.drag(11);
    t.context.move_pointer(t.at(3, 0.5));
    t.frame();
    t.context.primary_button(ElementState::Released);
    t.frame();
    assert!(matches!(
        t.moves()[..],
        [TreeEvent::Moved {
            position: Insertion::Before | Insertion::After,
            ..
        }]
    ));
}

#[test]
fn tree_keyboard_picks_up_the_focused_node_with_ctrl_space() {
    let mut t = Tree::new();
    t.context.request_focus(t.out.as_ref().unwrap().id);
    t.frame();
    t.context
        .key(KeyCode::ArrowDown, ElementState::Pressed, false);
    t.frame();
    let focused = t.out.as_ref().unwrap().focused.unwrap();
    // Plain Space still selects, as before.
    t.context.key(KeyCode::Space, ElementState::Pressed, false);
    t.frame();
    assert!(t.context.probe().drag.session.is_none());
    t.context
        .set_modifiers(winit::keyboard::ModifiersState::CONTROL);
    t.context.key(KeyCode::Space, ElementState::Pressed, false);
    t.context
        .set_modifiers(winit::keyboard::ModifiersState::empty());
    t.frame();
    t.frame();
    assert!(t
        .context
        .probe()
        .drag
        .session
        .as_ref()
        .is_some_and(|s| s.keyboard));
    t.context.key(KeyCode::End, ElementState::Pressed, false);
    t.frame();
    t.context.key(KeyCode::Enter, ElementState::Pressed, false);
    t.frame();
    assert_eq!(t.moves().len(), 1);
    let TreeEvent::Moved { node, .. } = t.moves()[0] else {
        unreachable!()
    };
    assert_eq!(node, focused);
}

struct Rows {
    context: Context,
    order: Vec<u64>,
    moved: Option<RowMove>,
    rects: Vec<(Id, Rect)>,
    step: u64,
    start: Instant,
    virtual_rows: bool,
    grid: bool,
}
impl Rows {
    fn new(total: u64, virtual_rows: bool, grid: bool) -> Self {
        let mut context = Context::new();
        context.set_viewport(PhysicalSize::new(800, 600), 1.0);
        let mut rows = Self {
            context,
            order: (0..total).collect(),
            moved: None,
            rects: Vec::new(),
            step: 0,
            start: Instant::now(),
            virtual_rows,
            grid,
        };
        rows.frame();
        rows.frame();
        rows
    }
    fn frame(&mut self) {
        self.step += 1;
        let now = self.start + Duration::from_millis(self.step * 16);
        let (order, virtual_rows, grid) = (&self.order, self.virtual_rows, self.grid);
        let (mut moved, mut rects) = (None, Vec::new());
        self.context.run_at(now, |c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                if let Some(dragged) = ui.dragging() {
                    if order.iter().any(|row| n(*row) == dragged) {
                        ui.keep_drag_source(dragged);
                    }
                }
                if grid {
                    let out = Grid::new("g")
                        .columns([Column::remainder("a").min_width(80.0)])
                        .drag_rows(true)
                        .show(ui, |g| {
                            for row in order {
                                g.row(*row, |r| {
                                    r.cell(|ui| {
                                        ui.label(format!("Row {row}"));
                                    })
                                });
                            }
                        });
                    moved = out.row_moved;
                    rects = out
                        .rows
                        .iter()
                        .zip(order)
                        .map(|(r, id)| (n(*id), *r))
                        .collect();
                } else {
                    let table = Table::new("t")
                        .columns([Column::remainder("a").min_width(80.0)])
                        .max_height(260.0)
                        .drag_rows(true);
                    let cell = |row: u64| {
                        move |r: &mut zaxis::GridRow<'_, '_, '_>| {
                            r.cell(|ui| {
                                ui.label(format!("Row {row}"));
                            })
                        }
                    };
                    let out: TableOutput<Range<usize>> = if virtual_rows {
                        table.show_rows(ui, 24.0, order.len(), |body, i| {
                            body.row_id(n(order[i]), cell(order[i]));
                        })
                    } else {
                        let out = table.show(ui, |body| {
                            for row in order {
                                body.row_id(n(*row), cell(*row));
                            }
                        });
                        TableOutput {
                            inner: 0..0,
                            rect: out.rect,
                            header_rect: out.header_rect,
                            viewport: out.viewport,
                            widths: out.widths,
                            offset: out.offset,
                            selected_row: out.selected_row,
                            row_clicked: out.row_clicked,
                            sort_request: out.sort_request,
                            row_moved: out.row_moved,
                            rows: out.rows,
                        }
                    };
                    moved = out.row_moved;
                    rects = out.rows;
                }
            });
        });
        self.moved = moved;
        self.rects = rects;
    }
    fn rect(&self, id: u64) -> Rect {
        self.rects.iter().find(|(x, _)| *x == n(id)).unwrap().1
    }
    fn press(&mut self, id: u64) {
        let start = self.rect(id).center();
        self.context.move_pointer(start);
        self.context.primary_button(ElementState::Pressed);
        self.context.move_pointer(start + Vec2::new(0.0, 8.0));
        self.frame();
        self.frame();
    }
}

fn drag_row(r: &mut Rows, from: u64, to: u64, fraction: f32) -> Option<RowMove> {
    r.press(from);
    let target = r.rect(to);
    r.context.move_pointer(Vec2::new(
        target.center().x,
        target.min.y + target.size().y * fraction,
    ));
    r.frame();
    r.context.primary_button(ElementState::Released);
    r.frame();
    let moved = r.moved;
    r.frame();
    assert!(r.moved.is_none(), "reported once");
    moved
}

#[test]
fn table_and_grid_rows_report_one_move_with_stable_ids() {
    for grid in [false, true] {
        let mut r = Rows::new(5, false, grid);
        let moved = drag_row(&mut r, 1, 3, 0.9);
        assert_eq!(
            moved,
            Some(RowMove {
                row: n(1),
                target: n(3),
                position: Insertion::After
            }),
            "grid {grid}"
        );
        assert_eq!(
            r.order,
            [0, 1, 2, 3, 4],
            "the library never reorders the model"
        );
        let moved = drag_row(&mut r, 3, 0, 0.1);
        assert_eq!(moved.map(|m| m.position), Some(Insertion::Before));
    }
}

#[test]
fn virtualized_table_keeps_the_source_alive_while_autoscrolling() {
    let mut r = Rows::new(200, true, false);
    r.press(1);
    let area = r.context.probe().scrolling.previous_order[0];
    let viewport = r.context.probe().scrolling.states[&area].clip;
    r.context
        .move_pointer(Vec2::new(viewport.center().x, viewport.max.y - 3.0));
    for _ in 0..90 {
        r.frame();
    }
    assert!(
        r.context.probe().scrolling.states[&area].offset.y > 300.0,
        "scrolled far enough"
    );
    assert!(
        !r.rects.iter().any(|(id, _)| *id == n(1)),
        "the source row is no longer built"
    );
    assert!(
        r.context.probe().drag.session.is_some(),
        "but the drag is alive"
    );
    // Rows built after the scroll are valid targets.
    let (target, rect) = r.rects[r.rects.len() / 2];
    r.context
        .move_pointer(Vec2::new(rect.center().x, rect.min.y + rect.size().y * 0.8));
    r.frame();
    r.context.primary_button(ElementState::Released);
    r.frame();
    assert_eq!(r.moved.map(|m| (m.row, m.target)), Some((n(1), target)));
}

#[test]
fn dragged_row_removed_from_the_model_ends_the_drag_as_source_lost() {
    let mut r = Rows::new(200, true, false);
    r.press(1);
    r.order.retain(|row| *row != 1);
    r.frame();
    r.frame();
    assert!(r.context.probe().drag.session.is_none());
    assert!(r.moved.is_none());
}

#[test]
fn reorder_helper_converts_a_drop_into_indices() {
    let ids: Vec<Id> = (0..4).map(n).collect();
    let drop = |source: u64, target: u64, insertion| zaxis::Dropped {
        source: n(source),
        target: n(target),
        position: Vec2::ZERO,
        local: Vec2::ZERO,
        insertion: Some(insertion),
        effect: zaxis::DragEffect::Move,
        payload: (),
    };
    assert_eq!(drop(0, 2, Insertion::After).reorder(&ids), Some((0, 2)));
    assert_eq!(drop(3, 1, Insertion::Before).reorder(&ids), Some((3, 1)));
    assert_eq!(drop(1, 2, Insertion::Before).reorder(&ids), None);
    assert_eq!(drop(9, 2, Insertion::Before).reorder(&ids), None);
    let mut list = vec![10, 11, 12, 13];
    zaxis::move_item(&mut list, 0, 2);
    assert_eq!(list, [11, 12, 10, 13]);
}
