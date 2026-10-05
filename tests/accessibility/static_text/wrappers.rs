//! Wrappers that restyle, move or animate what is built inside them: the nodes keep their
//! parents, their bounds follow the visual transform, and a settled animation is silent.

use super::*;

fn window(context: &mut Context, build: impl FnOnce(&mut Ui<'_>)) {
    Window::new("Stage").show(context, build);
}

fn stage(harness: &Harness) -> NodeId {
    harness.tree.expect(Role::Window, "Stage")
}

fn button(harness: &Harness, name: &str) -> accesskit::Rect {
    bounds(harness.node(Role::Button, name))
}

const SECOND: Duration = Duration::from_millis(1000);

#[test]
fn a_hover_wrapper_keeps_the_widget_node_and_its_click() {
    let mut harness = Harness::new();
    let clicks = Cell::new(0);
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            let style = HoverStyle::fill(Color::rgb(40, 90, 160));
            count(
                &clicks,
                ui.add(Hover::new(Button::new("Save")).style(style))
                    .clicked(),
            );
            ui.add(Hover::new(Button::new("Raw")).accessible_label("Export"));
        });
    };
    harness.pass(build);
    let save = harness.tree.expect(Role::Button, "Save");
    assert_eq!(harness.tree.parent(save), Some(stage(&harness)));
    assert!(
        harness.tree.find(Role::Button, "Export").is_some(),
        "the override reaches inside"
    );
    assert!(harness.act(save, Action::Click));
    harness.settle(build);
    assert_eq!(clicks.get(), 1);
    assert_eq!(harness.pass(build), None);
}

#[test]
fn rows_and_nested_layouts_keep_parents_and_bounds() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.button("Left");
                ui.spacer();
                ui.vertical(|ui| {
                    ui.button("Upper");
                    ui.button("Lower");
                });
            });
        });
    };
    harness.settle(build);
    let parent = stage(&harness);
    for name in ["Left", "Upper", "Lower"] {
        let id = harness.tree.expect(Role::Button, name);
        assert_eq!(
            harness.tree.parent(id),
            Some(parent),
            "layout scopes add no nodes"
        );
    }
    let outer = bounds(harness.tree.node(parent));
    let (left, upper, lower) = (
        button(&harness, "Left"),
        button(&harness, "Upper"),
        button(&harness, "Lower"),
    );
    assert!(
        upper.x0 > left.x1,
        "the spacer pushed the column right: {left:?} {upper:?}"
    );
    assert!(upper.x1 <= outer.x1 && lower.y0 >= upper.y1 && lower.x0 == upper.x0);
    assert_eq!(harness.pass(build), None);
}

#[test]
fn bounds_follow_a_visual_transform() {
    for scale in [1.0, 1.5] {
        let mut harness = Harness::with_scale(scale);
        let build = |ctx: &mut Context, transform: Transform, opacity: f32| {
            window(ctx, |ui| {
                ui.label("Top");
                ui.visual("moved", transform, opacity, |ui| {
                    ui.button("Moved");
                });
            });
        };
        let start = Instant::now();
        pass_at(&mut harness, start, |ctx| {
            build(ctx, Transform::IDENTITY, 1.0)
        });
        let id = harness.tree.expect(Role::Button, "Moved");
        assert_eq!(harness.tree.parent(id), Some(stage(&harness)));
        let base = button(&harness, "Moved");

        let shift = Transform::translation(Vec2::new(40.0, 10.0));
        let (now, _) = advance(&mut harness, start, SECOND, |ctx| build(ctx, shift, 1.0));
        let moved = button(&harness, "Moved");
        assert!(
            (moved.x0 - base.x0 - 40.0 * scale).abs() <= 1.0,
            "{scale}: {base:?} {moved:?}"
        );
        assert!((moved.y0 - base.y0 - 10.0 * scale).abs() <= 1.0);
        assert!(
            (moved.x1 - moved.x0 - (base.x1 - base.x0)).abs() <= 1.0,
            "the size is kept"
        );
        let idle = advance(&mut harness, now, SECOND / 2, |ctx| build(ctx, shift, 1.0));
        assert_eq!(idle.1, 0);

        let pivot = Vec2::new(base.x0 as f32, base.y0 as f32) / scale as f32;
        let double = Transform::around(pivot, 2.0, Vec2::ZERO);
        let (now, _) = advance(&mut harness, idle.0, SECOND, |ctx| build(ctx, double, 1.0));
        let grown = button(&harness, "Moved");
        assert!((grown.x0 - base.x0).abs() <= 1.0 && (grown.y0 - base.y0).abs() <= 1.0);
        assert!(
            (grown.x1 - grown.x0 - 2.0 * (base.x1 - base.x0)).abs() <= 2.0,
            "{grown:?}"
        );
        assert!((grown.y1 - grown.y0 - 2.0 * (base.y1 - base.y0)).abs() <= 2.0);
        assert!(!harness.tree.node(id).is_hidden());

        // Fully transparent content takes no input and is hidden from the tree's readers.
        advance(&mut harness, now, SECOND, |ctx| {
            build(ctx, Transform::IDENTITY, 0.0)
        });
        assert!(harness.tree.node(id).is_hidden());
        assert!(!harness.act(id, Action::Click));
    }
}

