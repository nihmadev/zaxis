use super::common::{dot, lane};
use std::time::Duration;
use zaxis::{
    vec2, Border, Color, CornerRadius, Easing, Rect, Repeat, Shape, Tween, TweenOptions, Ui,
};

const ROWS: [&str; 4] = [
    "Once",
    "Repeat::Count(3)",
    "Forever + auto_reverse",
    "delay 800 ms",
];

fn make(row: usize) -> Tween<f32> {
    let base = Tween::new(0.0_f32, 1.0, Duration::from_millis(900)).easing(Easing::CubicInOut);
    match row {
        0 => base,
        1 => base.repeat(Repeat::Count(3)),
        2 => base.repeat(Repeat::Forever).auto_reverse(true),
        _ => base.delay(Duration::from_millis(800)),
    }
}

fn control() -> Tween<f32> {
    Tween::new(0.0_f32, 1.0, Duration::from_millis(2400))
        .easing(Easing::SineInOut)
        .repeat(Repeat::Forever)
        .auto_reverse(true)
}

pub fn show(ui: &mut Ui<'_>, toggled: &mut bool) {
    if ui.button("Restart tweens").clicked() {
        for row in 0..ROWS.len() {
            let id = ui.animation_id(("tween", row));
            ui.context().restart_animation(id, make(row));
        }
    }
    for (row, name) in ROWS.into_iter().enumerate() {
        ui.horizontal(|ui| {
            let t = ui.animate(("tween", row), || make(row)).value;
            let track = lane(ui);
            dot(ui, track, t);
            ui.label(name);
        });
    }
    ui.separator();
    let id = ui.animation_id("control");
    let t = ui.animate("control", control).value;
    ui.horizontal(|ui| {
        let track = lane(ui);
        dot(ui, track, t);
        let status = ui.context().animation_status(id);
        ui.label(format!("{status:?}"));
    });
    ui.horizontal(|ui| {
        if ui.button("Pause").clicked() {
            ui.context().pause_animation(id);
        }
        if ui.button("Resume").clicked() {
            ui.context().resume_animation(id);
        }
        if ui.button("Cancel").clicked() {
            ui.context().cancel_animation(id);
        }
        if ui.button("Finish").clicked() {
            ui.context().finish_animation(id);
        }
        if ui.button("Restart").clicked() {
            ui.context().restart_animation(id, control());
        }
    });
    ui.separator();
    ui.checkbox(
        toggled,
        "Retarget mid-flight: Rect, Color, CornerRadius, Border",
    );
    let area = ui.allocate_space(vec2(540.0, 120.0));
    let motion = TweenOptions::new(Duration::from_millis(900)).easing(Easing::QuintInOut);
    let (rect, color, radius, border) = if *toggled {
        (
            Rect::from_min_size(vec2(330.0, 10.0), vec2(190.0, 100.0)),
            Color::rgb(87, 146, 206),
            CornerRadius::all(50.0),
            Border {
                width: 4.0,
                color: Color::WHITE,
            },
        )
    } else {
        (
            Rect::from_min_size(vec2(20.0, 30.0), vec2(90.0, 60.0)),
            Color::rgb(191, 118, 87),
            CornerRadius::all(4.0),
            Border {
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        )
    };
    let rect = ui.transition("t-rect", rect, motion.clone()).value;
    let color = ui.transition("t-color", color, motion.clone()).value;
    let radius = ui.transition("t-radius", radius, motion.clone()).value;
    let border = ui.transition("t-border", border, motion).value;
    ui.paint(
        Shape::rect(rect.translate(area.min), color)
            .corner_radius(radius)
            .border(border),
    );
}
