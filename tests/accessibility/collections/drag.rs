//! Dragging has no request of its own: assistive technology cannot start or drop a drag.
//! What is checked here is that dragging never damages the tree. Every pass of the
//! harness feeds the update to `accesskit_consumer` and validates the mirror, so a
//! duplicated or dangling node would panic the step that produced it.
use super::*;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
struct Parcel(u32);

#[derive(Default)]
struct Seen {
    started: u32,
    dropped: Vec<u32>,
    clicks: u32,
}

/// A source that wraps a button, optionally with a preview built by the application, and
/// a target that wraps a label.
fn scene(ctx: &mut Context, preview: bool, keyboard: bool, seen: &mut Seen) {
    Root::new().padding(Padding::all(20.0)).show(ctx, |ui| {
        let mut source = DragSource::new(Id::new("source"), Parcel(7)).keyboard(keyboard);
        if preview {
            source = source.preview(|ui| {
                ui.label("Carrying a parcel");
            });
        }
        let out = source.show(ui, |ui| ui.button("Parcel"));
        seen.started += u32::from(out.started);
        seen.clicks += u32::from(out.inner.clicked());
        ui.add_space(60.0);
        let out =
            DropTarget::new(Id::new("target"), |parcel: &Parcel| parcel.0 == 7).show(ui, |ui| {
                ui.label("Shelf");
                ui.allocate_space(Vec2::new(300.0, 60.0));
            });
        seen.dropped.extend(out.dropped.map(|drop| drop.payload.0));
    });
}

/// Node ids are unique by construction of the mirror; what can go wrong is the same widget
/// described twice, which the library reports.
fn assert_sound(harness: &Harness) {
    let collisions: Vec<_> = harness
        .context
        .diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::IdCollision)
        .collect();
    assert!(collisions.is_empty(), "{collisions:?}");
}

#[test]
fn a_drag_keeps_source_and_target_content_in_the_tree_once() {
    for preview in [false, true] {
        let mut harness = Harness::new();
        let mut seen = Seen::default();
        harness.pass(|ctx| scene(ctx, preview, false, &mut seen));
        let button = harness.tree.expect(Role::Button, "Parcel");
        let shelf = harness.tree.expect(Role::Label, "Shelf");
        let (from, to) = (logical(&harness, button), logical(&harness, shelf));
        let nodes = harness.tree.len();

        harness.context.move_pointer(from.center());
        harness.context.primary_button(ElementState::Pressed);
        let step = |harness: &mut Harness, at: Vec2, seen: &mut Seen| {
            harness.context.move_pointer(at);
            harness.pass(|ctx| scene(ctx, preview, false, seen));
            assert_sound(harness);
            // The source stays where it was and what it is; nothing follows the pointer.
            assert_eq!(harness.tree.all(Role::Button), [button]);
            assert_eq!(logical(harness, button), from);
            assert_eq!(
                harness.tree.all(Role::Label),
                [shelf],
                "the preview is not in the tree"
            );
            assert_eq!(harness.tree.len(), nodes);
        };
        for i in 1..=10 {
            let at = from
                .center()
                .lerp(to.center() + Vec2::new(40.0, 20.0), i as f32 / 10.0);
            step(&mut harness, at, &mut seen);
        }
        assert_eq!(seen.started, 1, "the drag did run (preview: {preview})");
        assert!(harness.context.dragging().is_some());
        harness.context.primary_button(ElementState::Released);
        for _ in 0..4 {
            harness.pass(|ctx| scene(ctx, preview, false, &mut seen));
            assert_sound(&harness);
        }
        assert_eq!(seen.dropped, [7]);
        assert_eq!(seen.clicks, 0);
        assert_eq!(harness.tree.len(), nodes);
        assert_eq!(logical(&harness, button), from);
        // The button under the source still answers a click request.
        assert!(harness.act(button, Action::Click));
        harness.settle(|ctx| scene(ctx, preview, false, &mut seen));
        assert_eq!(seen.clicks, 1);
    }
}

#[test]
fn a_source_that_is_a_tab_stop_holds_focus_in_the_tree() {
    let mut harness = Harness::new();
    let mut seen = Seen::default();
    harness.pass(|ctx| scene(ctx, false, true, &mut seen));
    let button = harness.tree.expect(Role::Button, "Parcel");
    let source = harness.tree.parent(button).expect("the source");
    assert_eq!(harness.tree.node(source).role(), Role::GenericContainer);
    assert!(harness.tree.node(source).supports_action(Action::Focus));
    harness
        .context
        .key(KeyCode::Tab, ElementState::Pressed, false);
    harness.pass(|ctx| scene(ctx, false, true, &mut seen));
    assert_eq!(
        harness.tree.focus(),
        source,
        "focus is on the source, not lost to the window"
    );
    assert_eq!(harness.pass(|ctx| scene(ctx, false, true, &mut seen)), None);
    // Without keyboard dragging the source is not a stop and adds no node.
    let mut plain = Harness::new();
    plain.pass(|ctx| scene(ctx, false, false, &mut seen));
    let button = plain.tree.expect(Role::Button, "Parcel");
    assert_eq!(
        plain
            .tree
            .node(plain.tree.parent(button).unwrap())
            .children()
            .len(),
        2
    );
}

