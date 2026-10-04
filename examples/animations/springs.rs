use super::common::{dot, lane, LANE};
use zaxis::{vec2, Border, Shape, SpringOptions, Ui, Vec2};

const RADIUS: f32 = 14.0;

#[derive(Default)]
pub struct Springs {
    pub right: bool,
    pub corner: usize,
}

pub fn show(ui: &mut Ui<'_>, state: &mut Springs) {
    ui.horizontal(|ui| {
        if ui.button("Retarget").clicked() {
            state.right = !state.right;
        }
        if ui.button("Next corner").clicked() {
            state.corner = (state.corner + 1) % 4;
        }
    });
    let target = if state.right { 1.0_f32 } else { 0.0 };
    let springs = [
        (
            "Underdamped (2 Hz, ratio 0.25)",
            SpringOptions::frequency(2.0, 0.25),
        ),
        ("Critical (default)", SpringOptions::default()),
        (
            "Overdamped (2 Hz, ratio 2.5)",
            SpringOptions::frequency(2.0, 2.5),
        ),
    ];
    for (name, options) in springs {
        ui.horizontal(|ui| {
            let value = ui
                .spring_transition(("spring", name), target, options)
                .value
                .value;
            let track = lane(ui);
            dot(ui, track, value);
            ui.label(name);
        });
    }
    ui.separator();
    ui.label("Vec2 spring keeps its velocity when the target changes");
    let area = ui.allocate_space(vec2(LANE.x, 150.0));
    let corners = [
        vec2(50.0, 40.0),
        vec2(LANE.x - 50.0, 30.0),
        vec2(LANE.x - 50.0, 110.0),
        vec2(50.0, 110.0),
    ];
    let motion = ui.spring_transition(
        "spring-2d",
        corners[state.corner],
        SpringOptions::frequency(1.6, 0.3),
    );
    let fill = ui.style().button_fill;
    ui.paint(Shape::rect(area, fill).corner_radius(6.0));
    let accent = ui.style().accent;
    ui.paint(Shape::Circle {
        center: (area.min + motion.value.value).clamp(
            area.min + Vec2::splat(RADIUS),
            area.max - Vec2::splat(RADIUS),
        ),
        radius: RADIUS,
        fill: accent,
        border: Border::NONE,
    });
}
