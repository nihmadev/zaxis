//! How the tree goes from one pass to the next: node ids that survive removal, repetition
//! and a modal dialog, and bounds held back while the UI moves.
use crate::support::*;
use std::time::Duration;

#[test]
fn held_bounds_are_published_once_the_motion_stops_and_then_nothing_repaints() {
    let mut harness = Harness::new();
    let start = harness.context.frame_time();
    let at = |ms: u64| start + Duration::from_millis(ms);
    let build = |ctx: &mut Context, x: f32, moving: bool| {
        if moving {
            ctx.request_repaint();
        }
        Window::new("Test").show(ctx, |ui| {
            ui.add_space(x);
            ui.button("Mover");
        });
    };
    harness.context.run_at(at(0), |ctx| build(ctx, 0.0, false));
    harness
        .tree
        .sync(&mut harness.context)
        .expect("the first tree");
    let before = harness.node(Role::Button, "Mover").bounds().unwrap();
    assert_eq!(harness.context.next_repaint(), None, "an idle tree waits");

    // Two frames of a motion: the bounds are held and a pass is scheduled for them.
    for (x, ms) in [(10.0, 16), (20.0, 32)] {
        harness.context.run_at(at(ms), |ctx| build(ctx, x, true));
        assert_eq!(
            harness.tree.sync(&mut harness.context),
            None,
            "held at {ms} ms"
        );
    }
    let wake = harness
        .context
        .next_repaint()
        .expect("a pass for the held bounds");
    assert_eq!(wake, at(216), "the hold of the first held pass");

    // The motion has stopped and nothing else redraws: the scheduled pass publishes the
    // final bounds, once.
    harness.context.run_at(wake, |ctx| build(ctx, 20.0, false));
    assert_eq!(harness.tree.sync(&mut harness.context), Some(1));
    let after = harness.node(Role::Button, "Mover").bounds().unwrap();
    assert_eq!(after.y0 - before.y0, 20.0);
    assert_eq!(
        harness.context.next_repaint(),
        None,
        "nothing left to publish"
    );
    assert!(!harness.context.needs_repaint_at(at(10_000)));
    for ms in [400, 800, 5_000] {
        harness
            .context
            .run_at(at(ms), |ctx| build(ctx, 20.0, false));
        assert_eq!(
            harness.tree.sync(&mut harness.context),
            None,
            "idle at {ms} ms"
        );
        assert_eq!(harness.context.next_repaint(), None, "idle at {ms} ms");
    }
    let stats = harness.context.accessibility_stats();
    assert_eq!((stats.updates, stats.full_updates), (2, 1));
}

#[test]
fn a_node_built_again_after_it_was_gone_gets_its_node_id_back() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context, extra: bool| {
        Window::new("Test").show(ctx, |ui| {
            ui.button("First");
            if extra {
                ui.button("Extra");
            }
            ui.button("Last");
        });
    };
    let buttons =
        |tree: &AccessTree| ["First", "Extra", "Last"].map(|n| tree.find(Role::Button, n));
    harness.pass(|ctx| build(ctx, true));
    let shown = buttons(&harness.tree);
    let [Some(first), Some(extra), Some(last)] = shown else {
        panic!("all three are in the tree: {shown:?}");
    };
    let parent = harness.tree.parent(extra).expect("a parent");

    // Gone: its parent is sent with the shorter child list, and the node itself is not.
    harness
        .pass(|ctx| build(ctx, false))
        .expect("the removal is published");
    let update = harness.tree.last.as_ref().unwrap();
    let (_, sent) = update
        .nodes
        .iter()
        .find(|(id, _)| *id == parent)
        .expect("the parent");
    assert_eq!(sent.children(), [first, last]);
    assert!(update.nodes.iter().all(|(id, _)| *id != extra));
    assert_eq!(buttons(&harness.tree), [Some(first), None, Some(last)]);

    // Back: the same node id, and the others kept theirs.
    harness
        .pass(|ctx| build(ctx, true))
        .expect("it is published again");
    assert_eq!(buttons(&harness.tree), shown);
    assert_eq!(harness.pass(|ctx| build(ctx, true)), None);
}

#[test]
fn repeated_ids_get_distinct_node_ids_that_every_pass_repeats() {
    let build = |ctx: &mut Context, on: &mut bool| {
        Window::new("Test").show(ctx, |ui| {
            ui.button("Same");
            ui.checkbox(on, "Toggle");
            ui.button("Same");
            ui.button("Same");
        });
    };
    let mut on = false;
    let mut harness = Harness::new();
    harness.pass(|ctx| build(ctx, &mut on));
    let same = harness.tree.all(Role::Button);
    assert_eq!(same.len(), 3);
    assert!(same[0] != same[1] && same[1] != same[2] && same[0] != same[2]);
    for _ in 0..3 {
        on = !on;
        // Only the checkbox is sent: the repeated buttons are the nodes they were.
        assert_eq!(harness.pass(|ctx| build(ctx, &mut on)), Some(1));
        assert_eq!(harness.tree.all(Role::Button), same);
    }
    // A tree sent again from scratch, and the tree of another context, use them too.
    harness.context.set_accessibility_active(false);
    harness.context.set_accessibility_active(true);
    harness
        .pass(|ctx| build(ctx, &mut on))
        .expect("the whole tree");
    assert!(harness.tree.last.as_ref().unwrap().tree.is_some());
    assert_eq!(harness.tree.all(Role::Button), same);
    let mut other = Harness::new();
    other.pass(|ctx| build(ctx, &mut on));
    assert_eq!(other.tree.all(Role::Button), same);
}

#[test]
fn nodes_behind_a_modal_come_back_with_their_node_ids() {
    let mut harness = Harness::new();
    let mut style = harness.context.style().clone();
    style.motion.reduced_motion = true;
    harness.context.set_style(style);
    let build = |ctx: &mut Context, open: &mut bool| {
        Window::new("Main").show(ctx, |ui| {
            ui.button("Behind");
            ui.label("Also behind");
            Modal::new("ask")
                .accessible_label("Ask")
                .show(ui, open, |ui| {
                    ui.button("Inside");
                });
        });
    };
    let mut open = false;
    harness.settle(|ctx| build(ctx, &mut open));
    let window = harness.tree.expect(Role::Window, "Main");
    let behind = harness.tree.expect(Role::Button, "Behind");
    let label = harness.tree.expect(Role::Label, "Also behind");

    open = true;
    harness.settle(|ctx| build(ctx, &mut open));
    harness.tree.expect(Role::Button, "Inside");
    for id in [window, behind, label] {
        assert!(harness.tree.get(id).is_none(), "{id:?} is behind the modal");
    }

    open = false;
    harness.settle(|ctx| build(ctx, &mut open));
    assert!(harness.tree.find(Role::Button, "Inside").is_none());
    assert_eq!(harness.tree.expect(Role::Window, "Main"), window);
    assert_eq!(harness.tree.expect(Role::Button, "Behind"), behind);
    assert_eq!(harness.tree.expect(Role::Label, "Also behind"), label);
}
