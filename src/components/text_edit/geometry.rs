use super::*;
use crate::context::Context;

/// Control characters are shown as spaces so byte offsets stay unchanged.
pub(super) fn display_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_control() { ' ' } else { c })
        .collect()
}
pub(super) fn positions(
    ctx: &mut Context,
    text: &str,
    size: f32,
    weight: crate::FontWeight,
) -> Vec<(usize, f32)> {
    let points = boundaries(text);
    ctx.text_carets(&display_line(text), size, weight)
        .into_iter()
        .filter(|(byte, _)| points.binary_search(byte).is_ok())
        .collect()
}
pub(super) fn x_at(points: &[(usize, f32)], byte: usize) -> f32 {
    points
        .iter()
        .rev()
        .find(|(i, _)| *i <= byte)
        .map_or(0.0, |(_, x)| *x)
}
pub(super) fn line(start: Vec2, end: Vec2, width: f32, color: Color) -> Paint {
    Paint::Shape(Shape::Line {
        start,
        end,
        width,
        color,
    })
}

impl TextEdit<'_> {
    pub(super) fn paint_affixes(
        &self,
        ui: &mut Ui<'_>,
        id: Id,
        outer: Rect,
        position: Vec2,
        size: f32,
        weight: crate::FontWeight,
        suffix_width: f32,
        color: Color,
    ) {
        ui.context.paint(
            id.with("affixes"),
            ui.window,
            ui.clip.intersect(outer),
            vec![
                Paint::Text {
                    text: self.affixes.0.clone(),
                    position: Vec2::new(outer.min.x, position.y),
                    size,
                    weight,
                    wrap_width: f32::INFINITY,
                    color,
                },
                Paint::Text {
                    text: self.affixes.1.clone(),
                    position: Vec2::new(outer.max.x - suffix_width, position.y),
                    size,
                    weight,
                    wrap_width: f32::INFINITY,
                    color,
                },
            ],
        );
    }
}
