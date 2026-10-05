//! Geometry and drawing of the thumb, the focus ring and segment contents.

use super::layout::{snap, Label, Metrics};
use crate::{
    components::{appearance::Appearance, Ui},
    context::Paint,
    Border, Color, CornerRadius, Id, ImageSource, Rect, Shape, Vec2,
};

/// Segment rectangles inside the plate; edges sit on whole physical pixels.
pub(super) fn segment_rects(
    bounds: Rect,
    widths: &[f32],
    m: &Metrics,
    vertical: bool,
) -> Vec<Rect> {
    let mut cursor = bounds.min + Vec2::splat(m.padding);
    widths
        .iter()
        .map(|w| {
            let rect = Rect::from_min_size(cursor, Vec2::new(*w, m.seg_h));
            if vertical {
                cursor.y += m.seg_h + m.gap;
            } else {
                cursor.x += w + m.gap;
            }
            rect
        })
        .collect()
}

/// The raised plate under the selected segment. Its shadow is drawn with the plate, outside
/// the track clip, so the plate's softness is never cut off by the track edge.
pub(super) fn paint_thumb(ui: &mut Ui<'_>, id: Id, rect: Rect, appearance: Appearance) {
    let mut paint = Vec::new();
    appearance.paint_shadow(rect, appearance.rounding, &mut paint);
    let style = ui.style().clone();
    appearance.paint_body(rect, appearance.rounding, &style, 0.0, &mut paint);
    ui.context.paint(id, ui.window, ui.clip, paint);
}

/// Keyboard-focus ring around one segment, inside the plate padding.
pub(super) fn paint_focus_ring(
    ui: &mut Ui<'_>,
    id: Id,
    rect: Rect,
    rounding: CornerRadius,
    border: Border,
) {
    ui.context.paint(
        id,
        ui.window,
        ui.clip,
        vec![Paint::Shape(Shape::Rect {
            rect,
            rounding,
            fill: Color::TRANSPARENT,
            border,
        })],
    );
}

/// Corner radii of segment `index` of `count` in the outlined variant: only the ends of the
/// row are round, so the selected fill follows the pill.
pub(super) fn end_rounding(
    index: usize,
    count: usize,
    radius: f32,
    vertical: bool,
) -> CornerRadius {
    let (first, last) = (index == 0, index + 1 == count);
    let r = |on: bool| if on { radius } else { 0.0 };
    if vertical {
        CornerRadius {
            top_left: r(first),
            top_right: r(first),
            bottom_right: r(last),
            bottom_left: r(last),
        }
    } else {
        CornerRadius {
            top_left: r(first),
            bottom_left: r(first),
            top_right: r(last),
            bottom_right: r(last),
        }
    }
}

pub(super) fn paint_fill(ui: &mut Ui<'_>, id: Id, rect: Rect, rounding: CornerRadius, fill: Color) {
    if fill.0[3] > 0 {
        ui.context.paint(
            id,
            ui.window,
            ui.clip,
            vec![Paint::Shape(Shape::Rect {
                rect,
                rounding,
                fill,
                border: Border::NONE,
            })],
        );
    }
}

/// Hairlines between neighbouring segments, centered in the gap.
pub(super) fn paint_dividers(
    ui: &mut Ui<'_>,
    id: Id,
    rects: &[Rect],
    border: Border,
    vertical: bool,
    gap: f32,
) {
    if border.width <= 0.0 {
        return;
    }
    let lines = rects.windows(2).map(|pair| {
        let (a, b) = (pair[0], pair[1]);
        let (start, end) = if vertical {
            let y = a.max.y + gap * 0.5;
            (Vec2::new(a.min.x, y), Vec2::new(a.max.x, y))
        } else {
            let x = a.max.x + gap * 0.5;
            (
                Vec2::new(x, a.min.y.min(b.min.y)),
                Vec2::new(x, a.max.y.max(b.max.y)),
            )
        };
        Paint::Shape(Shape::Line {
            start,
            end,
            width: border.width,
            color: border.color,
        })
    });
    ui.context.paint(id, ui.window, ui.clip, lines.collect());
}

/// What leads a segment's label: its icon, or the selection check that replaces it.
pub(super) enum Lead {
    None,
    Icon(ImageSource),
    Check,
}

/// Icon (or check) and label centered as one group, clipped to the segment so nothing leaks
/// while the thumb slides or the row is squeezed.
pub(super) fn paint_content(
    ui: &mut Ui<'_>,
    id: Id,
    rect: Rect,
    lead: Lead,
    label: Option<&Label>,
    color: Color,
    m: &Metrics,
) {
    let text_w = label.map_or(0.0, |l| l.size.x);
    let icon_w = if !matches!(lead, Lead::None) {
        m.icon + if text_w > 0.0 { m.icon_gap } else { 0.0 }
    } else {
        0.0
    };
    let mut x = snap(rect.center().x - (icon_w + text_w) * 0.5, m.scale).max(rect.min.x);
    let clip = ui.clip;
    ui.clip = clip.intersect(rect);
    let y = snap(rect.center().y - m.icon * 0.5, m.scale);
    let glyph = Rect::from_min_size(Vec2::new(x, y), Vec2::splat(m.icon));
    match lead {
        Lead::Icon(icon) => ui.paint_image_in(id.with("icon"), icon, glyph, color),
        Lead::Check => {
            let point = |px: f32, py: f32| glyph.min + Vec2::new(px, py) * m.icon;
            let width = (m.icon * 0.1).max(1.5);
            let line = |start, end| {
                Paint::Shape(Shape::Line {
                    start,
                    end,
                    width,
                    color,
                })
            };
            ui.context.paint(
                id.with("check"),
                ui.window,
                ui.clip,
                vec![
                    line(point(0.2, 0.52), point(0.42, 0.74)),
                    line(point(0.42, 0.74), point(0.8, 0.28)),
                ],
            );
        }
        Lead::None => {}
    }
    x += icon_w;
    if let Some(label) = label {
        let optical = ui
            .context
            .centered_line_offset(&label.text, m.font, m.weight);
        let y = snap(rect.center().y - label.size.y * 0.5 + optical, m.scale);
        ui.context.paint(
            id.with("label"),
            ui.window,
            ui.clip,
            vec![Paint::Text {
                text: label.text.clone(),
                position: Vec2::new(x, y),
                size: m.font,
                weight: m.weight,
                wrap_width: f32::INFINITY,
                color,
            }],
        );
    }
    ui.clip = clip;
}
