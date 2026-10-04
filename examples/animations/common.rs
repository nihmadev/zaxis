use zaxis::{vec2, Border, Color, Rect, Shape, Ui, Vec2};

pub const LANE: Vec2 = Vec2::new(380.0, 18.0);

/// Track with a rounded background; returns the lane for further painting.
pub fn lane(ui: &mut Ui<'_>) -> Rect {
    let rect = ui.allocate_space(LANE);
    let fill = ui.style().button_fill;
    ui.paint(Shape::rect(rect, fill).corner_radius(LANE.y * 0.5));
    rect
}

/// Dot at progress `t`. The end of the motion sits at 80% of the lane so
/// overshoot has headroom; the dot never leaves the track.
pub fn dot(ui: &mut Ui<'_>, lane: Rect, t: f32) {
    let radius = lane.size().y * 0.5 - 2.0;
    let (left, right) = (lane.min.x + radius + 2.0, lane.max.x - radius - 2.0);
    let x = (left + t * (right - left) * 0.8).clamp(left, right);
    let fill = ui.style().accent;
    ui.paint(Shape::Circle {
        center: vec2(x, lane.center().y),
        radius,
        fill,
        border: Border::NONE,
    });
}

/// Filled share of the lane.
pub fn fill(ui: &mut Ui<'_>, lane: Rect, t: f32, color: Color) {
    let width = lane.size().x * t.clamp(0.0, 1.0);
    if width > 0.5 {
        let rect = Rect::from_min_size(lane.min, vec2(width, lane.size().y));
        ui.paint(Shape::rect(rect, color).corner_radius(lane.size().y * 0.5));
    }
}
