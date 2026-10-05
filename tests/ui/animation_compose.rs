use crate::prelude::*;
use std::time::Duration;
use winit::dpi::PhysicalSize;
use zaxis::{
    animation::{testing, *},
    impl_interpolate, impl_spring_value, vec2, Color, Transform, Vec2, Window,
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

#[test]
fn parallel_offsets_hold_the_start_value_then_run_and_finish_exactly() {
    let group = Parallel::new()
        .with(Tween::new(0.0_f32, 1.0, ms(200)).easing(Easing::Linear))
        .at(
            ms(300),
            Tween::new(10.0_f32, 20.0, ms(400)).easing(Easing::Linear),
        );
    assert_eq!(group.duration(), Some(ms(700)));

    let early = group.sample(ms(100));
    assert_eq!(early.value, vec![0.5, 10.0]);
    assert!(!early.completed);
    let waiting = group.sample(ms(250));
    assert_eq!(waiting.value, vec![1.0, 10.0]);
    assert_eq!(waiting.wake, Wake::After(ms(50)), "sleeps until the offset");
    assert_eq!(group.sample(ms(500)).value, vec![1.0, 15.0]);
    assert!(group.sample(ms(700)).completed);
    testing::assert_exact_finish(&group, &testing::FRAME_RATES);
}

#[test]
fn sequence_marks_name_moments_and_reject_unknown_lengths() {
    let sequence = Sequence::new(0.0_f32)
        .mark("start")
        .then(Tween::new(0.0, 1.0, ms(300)))
        .mark("peak")
        .delay(1.0, ms(100))
        .then(Tween::new(1.0, 0.0, ms(200)))
        .mark("end");
    assert_eq!(sequence.time_of("start"), Some(Duration::ZERO));
    assert_eq!(sequence.time_of("peak"), Some(ms(300)));
    assert_eq!(sequence.time_of("end"), Some(ms(600)));
    assert_eq!(sequence.time_of("missing"), None);
    assert_eq!(sequence.duration(), sequence.time_of("end"));
    // A mark can place a parallel track.
    let peak = sequence.time_of("peak").unwrap();
    let both = Parallel::new()
        .with(sequence)
        .at(peak, Tween::new(0.0_f32, 5.0, ms(100)));
    testing::assert_exact_finish(&both, &testing::FRAME_RATES);
}

#[test]
#[should_panic(expected = "known length")]
fn marking_after_an_unbounded_stage_panics() {
    let _ = Sequence::new(0.0_f32)
        .then(Procedural::new(0.0, |_| AnimationSample::running(0.0)))
        .mark("never");
}

#[test]
#[should_panic(expected = "duplicate")]
fn duplicate_marks_panic() {
    let _ = Sequence::new(0.0_f32).mark("a").mark("a");
}

#[test]
fn option_vec_tuple_and_transform_interpolate() {
    assert_eq!(Some(2.0_f32).interpolate(&Some(4.0), 0.5), Some(3.0));
    assert_eq!(None::<f32>.interpolate(&Some(4.0), 0.5), None);
    assert_eq!(None::<f32>.interpolate(&Some(4.0), 1.0), Some(4.0));
    assert_eq!(Some(4.0_f32).interpolate(&None, 0.99), Some(4.0));
    assert_eq!(
        vec![0.0_f32, 10.0].interpolate(&vec![2.0, 20.0], 0.5),
        vec![1.0, 15.0]
    );
    assert_eq!(vec![1.0_f32].interpolate(&vec![1.0, 2.0], 0.5), vec![1.0]);
    assert_eq!(
        vec![1.0_f32].interpolate(&vec![1.0, 2.0], 1.0),
        vec![1.0, 2.0]
    );
    assert_eq!(
        (0.0_f32, 0.0_f32, 0.0_f32).interpolate(&(2.0, 4.0, 6.0), 0.5),
        (1.0, 2.0, 3.0)
    );
    let a = Transform::IDENTITY;
    let b = Transform {
        scale: 3.0,
        translation: vec2(10.0, 0.0),
        angle: 1.0,
    };
    let mid = a.interpolate(&b, 0.5);
    assert_eq!(
        (mid.scale, mid.translation, mid.angle),
        (2.0, vec2(5.0, 0.0), 0.5)
    );
}

#[test]
fn oklab_keeps_exact_endpoints_and_blends_perceptually() {
    let (black, white) = (Oklab(Color::BLACK), Oklab(Color::WHITE));
    assert_eq!(black.interpolate(&white, 0.0), black);
    assert_eq!(black.interpolate(&white, 1.0), white);
    // Linear light puts mid grey at ~188; Oklab lightness 0.5 is ~99.
    let linear = Color::BLACK.interpolate(&Color::WHITE, 0.5).0[0];
    let perceptual = black.interpolate(&white, 0.5).0 .0[0];
    assert!(
        linear > 180 && (95..=104).contains(&perceptual),
        "{linear} {perceptual}"
    );
    // Round trip: nothing happens when both ends are the same color.
    let teal = Oklab(Color::rgb(30, 160, 150));
    let same = teal.interpolate(&teal, 0.37).0;
    assert!(same
        .0
        .iter()
        .zip(teal.0 .0)
        .all(|(a, b)| a.abs_diff(b) <= 1));
    let translucent =
        Oklab(Color::rgba(255, 0, 0, 0)).interpolate(&Oklab(Color::rgba(255, 0, 0, 200)), 0.5);
    assert_eq!(translucent.0 .0[3], 100);
    assert_eq!(Color::from(Oklab::from(Color::WHITE)), Color::WHITE);
}

#[derive(Clone, Debug, PartialEq)]
struct Pose {
    offset: Vec2,
    angle: f32,
}
impl_interpolate!(Pose { offset, angle });
impl_spring_value!(Pose { offset, angle });

#[test]
fn field_macros_implement_interpolate_and_spring_value() {
    let a = Pose {
        offset: Vec2::ZERO,
        angle: 0.0,
    };
    let b = Pose {
        offset: vec2(8.0, 0.0),
        angle: 2.0,
    };
    assert_eq!(
        a.interpolate(&b, 0.5),
        Pose {
            offset: vec2(4.0, 0.0),
            angle: 1.0
        }
    );
    assert_eq!(b.sub(&a).add(&a), b);
    near(
        Pose {
            offset: vec2(3.0, 0.0),
            angle: 4.0,
        }
        .norm() as f32,
        5.0,
        1.0e-6,
    );
    let spring = Spring::new(a.clone(), b.clone());
    assert!(!spring.state(ms(10)).completed);
    assert_eq!(spring.finish().value, b);
}

#[test]
fn path_is_parameterised_by_distance_with_direction_and_trimming() {
    let path = Path::new(vec2(0.0, 0.0))
        .line_to(vec2(100.0, 0.0))
        .line_to(vec2(100.0, 50.0))
        .line_to(vec2(100.0, 50.0));
    assert_eq!(path.length(), 150.0);
    assert_eq!(path.point_at(0.0), vec2(0.0, 0.0));
    assert_eq!(path.point_at(1.0), vec2(100.0, 50.0));
    near(path.point_at(0.5).x, 75.0, 1.0e-4);
    assert_eq!(path.angle_at(0.1), 0.0);
    near(path.angle_at(0.9), std::f32::consts::FRAC_PI_2, 1.0e-6);
    assert_eq!(path.trimmed(0.0, 0.0), vec![vec2(0.0, 0.0)]);
    let part = path.trimmed(0.5, 1.0);
    assert_eq!(part.first(), Some(&path.point_at(0.5)));
    assert_eq!(part.last(), Some(&vec2(100.0, 50.0)));
    assert!(
        part.contains(&vec2(100.0, 0.0)),
        "keeps the corner: {part:?}"
    );
    assert_eq!(Path::new(vec2(5.0, 5.0)).point_at(0.7), vec2(5.0, 5.0));
}

#[test]
fn curves_move_at_constant_speed_and_follow_exactly() {
    let path =
        Path::new(vec2(0.0, 0.0)).cubic_to(vec2(0.0, 200.0), vec2(300.0, -200.0), vec2(300.0, 0.0));
    assert!(path.length() > 300.0);
    let steps: Vec<f32> = (0..=40)
        .map(|i| i as f32 / 40.0)
        .collect::<Vec<_>>()
        .windows(2)
        .map(|pair| path.point_at(pair[0]).distance(path.point_at(pair[1])))
        .collect();
    let mean = steps.iter().sum::<f32>() / steps.len() as f32;
    assert!(
        steps.iter().all(|step| (step - mean).abs() < mean * 0.05),
        "{steps:?}"
    );

    let follow = PathFollow::new(path.clone(), ms(500)).easing(Easing::CubicInOut);
    assert_eq!(follow.finish().position, vec2(300.0, 0.0));
    testing::assert_exact_finish(&follow, &testing::FRAME_RATES);
    let pose = follow.sample(ms(250)).value;
    assert_eq!(pose, path.pose_at(Easing::CubicInOut.sample(0.5)));
}

#[test]
fn skeleton_shimmer_runs_only_while_visible_and_motion_is_allowed() {
    let frame = |c: &mut Context, at: Instant, y: f32| {
        c.run_at(at, |c| {
            Window::new("Loading")
                .default_position(vec2(10.0, y))
                .default_size(vec2(300.0, 120.0))
                .show(c, |ui| {
                    ui.skeleton(16.0);
                });
        });
    };
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let t = c.frame_time() + ms(1000);
    frame(&mut c, t, 10.0);
    assert!(c.next_repaint().is_some(), "shimmer keeps animating");

    let mut off = Context::new();
    off.set_viewport(PhysicalSize::new(800, 600), 1.0);
    frame(&mut off, t, 5000.0);
    assert_eq!(off.next_repaint(), None, "clipped placeholder sleeps");

    let mut still = Context::new();
    still.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut style = still.style().clone();
    style.motion.reduced_motion = true;
    still.set_style(style);
    frame(&mut still, t, 10.0);
    assert_eq!(still.next_repaint(), None, "reduced motion is static");
}
