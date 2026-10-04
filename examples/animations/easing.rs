use super::common::{dot, lane};
use std::time::Duration;
use zaxis::{Easing, Repeat, ScrollArea, Tween, Ui};

fn back_out(t: f32) -> f32 {
    let c = 1.70158;
    1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
}

fn rows(ui: &mut Ui<'_>, curves: Vec<(String, Easing)>) {
    for (name, easing) in curves {
        ui.horizontal(|ui| {
            let t = ui
                .animate(("ease", name.clone()), || {
                    Tween::new(0.0_f32, 1.0, Duration::from_millis(1600))
                        .easing(easing)
                        .repeat(Repeat::Forever)
                        .auto_reverse(true)
                })
                .value;
            let track = lane(ui);
            dot(ui, track, t);
            ui.label(name);
        });
    }
}

pub fn show(ui: &mut Ui<'_>) {
    use Easing::*;
    let mut curves: Vec<_> = [
        Linear, QuadIn, QuadOut, QuadInOut, CubicIn, CubicOut, CubicInOut, QuintIn, QuintOut,
        QuintInOut, SineIn, SineOut, SineInOut,
    ]
    .into_iter()
    .map(|easing| (format!("{easing:?}"), easing))
    .collect();
    curves.push(("Custom (back out)".into(), Easing::custom(back_out)));
    rows(ui, curves);
}

/// Expressive families (overshoot, bounce, snap) and CSS-style curves.
pub fn show_expressive(ui: &mut Ui<'_>) {
    use Easing::*;
    let mut curves: Vec<_> = [
        ExpoIn,
        ExpoOut,
        ExpoInOut,
        CircIn,
        CircOut,
        CircInOut,
        BackIn,
        BackOut,
        BackInOut,
        ElasticIn,
        ElasticOut,
        ElasticInOut,
        BounceIn,
        BounceOut,
        BounceInOut,
    ]
    .into_iter()
    .map(|easing| (format!("{easing:?}"), easing))
    .collect();
    curves.push(("CSS ease".into(), Easing::ease()));
    curves.push(("CSS ease-in-out".into(), Easing::ease_in_out()));
    curves.push((
        "cubic_bezier(0.7, -0.4, 0.3, 1.4)".into(),
        Easing::cubic_bezier(0.7, -0.4, 0.3, 1.4),
    ));
    curves.push(("steps(6)".into(), Easing::steps(6)));
    ScrollArea::vertical()
        .id_source("expressive-curves")
        .max_height(470.0)
        .show(ui, |ui| rows(ui, curves));
}
