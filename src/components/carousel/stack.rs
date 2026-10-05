//! `Stack`: the front card with the following pages as sheets behind it.
use super::{
    pose::{paint_order, stack_pose, StackGeometry},
    region::{safe_content, with_region},
    scene::{bounds, Painted, Scene},
    style::Look,
    surface::paint_card,
};
use crate::{Rect, Transform, Ui, Vec2};

/// Rectangle of the front card: the area minus the room the sheets need on their side.
pub(super) fn card_rect(area: Rect, direction: Vec2, reserve: f32) -> Rect {
    let mut card = area;
    if direction.x > 0.0 {
        card.max.x -= reserve;
    } else if direction.x < 0.0 {
        card.min.x += reserve;
    }
    if direction.y > 0.0 {
        card.max.y -= reserve;
    } else if direction.y < 0.0 {
        card.min.y += reserve;
    }
    card
}

pub(super) fn geometry(area: Rect, look: &Look, axis: usize) -> StackGeometry {
    let direction = look.direction().vector();
    let layers = look.layers();
    let step = look.offset();
    StackGeometry {
        card: card_rect(area, direction, step * (layers - 1) as f32),
        direction,
        step,
        layers,
        scale_step: look.scale_step(),
        fade_step: look.fade_step(),
        exit: look.exit(axis),
        exit_rotation: look.exit_rotation(),
    }
}

/// Layers as `(k, page, distance)`, back to front.
///
/// A jump of several pages (an indicator click, `End`) must not flash the cards in
/// between. It is drawn as one step: the page we leave goes out and the page we go to
/// takes the next place in the stack; the pages in between are not drawn at all. Sheets are
/// blank, so swapping which page stands behind the front card cannot be seen.
fn layers(scene: &Scene<'_>, depth: usize) -> Vec<(i64, usize, f32)> {
    let (from, to) = (scene.from, scene.target);
    if (to - from).abs() <= 1 || scene.over != 0.0 {
        return scene.layers(0, depth, paint_order);
    }
    let step = (to - from).signum();
    let progress = ((scene.position - from as f32) / (to - from) as f32).clamp(0.0, 1.0);
    let at = from as f32 + step as f32 * progress;
    // Slot of page `k` in the one-step picture; `None` for the pages skipped over.
    let slot = |k: i64| -> Option<i64> {
        let between = if step > 0 {
            k > from && k < to
        } else {
            k < from && k > to
        };
        let beyond = if step > 0 { k > to } else { k < to };
        if between {
            None
        } else if k == to {
            Some(from + step)
        } else if beyond {
            Some(from + step + (k - to))
        } else {
            Some(k)
        }
    };
    let first = from.min(to) - 1;
    let last = from.max(to) + depth as i64;
    let mut layers: Vec<_> = (first..=last)
        .filter_map(|k| Some((k, scene.page(k)?, slot(k)? as f32 - at)))
        .collect();
    if scene.wrap {
        // One layer per page: keep the nearest of any repeats.
        layers.sort_by(|a, b| a.2.abs().total_cmp(&b.2.abs()));
        let mut seen = std::collections::HashSet::new();
        layers.retain(|layer| seen.insert(layer.1));
    }
    layers.sort_by(|a, b| paint_order(a.2, b.2));
    layers
}

pub(super) fn paint(
    ui: &mut Ui<'_>,
    scene: &Scene<'_>,
    build: &mut dyn FnMut(&mut Ui<'_>, usize),
) -> Painted {
    let g = geometry(scene.area, scene.look, scene.axis);
    let extent = g.card.size()[scene.axis] * scene.look.travel();
    let mut painted = Painted {
        layers: Vec::new(),
        front: g.card,
    };
    for (k, page, d) in layers(scene, g.layers) {
        let Some(pose) = stack_pose(&g, d) else {
            continue;
        };
        let transform = scene.overscroll(k, pose.transform, extent);
        let key = scene.model.key(page);
        let scope = scene.id.with(("page", key));
        let front = scene.interactive(k);
        let card = g.card;
        with_region(ui, scope, card, scene.stage, scene.enabled, |layer| {
            layer.visual_gated("layer", transform, pose.opacity, Some(front), |inner| {
                let current = k == scene.target;
                let node = super::access::page(inner, scope, page, scene.model.count, current);
                paint_card(inner, scope.with("card"), card, scene.look, d);
                if pose.content > 0.01 {
                    let content =
                        safe_content(card, scene.look.content_padding(), scene.look.rounding());
                    let mut run = |ui: &mut Ui<'_>| {
                        with_region(
                            ui,
                            scope.with("content"),
                            content,
                            content,
                            true,
                            |page_ui| build(page_ui, page),
                        )
                    };
                    if pose.content < 0.995 {
                        inner.visual_gated(
                            "fade",
                            Transform::IDENTITY,
                            pose.content,
                            Some(front),
                            run,
                        );
                    } else {
                        run(inner);
                    }
                }
                inner.a11y_end(node, Some(card));
                // The layer occupies its card: visibility and clipping are judged on this size.
                inner.allocate_space(card.size());
            });
        });
        let rect = bounds(transform, card);
        if k == scene.target {
            painted.front = rect;
        }
        painted.layers.push((k, rect));
    }
    painted
}
