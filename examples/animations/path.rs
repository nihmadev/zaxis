use super::common::LANE;
use std::time::Duration;
use zaxis::{
    vec2, Border, Color, Easing, Interpolate, Oklab, Path, PathFollow, Rect, Repeat, Shape, Tween,
    Ui, Vec2,
};

const AREA: Vec2 = Vec2::new(LANE.x, 150.0);
const BLUE: Color = Color::rgb(40, 80, 220);
const YELLOW: Color = Color::rgb(240, 210, 40);

fn route() -> Path {
    Path::new(vec2(30.0, 100.0))
        .cubic_to(vec2(110.0, -20.0), vec2(190.0, 220.0), vec2(270.0, 100.0))
        .quad_to(vec2(330.0, 20.0), vec2(360.0, 100.0))
}

fn stroke(ui: &mut Ui<'_>, points: &[Vec2], origin: Vec2, width: f32, color: Color) {
    for pair in points.windows(2) {
        ui.paint(Shape::Line {
            start: origin + pair[0],
            end: origin + pair[1],
            width,
            color,
        });
    }
}

fn panel(ui: &mut Ui<'_>) -> Vec2 {
    let area = ui.allocate_space(AREA);
    let fill = ui.style().button_fill;
    ui.paint(Shape::rect(area, fill).corner_radius(6.0));
    area.min
}

pub fn show(ui: &mut Ui<'_>) {
    let (accent, muted) = (ui.style().accent, ui.style().muted_text);

    ui.label("PathFollow: constant speed along lines and Bezier curves, facing the tangent");
    let origin = panel(ui);
    let outline = route();
    stroke(ui, outline.points(), origin, 2.0, muted);
    let pose = ui
        .animate("follow", || {
            PathFollow::new(route(), Duration::from_millis(3600))
                .easing(Easing::SineInOut)
                .repeat(Repeat::Forever)
                .auto_reverse(true)
        })
        .value;
    let centre = origin + pose.position;
    ui.paint(Shape::Circle {
        center: centre,
        radius: 9.0,
        fill: accent,
        border: Border::NONE,
    });
    ui.paint(Shape::Line {
        start: centre,
        end: centre + vec2(pose.angle.cos(), pose.angle.sin()) * 18.0,
        width: 3.0,
        color: Color::WHITE,
    });
    ui.separator();

    ui.label("Path::trimmed: draw-on stroke");
    let origin = panel(ui);
    stroke(ui, outline.points(), origin, 6.0, muted);
    let drawn = ui
        .animate("draw-on", || {
            Tween::new(0.0_f32, 1.0, Duration::from_millis(2400))
                .easing(Easing::CubicInOut)
                .repeat(Repeat::Forever)
                .auto_reverse(true)
        })
        .value;
    stroke(ui, &outline.trimmed(0.0, drawn), origin, 6.0, accent);
    ui.separator();

    ui.label("Color blends: linear light (top) vs Oklab (bottom), blue to yellow");
    let strip = |ui: &mut Ui<'_>, blend: &dyn Fn(f32) -> Color| {
        let rect = ui.allocate_space(vec2(LANE.x, 18.0));
        let steps = 40;
        let width = rect.size().x / steps as f32;
        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let swatch = Rect::from_min_size(
                rect.min + vec2(i as f32 * width, 0.0),
                vec2(width + 0.5, rect.size().y),
            );
            ui.paint(Shape::rect(swatch, blend(t)));
        }
    };
    strip(ui, &|t| BLUE.interpolate(&YELLOW, t));
    strip(ui, &|t| Oklab(BLUE).interpolate(&Oklab(YELLOW), t).0);
    ui.horizontal(|ui| {
        let tween = |from: Color, to: Color| {
            Tween::new(from, to, Duration::from_millis(1800))
                .repeat(Repeat::Forever)
                .auto_reverse(true)
        };
        let linear = ui.animate("blend-linear", || tween(BLUE, YELLOW)).value;
        let perceptual = ui
            .animate("blend-oklab", || {
                Tween::new(Oklab(BLUE), Oklab(YELLOW), Duration::from_millis(1800))
                    .repeat(Repeat::Forever)
                    .auto_reverse(true)
            })
            .value
            .0;
        for color in [linear, perceptual] {
            let swatch = ui.allocate_space(vec2(LANE.x * 0.5 - 4.0, 28.0));
            ui.paint(Shape::rect(swatch, color).corner_radius(4.0));
        }
    });
}
