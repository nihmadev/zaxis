use super::*;
use zaxis::accesskit::{Point, ScrollUnit};

const VIEW: f32 = 200.0;

/// A window with one vertical scroll area of forty buttons.
fn area(ctx: &mut Context, clicks: &mut Vec<usize>) {
    Root::new().show(ctx, |ui| {
        ScrollArea::vertical()
            .id_source("area")
            .max_height(VIEW)
            .show(ui, |ui| {
                for i in 0..40 {
                    if ui.button(format!("Item {i}")).clicked() {
                        clicks.push(i);
                    }
                }
            });
    });
}

fn view(harness: &Harness) -> NodeId {
    harness.tree.all(Role::ScrollView)[0]
}

fn offset(harness: &Harness) -> (f64, f64) {
    let node = harness.tree.node(view(harness));
    (
        node.scroll_y().expect("scroll_y"),
        node.scroll_y_max().expect("scroll_y_max"),
    )
}

#[test]
fn a_scroll_area_is_a_scroll_view_that_holds_its_content() {
    let mut harness = Harness::new();
    let mut clicks = Vec::new();
    harness.pass(|ctx| area(ctx, &mut clicks));
    let view = view(&harness);
    let node = harness.tree.node(view);
    assert!(node.clips_children());
    assert_eq!(node.scroll_y(), Some(0.0));
    assert_eq!(node.scroll_y_min(), Some(0.0));
    assert!(
        node.scroll_y_max().unwrap() > 500.0,
        "{:?}",
        node.scroll_y_max()
    );
    assert_eq!(node.scroll_x_max(), Some(0.0));
    assert!(node.supports_action(Action::ScrollDown));
    assert!(
        !node.supports_action(Action::ScrollUp),
        "already at the top"
    );
    assert!(!node.supports_action(Action::ScrollRight));
    let bounds = logical(&harness, view);
    assert!(
        (bounds.size().y - VIEW).abs() <= 6.0,
        "the viewport, not the content: {bounds:?}"
    );
    // Every button is a child, also the ones scrolled out of sight.
    assert_eq!(children(&harness, view, Role::Button).len(), 40);
    let far = harness.tree.expect(Role::Button, "Item 39");
    assert_eq!(harness.tree.parent(far), Some(view));
    assert!(harness
        .tree
        .node(far)
        .supports_action(Action::ScrollIntoView));
    assert_eq!(harness.pass(|ctx| area(ctx, &mut clicks)), None, "idle");
}

#[test]
fn scroll_requests_move_by_items_and_pages_and_stop_at_the_ends() {
    let mut harness = Harness::new();
    let mut clicks = Vec::new();
    harness.pass(|ctx| area(ctx, &mut clicks));
    let view = view(&harness);
    let height = f64::from(logical(&harness, view).size().y);
    let max = offset(&harness).1;

    assert!(harness.act_with(
        view,
        Action::ScrollDown,
        ActionData::ScrollUnit(ScrollUnit::Item)
    ));
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert_eq!(offset(&harness).0, 48.0, "one item is three lines of text");
    assert!(
        harness.act(view, Action::ScrollDown),
        "no unit means an item"
    );
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert_eq!(offset(&harness).0, 96.0);
    assert!(harness.act_with(
        view,
        Action::ScrollDown,
        ActionData::ScrollUnit(ScrollUnit::Page)
    ));
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert!(
        (offset(&harness).0 - (96.0 + height)).abs() < 0.01,
        "{:?}",
        offset(&harness)
    );
    assert!(harness.act_with(
        view,
        Action::ScrollUp,
        ActionData::ScrollUnit(ScrollUnit::Page)
    ));
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert!((offset(&harness).0 - 96.0).abs() < 0.01);
    assert!(harness.tree.node(view).supports_action(Action::ScrollUp));

    for _ in 0..40 {
        harness.act_with(
            view,
            Action::ScrollDown,
            ActionData::ScrollUnit(ScrollUnit::Page),
        );
        harness.pass(|ctx| area(ctx, &mut clicks));
    }
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert_eq!(offset(&harness).0, max, "clamped at the end");
    assert!(!harness.tree.node(view).supports_action(Action::ScrollDown));
    for _ in 0..40 {
        harness.act_with(
            view,
            Action::ScrollUp,
            ActionData::ScrollUnit(ScrollUnit::Page),
        );
        harness.pass(|ctx| area(ctx, &mut clicks));
    }
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert_eq!(offset(&harness).0, 0.0, "clamped at the start");
    // The horizontal axis does not scroll: nothing moves, nothing is published.
    harness.act(view, Action::ScrollRight);
    assert_eq!(harness.pass(|ctx| area(ctx, &mut clicks)), None);
    assert!(clicks.is_empty());
}

#[test]
fn a_scroll_offset_is_set_in_physical_pixels() {
    for scale in [1.0, 2.0] {
        let mut harness = Harness::with_scale(scale);
        let mut clicks = Vec::new();
        harness.pass(|ctx| area(ctx, &mut clicks));
        let view = view(&harness);
        let point = |y: f64| ActionData::SetScrollOffset(Point::new(0.0, y));
        assert!(harness.act_with(view, Action::SetScrollOffset, point(300.0 * scale)));
        harness.settle(|ctx| area(ctx, &mut clicks));
        assert_eq!(offset(&harness).0, 300.0 * scale, "scale {scale}");
        let max = offset(&harness).1;
        harness.act_with(view, Action::SetScrollOffset, point(1.0e9));
        harness.settle(|ctx| area(ctx, &mut clicks));
        assert_eq!(offset(&harness).0, max);
        harness.act_with(view, Action::SetScrollOffset, point(-50.0));
        harness.settle(|ctx| area(ctx, &mut clicks));
        assert_eq!(offset(&harness).0, 0.0);
        for bad in [f64::NAN, f64::INFINITY] {
            harness.act_with(view, Action::SetScrollOffset, point(bad));
            harness.settle(|ctx| area(ctx, &mut clicks));
            assert_eq!(offset(&harness).0, 0.0);
        }
    }
}

