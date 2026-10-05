//! Drawing helpers: painter hooks, the focus ring and the text block of a row.

use super::layout::{snap, Cell, Metrics};
use crate::{
    components::{
        theme::{painter::PaintHook, ControlPaint, PaintMode},
        Ui,
    },
    context::Paint,
    Border, Color, CornerRadius, Id, Rect, Shape, Vec2,
};

/// Runs `builtin` and the application's painter hook for one part, in the order its
/// [`PaintMode`] asks for; `Replace` skips `builtin`.
pub(super) fn part(
    hook: Option<&PaintHook<'_>>,
    clip: Rect,
    info: ControlPaint,
    out: &mut Vec<Paint>,
    builtin: impl FnOnce(&mut Vec<Paint>),
) {
    match hook {
        None => builtin(out),
        Some(h) => match h.mode {
            PaintMode::Replace => h.run(out, clip, info),
            PaintMode::Before => {
                h.run(out, clip, info);
                builtin(out);
            }
            PaintMode::After => {
                builtin(out);
                h.run(out, clip, info);
            }
        },
    }
}

/// The indicator square of a row: left edge on the row, centered on the first label line,
/// both snapped to physical pixels so it never shifts between frames.
pub(super) fn indicator_rect(row: Rect, cell: &Cell, m: &Metrics) -> Rect {
    let center = row.min.y + cell.text_top + cell.line_h * 0.5;
    let top = snap(center - m.d * 0.5, m.scale);
    Rect::from_min_size(Vec2::new(row.min.x + m.inset, top), Vec2::splat(m.d))
}

/// `rect` grown by `amount` on every side.
pub(super) fn grow(rect: Rect, amount: f32) -> Rect {
    Rect::from_min_max(
        rect.min - Vec2::splat(amount),
        rect.max + Vec2::splat(amount),
    )
}

pub(super) fn round(rect: Rect) -> CornerRadius {
    CornerRadius::all(rect.size().x.min(rect.size().y) * 0.5)
}

/// Keyboard focus ring around the indicator, outside of the halo's edge.
pub(super) fn paint_focus_ring(ui: &mut Ui<'_>, id: Id, rect: Rect, border: Border) {
    let ring = grow(rect, border.width + 1.0);
    ui.context.paint(
        id,
        ui.window,
        ui.clip,
        vec![Paint::Shape(Shape::Rect {
            rect: ring,
            rounding: round(ring),
            fill: Color::TRANSPARENT,
            border,
        })],
    );
}

pub(super) struct Texts<'a> {
    pub label: &'a str,
    pub description: &'a str,
    pub label_color: Color,
    pub description_color: Color,
}

/// Label and description, wrapped at the row's text width and clipped to the row so
/// nothing leaks into the container or the neighbours.
pub(super) fn paint_texts(
    ui: &mut Ui<'_>,
    id: Id,
    row: Rect,
    cell: &Cell,
    texts: &Texts<'_>,
    m: &Metrics,
) {
    if cell.text_w <= 0.0 {
        return;
    }
    let left = row.min.x + m.inset + m.d + m.label_gap;
    let top = row.min.y + cell.text_top;
    let mut list = Vec::new();
    if cell.label_h > 0.0 {
        list.push(Paint::Text {
            text: texts.label.to_owned(),
            position: Vec2::new(left, top),
            size: m.font,
            weight: m.weight,
            wrap_width: cell.text_w,
            color: texts.label_color,
        });
    }
    if cell.desc_h > 0.0 {
        list.push(Paint::Text {
            text: texts.description.to_owned(),
            position: Vec2::new(left, top + cell.label_h + m.line_gap),
            size: m.desc_font,
            weight: m.weight,
            wrap_width: cell.text_w,
            color: texts.description_color,
        });
    }
    ui.context
        .paint(id, ui.window, ui.clip.intersect(row), list);
}
