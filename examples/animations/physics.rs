use super::common::{dot, lane, LANE};
use std::time::Duration;
use zaxis::{
    vec2, Border, Decay, DecayOptions, Easing, Shape, SpringOptions, TweenOptions, Ui, Vec2,
};

#[derive(Default)]
pub struct Physics {
    pub right: bool,
    pub spot: usize,
}

const RADIUS: f32 = 14.0;

pub fn show(ui: &mut Ui<'_>, state: &mut Physics) {
    ui.horizontal(|ui| {
        if ui.button("Retarget").clicked() {
            state.right = !state.right;
        }
        if ui.button("Fling").clicked() {
            state.spot = (state.spot + 1) % 4;
            let id = ui.animation_id("decay");
            let from = ui
                .context()
                .sample_animation::<Vec2>(id)
                .map_or(Vec2::ZERO, |motion| motion.value);
            let options = DecayOptions::half_life(Duration::from_millis(350));
            let target = spots()[state.spot];
            // Choose the launch speed that makes the fling coast to the spot.
            let velocity = (target - from) * options.friction as f32;
            ui.context()
                .restart_animation(id, Decay::with_options(from, velocity, options));
        }
    });
    let target = if state.right { 1.0_f32 } else { 0.0 };

    ui.label("SpringOptions::duration_bounce(600 ms, bounce)");
    for bounce in [-0.4, 0.0, 0.35] {
        ui.horizontal(|ui| {
            let options = SpringOptions::duration_bounce(Duration::from_millis(600), bounce);
            let value = ui
                .spring_transition(("bounce", bounce.to_bits()), target, options)
                .value
                .value;
            let track = lane(ui);
            dot(ui, track, value);
            ui.label(format!("bounce {bounce:+.2}"));
        });
    }
    ui.separator();

    ui.label("Retarget mid-flight: transition vs transition_smooth");
    let motion = TweenOptions::new(Duration::from_millis(900)).easing(Easing::QuintInOut);
    ui.horizontal(|ui| {
        let value = ui.transition("plain", target, motion.clone()).value;
        let track = lane(ui);
        dot(ui, track, value);
        ui.label("transition: speed resets");
    });
    ui.horizontal(|ui| {
        let value = ui.transition_smooth("smooth", target, motion).value;
        let track = lane(ui);
        dot(ui, track, value);
        ui.label("transition_smooth: speed kept");
    });
    ui.separator();

    ui.label("Decay: Fling coasts to the next spot with exponential friction");
    let area = ui.allocate_space(vec2(LANE.x, 110.0));
    let motion = ui.animate("decay", || Decay::new(spots()[0], Vec2::ZERO));
    let fill = ui.style().button_fill;
    ui.paint(Shape::rect(area, fill).corner_radius(6.0));
    let accent = ui.style().accent;
    ui.paint(Shape::Circle {
        center: area.min + motion.value,
        radius: RADIUS,
        fill: accent,
        border: Border::NONE,
    });
}

fn spots() -> [Vec2; 4] {
    [
        vec2(40.0, 30.0),
        vec2(LANE.x - 40.0, 30.0),
        vec2(LANE.x - 40.0, 80.0),
        vec2(40.0, 80.0),
    ]
}
