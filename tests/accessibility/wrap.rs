//! A wrap adds no node of its own: its items describe themselves, in declaration order,
//! with the bounds they are shown at.
use crate::support::*;

const NAMES: [&str; 6] = ["One", "Two", "Three", "Four", "Five", "Six"];

fn wrapped(ctx: &mut Context, width: f32) -> WrapOutput<()> {
    let mut out = None;
    Root::new().padding(Padding::all(0.0)).show(ctx, |ui| {
        out = Some(Wrap::new("tags").width(width).gap(6.0).show(ui, |ui| {
            for name in NAMES {
                ui.button(name);
            }
        }));
    });
    out.unwrap()
}

fn logical(harness: &Harness, id: NodeId) -> Rect {
    let scale = harness.context.scale_factor() as f64;
    let b = harness.tree.node(id).bounds().expect("bounds");
    Rect::from_min_max(
        Vec2::new((b.x0 / scale) as f32, (b.y0 / scale) as f32),
        Vec2::new((b.x1 / scale) as f32, (b.y1 / scale) as f32),
    )
}

#[test]
fn nodes_are_at_the_displayed_positions_in_order_and_at_every_scale() {
    for scale in [1.0, 1.5, 2.0] {
        let mut harness = Harness::with_scale(scale);
        let mut out = None;
        harness.settle(|ctx| out = Some(wrapped(ctx, 130.0)));
        let out = out.unwrap();
        assert!(out.lines.len() > 1, "the buttons wrap in 130 px");
        for (name, item) in NAMES.iter().zip(&out.items) {
            let node = harness.tree.expect(Role::Button, name);
            let bounds = logical(&harness, node);
            assert!(
                // Bounds are snapped to whole physical pixels.
                (bounds.min - item.min).abs().max_element() < 1.0
                    && (bounds.max - item.max).abs().max_element() < 1.0,
                "{name} at scale {scale}: {bounds:?} vs {item:?}"
            );
        }
        let names: Vec<_> = harness
            .tree
            .all(Role::Button)
            .into_iter()
            .map(|id| harness.tree.name(id))
            .collect();
        assert_eq!(names, NAMES);
    }
}

#[test]
fn a_wrap_costs_no_nodes_compared_with_a_row() {
    let mut row = Harness::new();
    let rows = row.pass(|ctx| {
        Root::new().padding(Padding::all(0.0)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                for name in NAMES {
                    ui.button(name);
                }
            });
        })
    });
    let mut wrap = Harness::new();
    let wraps = wrap.pass(|ctx| drop(wrapped(ctx, 130.0)));
    assert_eq!(wraps, rows);
}

#[test]
fn a_reflow_moves_nodes_without_adding_or_removing_any() {
    let mut harness = Harness::new();
    harness.settle(|ctx| drop(wrapped(ctx, 600.0)));
    let wide = harness.tree.len();
    let before = logical(&harness, harness.tree.expect(Role::Button, "Six"));
    harness.settle(|ctx| drop(wrapped(ctx, 100.0)));
    let after = logical(&harness, harness.tree.expect(Role::Button, "Six"));
    assert_eq!(harness.tree.len(), wide);
    assert!(after.min.y > before.min.y && after.min.x < before.min.x);
}