#[test]
fn dragging_a_list_row_keeps_every_option_in_place() {
    let mut harness = Harness::new();
    let mut order: Vec<u32> = (0..8).collect();
    let mut moved = Vec::new();
    let mut selection = HashSet::new();
    let mut pass = |harness: &mut Harness, order: &mut Vec<u32>, moved: &mut Vec<ListEvent>| {
        let names: Vec<String> = order.iter().map(|n| format!("Row {n}")).collect();
        let (rows, selection) = (&*order, &mut selection);
        let result = harness.pass(|ctx| {
            Root::new().show(ctx, |ui| {
                let out = ListBox::new("rows")
                    .accessible_label("Rows")
                    .selection(selection)
                    .drag_rows(true)
                    .revision(rows.iter().fold(0, |hash, n| hash * 31 + u64::from(*n)))
                    .row_height(24.0)
                    .show_rows(
                        ui,
                        rows.len(),
                        |i| ListEntry::item(rows[i], &names[i]),
                        |ui, row| {
                            ui.label(row.text);
                        },
                    );
                moved.extend(
                    out.events
                        .iter()
                        .copied()
                        .filter(|e| matches!(e, ListEvent::Moved { .. })),
                );
            });
        });
        // The application applies the move, as the list asks it to.
        if let Some(ListEvent::Moved {
            key,
            target,
            position,
        }) = moved.last().copied()
        {
            let from = order.iter().position(|n| Id::new(n) == key).unwrap();
            let row = order.remove(from);
            let at = order.iter().position(|n| Id::new(n) == target).unwrap();
            order.insert(at + usize::from(position == Insertion::After), row);
        }
        result
    };
    pass(&mut harness, &mut order, &mut moved);
    let names = |harness: &Harness| -> Vec<String> {
        let list = harness.tree.expect(Role::ListBox, "Rows");
        let rows = children(harness, list, Role::ListBoxOption);
        rows.iter().map(|row| harness.tree.name(*row)).collect()
    };
    let before = names(&harness);
    let row = harness.tree.expect(Role::ListBoxOption, "Row 1");
    let (from, to) = (
        logical(&harness, row).center(),
        logical(&harness, harness.tree.expect(Role::ListBoxOption, "Row 5")).center(),
    );
    harness.context.move_pointer(from);
    harness.context.primary_button(ElementState::Pressed);
    for i in 1..=8 {
        harness
            .context
            .move_pointer(from.lerp(to + Vec2::new(0.0, 8.0), i as f32 / 8.0));
        pass(&mut harness, &mut order, &mut moved);
        assert_sound(&harness);
        assert_eq!(
            names(&harness),
            before,
            "rows keep their order while one is carried"
        );
        assert_eq!(harness.tree.expect(Role::ListBoxOption, "Row 1"), row);
    }
    assert!(harness.context.dragging().is_some());
    harness.context.primary_button(ElementState::Released);
    for _ in 0..4 {
        pass(&mut harness, &mut order, &mut moved);
        assert_sound(&harness);
    }
    assert_eq!(moved.len(), 1, "one drop, one move");
    assert_eq!(order, [0, 2, 3, 4, 5, 1, 6, 7]);
    let after: Vec<String> = order.iter().map(|n| format!("Row {n}")).collect();
    assert_eq!(names(&harness), after, "the tree follows the model");
    assert_eq!(
        harness.tree.expect(Role::ListBoxOption, "Row 1"),
        row,
        "a moved row keeps its node"
    );
    let node = harness.tree.node(row);
    assert_eq!(
        (node.position_in_set(), node.size_of_set()),
        (Some(5), Some(8))
    );
}

#[test]
fn reordered_rows_stay_in_the_tree_and_follow_the_model() {
    let mut harness = Harness::new();
    let mut style = harness.context.style().clone();
    style.motion.reduced_motion = true;
    harness.context.set_style(style);
    let build = |ctx: &mut Context, order: &[u32]| {
        Root::new().show(ctx, |ui| {
            Reorder::new("tasks").show(ui, order.iter().map(Id::new), |rows| {
                for n in order {
                    rows.item(Id::new(n), |ui| {
                        ui.push_id(n, |ui| ui.button(format!("Task {n}")));
                    });
                }
            });
        });
    };
    let names = |harness: &Harness| -> Vec<String> {
        harness
            .tree
            .all(Role::Button)
            .iter()
            .map(|id| harness.tree.name(*id))
            .collect()
    };
    harness.pass(|ctx| build(ctx, &[0, 1, 2, 3]));
    let first = harness.tree.expect(Role::Button, "Task 0");
    let tops: Vec<f32> = harness
        .tree
        .all(Role::Button)
        .iter()
        .map(|id| logical(&harness, *id).min.y)
        .collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(harness.pass(|ctx| build(ctx, &[0, 1, 2, 3])), None);

    harness.settle(|ctx| build(ctx, &[1, 2, 0, 3]));
    assert_sound(&harness);
    assert_eq!(names(&harness), ["Task 1", "Task 2", "Task 0", "Task 3"]);
    assert_eq!(
        harness.tree.expect(Role::Button, "Task 0"),
        first,
        "the same node, moved"
    );
    assert!(
        (logical(&harness, first).min.y - tops[2]).abs() < 0.5,
        "bounds moved with it"
    );
    assert_eq!(harness.pass(|ctx| build(ctx, &[1, 2, 0, 3])), None);
    // A removed row leaves; the others close up.
    harness.settle(|ctx| build(ctx, &[1, 0, 3]));
    assert_eq!(names(&harness), ["Task 1", "Task 0", "Task 3"]);
    assert!((logical(&harness, first).min.y - tops[1]).abs() < 0.5);
}
