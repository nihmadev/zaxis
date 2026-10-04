use super::common::LANE;
use std::time::Duration;
use zaxis::{vec2, Loader, Progress, ProgressState, Pulse, Repeat, Rotation, Shape, Tween, Ui};

#[derive(Default)]
pub struct Effects {
    pub pulse: bool,
    pub revision: u64,
}

pub fn show(ui: &mut Ui<'_>, state: &mut Effects) {
    ui.label("Loader");
    ui.horizontal(|ui| {
        ui.add(Loader::new().size(16.0));
        ui.add(Loader::new().size(24.0).period(Duration::from_millis(600)));
        ui.add(
            Loader::new()
                .size(36.0)
                .stroke(3.0)
                .period(Duration::from_millis(2400)),
        );
    });
    ui.label("Progress");
    ui.add(Progress::new(ProgressState::Indeterminate { active: true }).size(vec2(LANE.x, 8.0)));
    let value = ui
        .animate("progress", || {
            Tween::new(0.0_f32, 1.0, Duration::from_millis(3000)).repeat(Repeat::Forever)
        })
        .value;
    ui.add(Progress::new(ProgressState::Determinate(value)).size(vec2(LANE.x, 8.0)));
    ui.add(Progress::new(ProgressState::Complete).size(vec2(LANE.x, 8.0)));
    ui.separator();

    ui.label("Skeleton");
    ui.skeleton(14.0);
    ui.horizontal(|ui| {
        ui.add(zaxis::Skeleton::new(36.0).width(36.0).corner_radius(18.0));
        ui.vertical(|ui| {
            ui.add(zaxis::Skeleton::new(12.0).width(220.0));
            ui.add(zaxis::Skeleton::new(12.0).width(140.0));
        });
    });
    ui.separator();

    ui.label("Rotation");
    let angle = ui
        .animate("rotation", || Rotation::new(Duration::from_millis(2000)))
        .value;
    let area = ui.allocate_space(vec2(48.0, 48.0));
    let (center, color) = (area.center(), ui.style().accent);
    ui.paint(Shape::Line {
        start: center,
        end: center + vec2(angle.cos(), angle.sin()) * 22.0,
        width: 3.0,
        color,
    });
    ui.separator();

    ui.label("Pulse");
    ui.checkbox(&mut state.pulse, "Active");
    ui.horizontal(|ui| {
        ui.pulse("pulse-default", state.pulse, |ui| ui.label("Default pulse"));
        let effect = Pulse::new(Duration::from_millis(700)).minimum(0.15);
        ui.pulse_with("pulse-fast", state.pulse, effect, |ui| {
            ui.label("Fast, deep pulse")
        });
    });
    ui.separator();

    ui.label("Highlight");
    ui.horizontal(|ui| {
        if ui.button("Change value").clicked() {
            state.revision += 1;
        }
        let rect = ui.allocate_space(vec2(160.0, 28.0));
        let (base, accent) = (ui.style().button_fill, ui.style().accent);
        ui.highlight("highlight", state.revision, rect, base, accent);
        ui.label(format!("revision {}", state.revision));
    });
}