#[test]
fn a_presence_publishes_where_it_settles_and_hides_while_it_leaves() {
    let mut harness = Harness::new();
    let clicks = Cell::new(0);
    let build = |ctx: &mut Context, visible: bool| {
        window(ctx, |ui| {
            ui.button("Above");
            Presence::slide(Vec2::new(0.0, 30.0)).show(ui, "panel", visible, |ui| {
                count(&clicks, ui.button("Inside").clicked());
            });
            ui.button("Below");
        });
    };
    let start = Instant::now();
    pass_at(&mut harness, start, |ctx| build(ctx, true));
    let inside = harness.tree.expect(Role::Button, "Inside");
    assert_eq!(harness.tree.parent(inside), Some(stage(&harness)));
    let (now, _) = advance(&mut harness, start, SECOND, |ctx| build(ctx, true));
    let (above, shown, below) = (
        button(&harness, "Above"),
        button(&harness, "Inside"),
        button(&harness, "Below"),
    );
    assert!(
        shown.y0 >= above.y1 && shown.y1 <= below.y0,
        "settled between its neighbours"
    );
    assert!(!harness.tree.node(inside).is_hidden());
    let (now, updates) = advance(&mut harness, now, SECOND / 2, |ctx| build(ctx, true));
    assert_eq!(updates, 0, "a settled presence is silent");
    assert!(harness.act(inside, Action::Click));
    let (now, _) = advance(&mut harness, now, SECOND / 10, |ctx| build(ctx, true));
    assert_eq!(clicks.get(), 1);

    // Leaving: still drawn, so still readable, but inert for requests as for the pointer.
    pass_at(&mut harness, now, |ctx| build(ctx, false));
    let leaving = harness.tree.node(inside);
    assert!(!leaving.is_hidden() && leaving.is_disabled(), "{leaving:?}");
    assert!(!harness.act(inside, Action::Click));
    let (now, _) = advance(&mut harness, now, SECOND, |ctx| build(ctx, false));
    assert!(
        harness.tree.find(Role::Button, "Inside").is_none(),
        "gone when the exit ends"
    );
    assert_eq!(
        advance(&mut harness, now, SECOND / 2, |ctx| build(ctx, false)).1,
        0
    );
    assert_eq!(clicks.get(), 1);
}

#[test]
fn a_reveal_shows_its_content_when_open_and_drops_it_when_closed() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context, open: bool| {
        window(ctx, |ui| {
            ui.button("Header");
            ui.reveal("details", open, |ui| {
                ui.button("Detail");
                ui.label("More text");
            });
            ui.button("Footer");
        });
    };
    let start = Instant::now();
    pass_at(&mut harness, start, |ctx| build(ctx, true));
    let (now, _) = advance(&mut harness, start, SECOND, |ctx| build(ctx, true));
    let detail = harness.tree.expect(Role::Button, "Detail");
    assert_eq!(harness.tree.parent(detail), Some(stage(&harness)));
    assert!(!harness.tree.node(detail).is_hidden());
    let (header, shown, footer) = (
        button(&harness, "Header"),
        button(&harness, "Detail"),
        button(&harness, "Footer"),
    );
    assert!(
        shown.y0 >= header.y1 && footer.y0 >= shown.y1,
        "the footer moved below"
    );
    let (now, updates) = advance(&mut harness, now, SECOND / 2, |ctx| build(ctx, true));
    assert_eq!(updates, 0);

    pass_at(&mut harness, now, |ctx| build(ctx, false));
    let closing = harness.tree.node(detail);
    assert!(!closing.is_hidden() && closing.is_disabled(), "{closing:?}");
    let (now, _) = advance(&mut harness, now, SECOND, |ctx| build(ctx, false));
    assert!(harness.tree.find(Role::Button, "Detail").is_none());
    let closed = button(&harness, "Footer");
    assert!(closed.y0 < footer.y0, "the footer came back up: {closed:?}");
    assert_eq!(
        advance(&mut harness, now, SECOND / 2, |ctx| build(ctx, false)).1,
        0
    );
}

#[test]
fn a_moving_element_ends_where_it_is_built() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context, gap: f32| {
        window(ctx, |ui| {
            ui.add_space(gap);
            ui.shared("card", |ui| {
                ui.button("Card");
            });
        });
    };
    let start = Instant::now();
    pass_at(&mut harness, start, |ctx| build(ctx, 0.0));
    let (now, _) = advance(&mut harness, start, SECOND / 2, |ctx| build(ctx, 0.0));
    let card = harness.tree.expect(Role::Button, "Card");
    assert_eq!(harness.tree.parent(card), Some(stage(&harness)));
    let before = button(&harness, "Card");
    let (now, updates) = advance(&mut harness, now, SECOND * 2, |ctx| build(ctx, 120.0));
    let after = button(&harness, "Card");
    assert!(
        (after.y0 - before.y0 - 120.0).abs() <= 1.0,
        "{before:?} -> {after:?}"
    );
    assert_eq!(after.x0, before.x0);
    assert!(
        (1..=12).contains(&updates),
        "the glide is published in a few steps: {updates}"
    );
    assert_eq!(
        advance(&mut harness, now, SECOND / 2, |ctx| build(ctx, 120.0)).1,
        0
    );
}

#[test]
fn a_pulsing_subtree_never_touches_the_tree() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.pulse("live", true, |ui| {
                ui.label("Recording");
            });
        });
    };
    let start = Instant::now();
    pass_at(&mut harness, start, build);
    let label = harness.tree.expect(Role::Label, "Recording");
    assert_eq!(harness.tree.parent(label), Some(stage(&harness)));
    assert!(harness.context.wants_animation_frame(), "the pulse runs");
    let (_, updates) = advance(&mut harness, start, SECOND * 2, build);
    assert_eq!(updates, 0);
    assert!(!harness.tree.node(label).is_hidden());
}
