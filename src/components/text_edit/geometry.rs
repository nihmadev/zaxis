use super::*;

pub(super) fn single_line(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}
pub(super) fn display_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_control() { ' ' } else { c })
        .collect()
}
pub(super) fn positions(ui: &mut Ui<'_>, text: &str, size: f32) -> Vec<(usize, f32)> {
    let points = boundaries(text);
    ui.context
        .text_carets(&display_line(text), size)
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
                    wrap_width: f32::INFINITY,
                    color,
                },
                Paint::Text {
                    text: self.affixes.1.clone(),
                    position: Vec2::new(outer.max.x - suffix_width, position.y),
                    size,
                    wrap_width: f32::INFINITY,
                    color,
                },
            ],
        );
    }
}
