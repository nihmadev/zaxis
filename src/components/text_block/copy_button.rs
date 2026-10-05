//! The small copy button beside a copyable label: a Lucide-style two-sheet icon that turns
//! into a check mark for a moment after it copied. It is a pointer-only target, so it adds
//! no extra stop to the Tab cycle of a label that already has focus.

use super::state::BlockState;
use crate::{
    components::{Sense, Ui},
    Color, Id, Rect, Shape, Vec2,
};
use std::time::Duration;

/// Side of the icon box and its distance from the text, in logical pixels.
pub(crate) const SIZE: f32 = 16.0;
pub(crate) const GAP: f32 = 6.0;
const FLASH: Duration = Duration::from_millis(1400);

fn stroke(shapes: &mut Vec<Shape>, points: &[Vec2], color: Color, width: f32) {
    for pair in points.windows(2) {
        shapes.push(Shape::Line {
            start: pair[0],
            end: pair[1],
            width,
            color,
        });
    }
}

/// A quarter turn from `from` to `to` around `center`, as short segments.
fn arc(center: Vec2, radius: f32, from: f32, to: f32) -> Vec<Vec2> {
    (0..=4)
        .map(|i| {
            let t = from + (to - from) * i as f32 / 4.0;
            center + Vec2::new(t.cos(), t.sin()) * radius
        })
        .collect()
}

fn copy_icon(rect: Rect, color: Color) -> Vec<Shape> {
    // Lucide `copy` on a 24 unit grid: a front sheet and the edge of the one behind it.
    let k = rect.size().x / 24.0;
    let p = |x: f32, y: f32| rect.min + Vec2::new(x, y) * k;
    let (w, pi) = (2.0 * k.max(0.5), std::f32::consts::PI);
    let mut shapes = vec![Shape::Rect {
        rect: Rect::from_min_max(p(8.0, 8.0), p(22.0, 22.0)),
        fill: Color::TRANSPARENT,
        rounding: crate::CornerRadius::all(2.0 * k),
        border: crate::Border::new(w, color),
    }];
    let mut back = vec![p(4.0, 16.0)];
    back.extend(arc(p(4.0, 14.0), 2.0 * k, pi * 0.5, pi));
    back.push(p(2.0, 4.0));
    back.extend(arc(p(4.0, 4.0), 2.0 * k, pi, pi * 1.5));
    back.push(p(14.0, 2.0));
    back.extend(arc(p(14.0, 4.0), 2.0 * k, pi * 1.5, pi * 2.0));
    back.push(p(16.0, 8.0));
    stroke(&mut shapes, &back, color, w);
    shapes
}

fn check_icon(rect: Rect, color: Color) -> Vec<Shape> {
    let k = rect.size().x / 24.0;
    let p = |x: f32, y: f32| rect.min + Vec2::new(x, y) * k;
    let mut shapes = Vec::new();
    stroke(
        &mut shapes,
        &[p(20.0, 6.0), p(9.0, 17.0), p(4.0, 12.0)],
        color,
        2.0 * k.max(0.5),
    );
    shapes
}

/// Draw the button to the right of `text` and copy the whole source text when it is
/// clicked. Returns whether it copied during this pass.
pub(crate) fn show(ui: &mut Ui<'_>, state: &mut BlockState, id: Id, text: Rect) -> bool {
    let top = text.min.y + (text.size().y - SIZE) * 0.5;
    let rect = Rect::from_min_size(Vec2::new(text.max.x + GAP, top), Vec2::splat(SIZE));
    let hit = Rect::from_min_max(rect.min - Vec2::splat(4.0), rect.max + Vec2::splat(4.0));
    let response = ui.interact(hit, (id, "copy-button"), Sense::CLICK);
    ui.a11y(response.id, hit, crate::AccessRole::Button, |node| {
        node.label("Copy").clicks(response.id);
    });
    // A press on the button must not clear the selection of the text beside it.
    let scope = state.scope;
    ui.context.selection.owners.insert(response.id, (id, scope));
    let mut copied = false;
    if response.clicked() {
        let all = state.text.clone();
        copied = ui.context.copy_text(all).is_ok();
        if copied {
            state.copied = Some(ui.context.frame_time());
        }
    }
    let now = ui.context.frame_time();
    let recent = state
        .copied
        .and_then(|at| FLASH.checked_sub(now.saturating_duration_since(at)));
    match recent {
        Some(left) => ui.context.request_repaint_after(left),
        None => state.copied = None,
    }
    let style = ui.style();
    let color = if !response.enabled {
        style.disabled_text
    } else if recent.is_some() {
        style.success
    } else if response.hovered {
        style.text_color
    } else {
        style.muted_text
    };
    let shapes = if recent.is_some() {
        check_icon(rect, color)
    } else {
        copy_icon(rect, color)
    };
    let paints = shapes
        .into_iter()
        .map(crate::context::Paint::Shape)
        .collect();
    let (window, clip) = (ui.window, ui.clip);
    ui.context.paint(id.with("copy-icon"), window, clip, paints);
    copied
}
