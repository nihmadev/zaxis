use crate::prelude::*;
use std::time::Duration;
use winit::dpi::PhysicalSize;
use zaxis::{
    animation::{testing, *},
    vec2, Vec2,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn near(value: f32, expected: f32, tolerance: f32) {
    assert!(
        (value - expected).abs() <= tolerance,
        "{value} != {expected} (+-{tolerance})"
    );
}
fn grid() -> impl Iterator<Item = f32> {
    (1..100).map(|i| i as f32 / 100.0)
}

/// (in, out, in-out) triples of every mirrored family.
fn families() -> Vec<(Easing, Easing, Easing)> {
    use Easing::*;
    vec![
        (QuadIn, QuadOut, QuadInOut),
        (CubicIn, CubicOut, CubicInOut),
        (QuintIn, QuintOut, QuintInOut),
        (SineIn, SineOut, SineInOut),
        (ExpoIn, ExpoOut, ExpoInOut),
        (CircIn, CircOut, CircInOut),
        (BackIn, BackOut, BackInOut),
        (ElasticIn, ElasticOut, ElasticInOut),
        (BounceIn, BounceOut, BounceInOut),
    ]
}

#[test]
fn every_family_has_exact_endpoints_continuous_ends_and_mirrored_forms() {
    for (curve_in, curve_out, curve_in_out) in families() {
        for curve in [&curve_in, &curve_out, &curve_in_out] {
            assert_eq!(curve.sample(0.0), 0.0, "{curve:?}");
            assert_eq!(curve.sample(1.0), 1.0, "{curve:?}");
            // Circ has a vertical tangent at its square-root end.
            let tolerance = if format!("{curve:?}").starts_with("Circ") {
                2.0e-2
            } else {
                2.0e-3
            };
            near(curve.sample(1.0 - 1.0e-4), 1.0, tolerance);
            near(curve.sample(1.0e-4), 0.0, tolerance);
        }
        near(curve_in_out.sample(0.5), 0.5, 1.0e-5);
        for t in grid() {
            near(curve_in.sample(t) + curve_out.sample(1.0 - t), 1.0, 1.0e-5);
        }
    }
}

#[test]
fn overshooting_and_bounded_families_behave_as_documented() {
    let peak = |easing: &Easing| grid().map(|t| easing.sample(t)).fold(f32::MIN, f32::max);
    let floor = |easing: &Easing| grid().map(|t| easing.sample(t)).fold(f32::MAX, f32::min);
    assert!(peak(&Easing::BackOut) > 1.05 && floor(&Easing::BackIn) < -0.05);
    assert!(peak(&Easing::ElasticOut) > 1.1 && floor(&Easing::ElasticIn) < -0.1);
    for bounded in [
        Easing::BounceOut,
        Easing::BounceIn,
        Easing::CircOut,
        Easing::ExpoOut,
    ] {
        assert!(
            peak(&bounded) <= 1.0 + 1.0e-6 && floor(&bounded) >= -1.0e-6,
            "{bounded:?}"
        );
    }
    let bounces = grid()
        .map(|t| Easing::BounceOut.sample(t))
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|pair| pair[1] < pair[0])
        .count();
    assert!(bounces > 10, "bounce must come back down several times");
}

#[test]
fn cubic_bezier_matches_css_and_steps_jump_at_interval_ends() {
    let linear = Easing::cubic_bezier(0.0, 0.0, 1.0, 1.0);
    for t in grid() {
        near(linear.sample(t), t, 1.0e-3);
    }
    near(Easing::ease().sample(0.5), 0.8024, 2.0e-3);
    near(Easing::ease_in_out().sample(0.5), 0.5, 1.0e-3);
    let samples: Vec<_> = grid().map(|t| Easing::ease_in_out().sample(t)).collect();
    assert!(samples.windows(2).all(|pair| pair[1] >= pair[0]));
    assert_eq!(Easing::ease(), Easing::ease());
    assert_ne!(Easing::ease(), Easing::ease_in_out());
    assert_eq!(Easing::steps(4), Easing::steps(4));
    assert_ne!(Easing::steps(4), Easing::steps(5));

    let steps = Easing::steps(4);
    assert_eq!(steps.sample(0.1), 0.0);
    assert_eq!(steps.sample(0.3), 0.25);
    assert_eq!(steps.sample(0.99), 0.75);
    assert_eq!(steps.sample(1.0), 1.0);
}

#[test]
#[should_panic(expected = "x1, x2")]
fn cubic_bezier_rejects_time_going_backwards() {
    Easing::cubic_bezier(1.5, 0.0, 0.5, 1.0);
}

#[test]
#[should_panic(expected = "at least one step")]
fn zero_steps_are_rejected() {
    Easing::steps(0);
}