#[test]
fn scroll_into_view_brings_a_far_button_into_the_viewport() {
    let mut harness = Harness::new();
    let mut clicks = Vec::new();
    harness.pass(|ctx| area(ctx, &mut clicks));
    let view = view(&harness);
    let far = harness.tree.expect(Role::Button, "Item 30");
    let inside = |harness: &Harness| {
        let (button, port) = (logical(harness, far), logical(harness, view));
        button.min.y >= port.min.y - 0.5 && button.max.y <= port.max.y + 0.5
    };
    assert!(!inside(&harness));
    assert!(harness.act(far, Action::ScrollIntoView));
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert!(inside(&harness), "{:?}", logical(&harness, far));
    let after = offset(&harness).0;
    // Already visible: nothing moves.
    assert!(harness.act(far, Action::ScrollIntoView));
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert_eq!(offset(&harness).0, after);
    // And the revealed button is the one a click reaches.
    assert!(harness.act(far, Action::Click));
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert_eq!(clicks, [30]);
    let first = harness.tree.expect(Role::Button, "Item 0");
    assert!(harness.act(first, Action::ScrollIntoView));
    harness.settle(|ctx| area(ctx, &mut clicks));
    assert_eq!(offset(&harness).0, 0.0);
}

#[test]
fn a_wheel_glide_publishes_the_target_offset_once() {
    let mut harness = Harness::new();
    let mut clicks = Vec::new();
    harness.pass(|ctx| area(ctx, &mut clicks));
    let at = logical(&harness, view(&harness)).center();
    harness.context.move_pointer(at);
    harness.pass(|ctx| area(ctx, &mut clicks));
    assert!(harness.context.scroll_wheel(Vec2::new(0.0, 120.0)));
    harness.pass(|ctx| area(ctx, &mut clicks));
    // The drawn offset eases toward the target for a few frames; the published one is
    // already there, so the view itself is sent once for the whole glide.
    assert_eq!(offset(&harness).0, 120.0);
    let view_id = view(&harness);
    for _ in 0..3 {
        if harness.pass(|ctx| area(ctx, &mut clicks)).is_some() {
            let update = harness.tree.last.as_ref().unwrap();
            assert!(
                update.nodes.iter().all(|(id, _)| *id != view_id),
                "the view was resent"
            );
        }
    }
    assert_eq!(offset(&harness).0, 120.0);
}

/// An outer area that holds a few buttons and an inner area of its own.
fn nested(ctx: &mut Context) {
    Root::new().show(ctx, |ui| {
        ScrollArea::vertical()
            .id_source("outer")
            .max_height(240.0)
            .show(ui, |ui| {
                for i in 0..6 {
                    ui.button(format!("Outer {i}"));
                }
                ScrollArea::vertical()
                    .id_source("inner")
                    .max_height(120.0)
                    .show(ui, |ui| {
                        for i in 0..30 {
                            ui.button(format!("Inner {i}"));
                        }
                    });
                for i in 6..30 {
                    ui.button(format!("Outer {i}"));
                }
            });
    });
}

#[test]
fn nested_scroll_areas_nest_and_scroll_on_their_own() {
    let mut harness = Harness::new();
    harness.pass(nested);
    let views = harness.tree.all(Role::ScrollView);
    assert_eq!(views.len(), 2);
    let (outer, inner) = (views[0], views[1]);
    assert_eq!(harness.tree.parent(inner), Some(outer));
    assert_eq!(children(&harness, inner, Role::Button).len(), 30);
    assert_eq!(children(&harness, outer, Role::Button).len(), 30);
    let y = |harness: &Harness, id: NodeId| harness.tree.node(id).scroll_y().unwrap();

    assert!(harness.act(inner, Action::ScrollDown));
    harness.settle(nested);
    assert_eq!((y(&harness, outer), y(&harness, inner)), (0.0, 48.0));
    // Revealing a row of the inner area moves the inner area only.
    let row = harness.tree.expect(Role::Button, "Inner 20");
    assert!(harness.act(row, Action::ScrollIntoView));
    harness.settle(nested);
    assert_eq!(y(&harness, outer), 0.0);
    let (button, port) = (logical(&harness, row), logical(&harness, inner));
    assert!(button.min.y >= port.min.y - 0.5 && button.max.y <= port.max.y + 0.5);
    // A row of the outer area below the fold moves the outer one.
    let low = harness.tree.expect(Role::Button, "Outer 25");
    let before = y(&harness, inner);
    assert!(harness.act(low, Action::ScrollIntoView));
    harness.settle(nested);
    assert!(y(&harness, outer) > 0.0);
    assert_eq!(y(&harness, inner), before);
    assert_eq!(harness.pass(nested), None);
}

#[test]
fn a_disabled_scroll_area_refuses_requests() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        Root::new().show(ctx, |ui| {
            ui.add_enabled_ui(false, |ui| {
                ScrollArea::vertical().max_height(VIEW).show(ui, |ui| {
                    for i in 0..40 {
                        ui.button(format!("Item {i}"));
                    }
                });
            });
        });
    };
    harness.pass(build);
    let view = view(&harness);
    assert!(harness.tree.node(view).is_disabled());
    assert!(!harness.act(view, Action::ScrollDown));
    harness.settle(build);
    assert_eq!(harness.tree.node(view).scroll_y(), Some(0.0));
}
