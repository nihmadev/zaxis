use super::common::{dot, lane};
use std::time::Duration;
use zaxis::{Easing, Tween, Ui};

fn track() -> Tween<f32> {
    Tween::new(0.0_f32, 1.0, Duration::from_millis(4000)).easing(Easing::SineInOut)
}

/// A channel that can be paused, reversed, sped up and scrubbed.
pub fn show(ui: &mut Ui<'_>) {
    let id = ui.animation_id("timeline");
    let value = ui.animate("timeline", track);
    ui.horizontal(|ui| {
        let lane = lane(ui);
        dot(ui, lane, value.value);
        let rate = ui.context().animation_rate(id).unwrap_or(1.0);
        ui.label(format!("{:?}, {rate:+.2}x", value.status));
    });
    ui.horizontal(|ui| {
        if ui.button("Restart").clicked() {
            ui.context().restart_animation(id, track());
        }
        if ui.button("Pause").clicked() {
            ui.context().pause_animation(id);
        }
        if ui.button("Resume").clicked() {
            ui.context().resume_animation(id);
        }
        if ui.button("Reverse").clicked() {
            ui.context().reverse_animation(id);
        }
    });
    ui.horizontal(|ui| {
        for rate in [0.25, 0.5, 1.0, 2.0, 4.0] {
            if ui.button(format!("{rate}x")).clicked() {
                let sign = ui.context().animation_rate(id).map_or(1.0, f64::signum);
                ui.context().set_animation_rate(id, rate * sign);
            }
        }
    });
    let elapsed = ui.context().animation_elapsed(id).unwrap_or_default();
    let length = ui.context().animation_duration(id);
    let mut fraction = length.map_or(1.0, |length| {
        (elapsed.as_secs_f32() / length.as_secs_f32()).clamp(0.0, 1.0)
    });
    let scrub = ui.slider(&mut fraction, 0.0..=1.0);
    if let (true, Some(length)) = (scrub.changed(), length) {
        ui.context().seek_animation(id, length.mul_f32(fraction));
    }
    if length.is_none() {
        ui.label("Finished: restart to scrub again");
    }
}
