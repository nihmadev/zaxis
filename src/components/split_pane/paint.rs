use super::{allocation::dimension, SplitHandle, SplitHandleStyle, SplitSurface, Ui};
use crate::{context::Paint, CornerRadius, Id, Layout, Padding, Rect, Shape, Vec2};

pub(super) fn validate_surface(s: SplitSurface) {
    for n in [
        s.padding.left,
        s.padding.right,
        s.padding.top,
        s.padding.bottom,
        s.border.width,
        s.rounding.top_left,
        s.rounding.top_right,
        s.rounding.bottom_left,
        s.rounding.bottom_right,
        s.blur,
    ] {
        dimension(n);
    }
    if let Some(shadow) = s.shadow {
        dimension(shadow.blur_radius);
        assert!(
            shadow.offset.is_finite() && shadow.spread.is_finite(),
            "split shadow must be finite"
        );
    }
}
pub(super) fn validate_handle(h: SplitHandleStyle) {
    for n in [
        h.hit_width,
        h.thickness,
        h.length,
        h.inset,
        h.rounding.top_left,
        h.rounding.top_right,
        h.rounding.bottom_left,
        h.rounding.bottom_right,
    ] {
        dimension(n);
    }
}
/// Inscribed rectangle: every corner is inside the normalized rounded shape.
/// Requested insets collapse proportionally to an empty rectangle on tiny panels.
/// This is intentionally a rectangular content clip, not a curved stencil.
pub(super) fn content_bounds(rect: Rect, s: SplitSurface) -> Rect {
    let [tl, tr, br, bl] = s.rounding.values(rect);
    let inset = |r: f32| r * (1.0 - std::f32::consts::FRAC_1_SQRT_2) + s.border.width;
    let p = Padding {
        left: s.padding.left.max(inset(tl.max(bl))),
        right: s.padding.right.max(inset(tr.max(br))),
        top: s.padding.top.max(inset(tl.max(tr))),
        bottom: s.padding.bottom.max(inset(bl.max(br))),
    };
    let size = rect.size();
    let x = if p.left + p.right > size.x {
        size.x / (p.left + p.right)
    } else {
        1.0
    };
    let y = if p.top + p.bottom > size.y {
        size.y / (p.top + p.bottom)
    } else {
        1.0
    };
    Rect::from_min_max(
        rect.min + Vec2::new(p.left * x, p.top * y),
        rect.max - Vec2::new(p.right * x, p.bottom * y),
    )
    .intersect(rect)
}
pub(super) fn surface(ui: &mut Ui<'_>, id: Id, rect: Rect, clip: Rect, s: SplitSurface) {
    if let Some(shadow) = s.shadow {
        ui.context.paint(
            id.with("shadow"),
            ui.window,
            clip,
            vec![Paint::Shape(Shape::Shadow {
                rect,
                rounding: s.rounding,
                shadow,
            })],
        );
    }
    ui.context.paint_blur(
        id.with("blur"),
        ui.window,
        clip,
        crate::Blur::new(rect)
            .radius(s.blur)
            .corner_radius(s.rounding),
    );
    let fill = ui.style().backdrop_fill(s.fill, s.blur);
    ui.context.paint(
        id.with("surface"),
        ui.window,
        clip,
        vec![Paint::Shape(
            Shape::rect(rect, fill)
                .corner_radius(s.rounding)
                .border(s.border)
                .into(),
        )],
    );
}
pub(super) fn hit_rect(
    axis: Layout,
    a: Rect,
    b: Rect,
    container: Rect,
    h: SplitHandleStyle,
) -> Rect {
    let edge = axis.main(a.max);
    let next = axis.main(b.min);
    let center = if h.kind == SplitHandle::PanelEdge {
        edge - h.hit_width * 0.5
    } else {
        (edge + next) * 0.5
    };
    // Never cross the midpoints of adjoining panels or overlap another boundary.
    let low = axis.main(a.min) + axis.main(a.size()) * 0.5;
    let high = if h.kind == SplitHandle::PanelEdge {
        edge
    } else {
        axis.main(b.min) + axis.main(b.size()) * 0.5
    };
    let start = (center - h.hit_width * 0.5).max(low).min(high);
    let end = (center + h.hit_width * 0.5).min(high).max(start);
    Rect::from_min_size(
        axis.size(start, axis.cross(container.min)),
        axis.size(end - start, axis.cross(container.size())),
    )
    .intersect(container)
}
pub(super) fn handle(
    ui: &mut Ui<'_>,
    id: Id,
    axis: Layout,
    bounds: Rect,
    h: SplitHandleStyle,
    hovered: bool,
    focus: bool,
    pressed: bool,
) {
    let visible = !matches!(h.kind, SplitHandle::Invisible | SplitHandle::PanelEdge)
        || hovered
        || focus
        || pressed;
    let color = if pressed {
        h.pressed
    } else if focus {
        h.focus
    } else if hovered {
        h.hover
    } else {
        h.idle
    };
    let cross = (axis.cross(bounds.size()) - h.inset * 2.0).max(0.0);
    let length = if h.kind == SplitHandle::Line {
        cross
    } else {
        h.length.min(cross)
    };
    let thickness = h.thickness.min(axis.main(bounds.size()));
    let rect = Rect::from_min_size(
        bounds.center() - axis.size(thickness, length) * 0.5,
        axis.size(thickness, length),
    );
    let mut paint = Vec::new();
    if visible {
        paint.push(Paint::Shape(
            Shape::rect(rect, color).corner_radius(h.rounding).into(),
        ));
        // Focus remains visible even when the requested grip thickness is zero.
        if focus {
            let focus_rect = Rect::from_min_size(
                bounds.center() - axis.size(2.0, length.max(12.0).min(cross)) * 0.5,
                axis.size(2.0, length.max(12.0).min(cross)),
            )
            .intersect(bounds);
            paint.push(Paint::Shape(
                Shape::rect(focus_rect, h.focus)
                    .corner_radius(CornerRadius::all(1.0))
                    .into(),
            ));
        }
    }
    if pressed && h.capture_indicator {
        paint.push(Paint::Shape(Shape::Circle {
            center: bounds.center(),
            radius: 3.0,
            fill: h.pressed,
            border: crate::Border::NONE,
        }));
    }
    ui.context.paint(
        id.with("handle"),
        ui.window,
        ui.clip.intersect(bounds),
        paint,
    );
}
