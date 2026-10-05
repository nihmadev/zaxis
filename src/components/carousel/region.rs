//! Child UIs pinned to a rectangle, for content that is drawn at a place of the carousel's
//! choosing instead of at the layout cursor.
use crate::{components::Ui, layout::LayoutCursor, Align, Id, Layout, Padding, Rect, Vec2};

/// Run `build` in a vertical layout that starts at `rect.min`, with paint and hits clipped
/// to `clip`. The UI is disabled when `enabled` is false: its controls show but do not act.
pub(super) fn with_region<R>(
    ui: &mut Ui<'_>,
    scope: Id,
    rect: Rect,
    clip: Rect,
    enabled: bool,
    build: impl FnOnce(&mut Ui<'_>) -> R,
) -> R {
    let spacing = ui.style().spacing.max(0.0);
    let mut child = Ui {
        context: &mut *ui.context,
        window: ui.window,
        scope,
        sequence: 0,
        clip,
        layout: LayoutCursor::new(rect, Layout::Vertical, spacing),
        enabled: ui.enabled && enabled,
        backdrop_blur: ui.backdrop_blur,
        hover_style: ui.hover_style,
        flow: None,
        local_style: ui.local_style.clone(),
        local_style_revision: ui.local_style_revision,
    };
    child.begin_layout(Align::Start);
    let result = build(&mut child);
    child.finish_layout();
    result
}

/// The part of `bounds` that content may use. It is inset by `padding` and, on each side,
/// by at least the sagitta of the corner arc: a rectangle that far in from a rounded
/// outline has its corners on the arc, so clipping to it can never show content outside
/// the rounded shape. The renderer clips to rectangles only; this is the honest substitute
/// for a rounded clip (the corner slivers between the rectangle and the arc stay empty).
pub(super) fn safe_content(bounds: Rect, padding: Padding, rounding: f32) -> Rect {
    let arc = rounding.max(0.0) * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
    let rect = Padding {
        left: padding.left.max(arc),
        right: padding.right.max(arc),
        top: padding.top.max(arc),
        bottom: padding.bottom.max(arc),
    }
    .inset(bounds);
    if rect.size().min_element() <= 0.0 {
        Rect::from_min_size(bounds.center(), Vec2::ZERO)
    } else {
        rect
    }
}