#[test]
fn tweens_with_new_curves_finish_exactly_at_every_frame_rate() {
    for easing in [
        Easing::ExpoOut,
        Easing::BackInOut,
        Easing::ElasticOut,
        Easing::BounceOut,
        Easing::ease(),
        Easing::steps(5),
    ] {
        testing::assert_exact_finish(
            &Tween::new(0.0_f32, 10.0, ms(400)).easing(easing),
            &testing::FRAME_RATES,
        );
    }
}

#[test]
fn decay_glides_to_a_known_rest_position_and_ends_exactly() {
    let fling = Decay::new(10.0_f32, 400.0);
    assert_eq!(fling.rest(), 10.0 + 400.0 / 4.0);
    testing::assert_exact_finish(&fling, &testing::FRAME_RATES);
    testing::assert_monotonic(&fling, &testing::FRAME_RATES, true);
    let length = fling.duration().unwrap();
    assert!(fling.sample(length - ms(1)).value < fling.rest());
    assert!(fling.sample(length).completed);

    let leftward = Decay::new(vec2(0.0, 0.0), vec2(-300.0, 120.0));
    testing::assert_exact_finish(&leftward, &testing::FRAME_RATES);
    assert_eq!(leftward.finish(), vec2(-75.0, 30.0));

    let half = Decay::with_options(0.0_f32, 100.0, DecayOptions::half_life(ms(500)));
    let k = std::f32::consts::LN_2 / 0.5;
    near(half.sample(ms(500)).value, 100.0 * 0.5 / k, 1.0e-3);
    assert!(Decay::new(5.0_f32, 0.0).sample(Duration::ZERO).completed);
}

#[test]
fn spring_duration_and_bounce_map_to_damping_ratio() {
    let options = SpringOptions::duration_bounce(ms(500), 0.3);
    assert_eq!(options, SpringOptions::frequency(2.0, 0.7));
    let overshoot = |bounce: f64| {
        let spring = Spring::with_options(
            0.0_f32,
            1.0,
            SpringOptions::duration_bounce(ms(500), bounce),
        );
        (0..400)
            .map(|i| spring.state(ms(i * 5)).value.value)
            .fold(f32::MIN, f32::max)
    };
    assert!(overshoot(0.4) > 1.05);
    assert!(overshoot(0.0) <= 1.0 + 1.0e-6, "critical never overshoots");
    assert!(
        overshoot(-0.5) <= 1.0 + 1.0e-6,
        "overdamped never overshoots"
    );
}

#[test]
#[should_panic(expected = "bounce")]
fn spring_bounce_of_one_is_rejected() {
    SpringOptions::duration_bounce(ms(500), 1.0);
}

#[test]
fn smooth_retarget_keeps_velocity_where_plain_retarget_reverses_at_once() {
    let mut ctx = Context::new();
    ctx.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let start = ctx.frame_time() + ms(1000);
    let motion = TweenOptions::new(ms(1000)).easing(Easing::Linear);
    let mut pass = |at: u64, target: f32| {
        let mut out = None;
        ctx.run_at(start + ms(at), |ctx| {
            let smooth = ctx.transition_smooth(Id::new("smooth"), target, motion.clone());
            let plain = ctx.transition(Id::new("plain"), target, motion.clone());
            out = Some((smooth, plain));
        });
        out.unwrap()
    };
    pass(0, 0.0);
    pass(1, 100.0);
    let (smooth, plain) = pass(501, 0.0);
    near(smooth.value, 50.0, 1.0e-3);
    near(plain.value, 50.0, 1.0e-3);

    // Velocity was +100 units/s: two milliseconds later the smooth value is
    // still climbing at that speed, the plain tween has already turned around.
    let (smooth, plain) = pass(503, 0.0);
    near(smooth.value, 50.2, 0.05);
    assert!(plain.value < 50.0);
    let (smooth, plain) = pass(551, 0.0);
    assert!(smooth.value > 53.0 && plain.value < 50.0);

    let (smooth, plain) = pass(1700, 0.0);
    assert_eq!((smooth.value, plain.value), (0.0, 0.0));
    assert!(smooth.completed() && smooth.just_completed);
}

#[test]
fn smooth_transition_works_for_vectors_and_starts_from_rest_unchanged() {
    let mut ctx = Context::new();
    ctx.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let start = ctx.frame_time() + ms(1000);
    let motion = TweenOptions::new(ms(100)).easing(Easing::Linear);
    let mut pass = |at: u64, target: Vec2| {
        let mut out = None;
        ctx.run_at(start + ms(at), |ctx| {
            out = Some(
                ctx.transition_smooth(Id::new("pos"), target, motion.clone())
                    .value,
            );
        });
        out.unwrap()
    };
    assert_eq!(pass(0, vec2(1.0, 2.0)), vec2(1.0, 2.0), "first call snaps");
    pass(1, vec2(11.0, 2.0));
    // From rest the configured easing applies; halfway is the midpoint.
    near(pass(51, vec2(11.0, 2.0)).x, 6.0, 1.0e-3);
    assert_eq!(pass(500, vec2(11.0, 2.0)), vec2(11.0, 2.0));
}
