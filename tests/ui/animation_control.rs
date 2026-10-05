use crate::prelude::*;
use std::time::Duration;
use winit::dpi::PhysicalSize;
use zaxis::animation::{testing, *};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn setup() -> (Context, Instant) {
    let mut ctx = Context::new();
    ctx.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let start = ctx.frame_time() + ms(1000);
    (ctx, start)
}
fn linear() -> Tween<f32> {
    Tween::new(0.0, 1.0, ms(1000)).easing(Easing::Linear)
}
fn id() -> Id {
    Id::new("channel")
}
/// One pass at `start + at`: apply `control`, then read the channel.
fn step(
    ctx: &mut Context,
    start: Instant,
    at: u64,
    control: impl FnOnce(&mut Context),
) -> Animated<f32> {
    let mut out = None;
    ctx.run_at(start + ms(at), |ctx| {
        control(ctx);
        out = Some(ctx.animate(id(), linear));
    });
    out.unwrap()
}
fn near(value: f32, expected: f32) {
    assert!((value - expected).abs() < 1.0e-4, "{value} != {expected}");
}

#[test]
fn rate_keeps_position_and_changes_speed_from_now() {
    let (mut ctx, t) = setup();
    step(&mut ctx, t, 0, |_| {});
    near(step(&mut ctx, t, 250, |_| {}).value, 0.25);
    near(
        step(&mut ctx, t, 250, |c| {
            c.set_animation_rate(id(), 2.0);
        })
        .value,
        0.25,
    );
    near(step(&mut ctx, t, 500, |_| {}).value, 0.75);
    assert_eq!(ctx.animation_rate(id()), Some(2.0));
    let done = step(&mut ctx, t, 625, |_| {});
    assert!(done.completed() && done.just_completed);
    assert_eq!(done.value, 1.0);
}

#[test]
fn reverse_plays_back_to_the_start_pose_and_completes() {
    let (mut ctx, t) = setup();
    step(&mut ctx, t, 0, |_| {});
    near(
        step(&mut ctx, t, 500, |c| {
            c.reverse_animation(id());
        })
        .value,
        0.5,
    );
    near(step(&mut ctx, t, 700, |_| {}).value, 0.3);
    let done = step(&mut ctx, t, 1200, |_| {});
    assert!(done.completed() && done.just_completed);
    assert_eq!(done.value, 0.0, "reverse ends at the start pose");
}

#[test]
fn seek_scrubs_running_and_paused_channels() {
    let (mut ctx, t) = setup();
    step(&mut ctx, t, 0, |_| {});
    near(
        step(&mut ctx, t, 100, |c| {
            c.seek_animation(id(), ms(800));
        })
        .value,
        0.8,
    );
    step(&mut ctx, t, 200, |c| {
        c.pause_animation(id());
    });
    let paused = step(&mut ctx, t, 300, |c| {
        c.seek_animation(id(), ms(250));
    });
    near(paused.value, 0.25);
    assert_eq!(paused.status, AnimationStatus::Paused);
    assert_eq!(ctx.animation_elapsed(id()), Some(ms(250)));
    assert_eq!(ctx.animation_duration(id()), Some(ms(1000)));
    near(
        step(&mut ctx, t, 400, |c| {
            c.resume_animation(id());
        })
        .value,
        0.25,
    );
    near(step(&mut ctx, t, 500, |_| {}).value, 0.35);
    let past = step(&mut ctx, t, 510, |c| {
        c.seek_animation(id(), ms(5000));
    });
    assert!(past.completed() && past.just_completed);
    assert_eq!(past.value, 1.0);
    assert!(!ctx.seek_animation(Id::new("missing"), ms(1)));
}

#[test]
fn global_time_scale_slows_every_channel_without_a_jump() {
    let (mut ctx, t) = setup();
    step(&mut ctx, t, 0, |_| {});
    near(step(&mut ctx, t, 400, |_| {}).value, 0.4);
    let mut style = ctx.style().clone();
    style.motion.time_scale = 0.5;
    ctx.set_style(style);
    near(step(&mut ctx, t, 400, |_| {}).value, 0.4);
    near(step(&mut ctx, t, 1000, |_| {}).value, 0.7);
    assert!(step(&mut ctx, t, 2200, |_| {}).completed());
}

#[test]
fn sleeping_stage_wakes_after_wall_time_not_track_time() {
    let (mut ctx, t) = setup();
    let pass = |ctx: &mut Context, at, control: bool| {
        ctx.run_at(t + ms(at), |ctx| {
            ctx.animate(id(), || Delay::new(0.0_f32, ms(1000)));
            if control {
                ctx.set_animation_rate(id(), 0.5);
            }
            ctx.sample_animation::<f32>(id());
        });
    };
    pass(&mut ctx, 0, false);
    pass(&mut ctx, 100, true);
    assert_eq!(ctx.next_repaint(), Some(t + ms(1900)));
}

#[test]
#[should_panic(expected = "nonzero")]
fn zero_rate_is_rejected() {
    let (mut ctx, _) = setup();
    ctx.set_animation_rate(id(), 0.0);
}

#[test]
fn builtin_tracks_satisfy_the_testing_contract() {
    let rates = testing::FRAME_RATES;
    testing::assert_exact_finish(&linear(), &rates);
    testing::assert_monotonic(&linear(), &rates, true);
    testing::assert_exact_finish(
        &Sequence::new(0.0_f32)
            .then(Tween::new(0.0, 1.0, ms(300)).easing(Easing::CubicInOut))
            .delay(1.0, ms(100))
            .then(Tween::new(1.0, 0.0, ms(300)).easing(Easing::SineOut)),
        &rates,
    );
    testing::assert_exact_finish(
        &Keyframes::new([
            Keyframe::new(Duration::ZERO, 0.0_f32),
            Keyframe::new(ms(200), 1.0).easing(Easing::QuintOut),
            Keyframe::new(ms(500), 0.25),
        ]),
        &rates,
    );
    testing::assert_exact_finish(
        &Parallel::new()
            .with(Tween::new(0.0_f32, 1.0, ms(200)))
            .with(Tween::new(0.0_f32, 1.0, ms(900))),
        &rates,
    );
}
