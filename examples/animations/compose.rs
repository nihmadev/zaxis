use super::common::{fill, lane, LANE};
use std::time::Duration;
use zaxis::{
    vec2, AnimationSample, Border, Easing, Id, Keyframe, Keyframes, Parallel, Procedural, Rect,
    Repeat, Sequence, Shape, Stagger, Tween, Ui,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn sequence() -> Sequence<f32> {
    Sequence::new(0.0)
        .then(Tween::new(0.0, 1.0, ms(600)).easing(Easing::CubicOut))
        .delay(1.0, ms(400))
        .then(Tween::new(1.0, 0.2, ms(600)).easing(Easing::QuadInOut))
}
fn parallel() -> Parallel<f32> {
    Parallel::new()
        .with(Tween::new(0.0, 1.0, ms(500)).easing(Easing::QuadOut))
        .with(Tween::new(0.0, 1.0, ms(1400)).easing(Easing::SineInOut))
}
/// Two tracks where the second starts at a named moment of a sequence.
fn offsets() -> Parallel<f32> {
    let lead = Sequence::new(0.0_f32)
        .then(Tween::new(0.0, 1.0, ms(500)).easing(Easing::CubicOut))
        .mark("peak")
        .then(Tween::new(1.0, 0.3, ms(500)).easing(Easing::QuadInOut));
    let peak = lead.time_of("peak").unwrap();
    Parallel::new()
        .with(lead)
        .at(peak, Tween::new(0.0, 1.0, ms(700)).easing(Easing::BackOut))
}
fn stagger() -> Stagger<f32> {
    let mut stagger = Stagger::new(ms(90)).max_delay(ms(600));
    for i in 0..8 {
        stagger.insert(
            Id::new(("bar", i)),
            Duration::ZERO,
            Tween::new(0.0, 1.0, ms(450)).easing(Easing::CubicOut),
        );
    }
    stagger
}
fn keyframes() -> Keyframes<f32> {
    Keyframes::new([
        Keyframe::new(Duration::ZERO, 0.0),
        Keyframe::new(ms(400), 1.0).easing(Easing::CubicOut),
        Keyframe::new(ms(700), 0.4).easing(Easing::QuadInOut),
        Keyframe::new(ms(1200), 1.0).easing(Easing::SineInOut),
    ])
    .repeat(Repeat::Forever)
    .auto_reverse(true)
}

pub fn show(ui: &mut Ui<'_>) {
    if ui.button("Restart").clicked() {
        let ids = ["seq", "par", "stagger", "keys", "offsets"].map(|name| ui.animation_id(name));
        let context = ui.context();
        context.restart_animation(ids[0], sequence());
        context.restart_animation(ids[1], parallel());
        context.restart_animation(ids[2], stagger());
        context.restart_animation(ids[3], keyframes());
        context.restart_animation(ids[4], offsets());
    }
    let accent = ui.style().accent;
    let warning = ui.style().warning;

    ui.horizontal(|ui| {
        let t = ui.animate("seq", sequence).value;
        let track = lane(ui);
        fill(ui, track, t, accent);
        ui.label("Sequence: tween, Delay, tween");
    });
    ui.horizontal(|ui| {
        let values = ui.animate("par", parallel).value;
        ui.vertical(|ui| {
            for (value, color) in values.into_iter().zip([accent, warning]) {
                let track = lane(ui);
                fill(ui, track, value, color);
            }
        });
        ui.label("Parallel: 500 ms and 1400 ms tracks");
    });
    ui.horizontal(|ui| {
        let values = ui.animate("offsets", offsets).value;
        ui.vertical(|ui| {
            for (value, color) in values.into_iter().zip([accent, warning]) {
                let track = lane(ui);
                fill(ui, track, value, color);
            }
        });
        ui.label("Parallel::at: second starts at the \"peak\" mark");
    });
    ui.horizontal(|ui| {
        let values = ui.animate("stagger", stagger).value;
        let area = ui.allocate_space(vec2(LANE.x, 56.0));
        let width = area.size().x / values.len() as f32;
        for (i, (_, t)) in values.into_iter().enumerate() {
            let height = 6.0 + 50.0 * t;
            let rect = Rect::from_min_size(
                vec2(area.min.x + i as f32 * width + 3.0, area.max.y - height),
                vec2(width - 6.0, height),
            );
            ui.paint(Shape::rect(rect, accent).corner_radius(3.0));
        }
        ui.label("Stagger: 90 ms steps");
    });
    ui.horizontal(|ui| {
        let t = ui.animate("keys", keyframes).value;
        let track = lane(ui);
        fill(ui, track, t, accent);
        ui.label("Keyframes with per-segment easing");
    });
    ui.horizontal(|ui| {
        let t = ui
            .animate("procedural", || {
                Procedural::new(0.5_f32, |elapsed: Duration| {
                    let t = elapsed.as_secs_f32();
                    AnimationSample::running(0.5 + 0.5 * (t * 4.0).sin() * (t * 1.3).cos())
                })
            })
            .value;
        let track = lane(ui);
        let center = track.center();
        let x = track.min.x + 10.0 + t * (track.size().x - 20.0);
        ui.paint(Shape::Circle {
            center: vec2(x, center.y),
            radius: 7.0,
            fill: warning,
            border: Border::NONE,
        });
        ui.label("Procedural closure");
    });
}
