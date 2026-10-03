use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};
use zaxis::{animation::*, vec2, Color, Context, Id, Vec2};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn clock() -> (Context, Instant) {
    let context = Context::new();
    let now = context.frame_time() + ms(1000);
    (context, now)
}
fn read(ctx: &mut Context, now: Instant, id: Id) -> Animated<f32> {
    let mut value = None;
    ctx.run_at(now, |ctx| value = ctx.sample_animation(id));
    value.unwrap()
}

#[test]
fn frame_rates_and_long_suspension_have_identical_values_and_exact_endpoints() {
    for hz in [30, 60, 144] {
        let (mut ctx, start) = clock();
        let id = Id::new("rate");
        ctx.run_at(start, |ctx| {
            ctx.animate(id, || Tween::new(2.0_f32, 9.0, ms(1000)));
        });
        for i in 1..hz / 2 {
            read(
                &mut ctx,
                start + Duration::from_secs_f64(i as f64 / hz as f64),
                id,
            );
        }
        assert_eq!(read(&mut ctx, start + ms(500), id).value, 5.5);
        let end = read(&mut ctx, start + ms(1000), id);
        assert_eq!(end.value, 9.0);
        assert!(end.completed() && end.just_completed);
        assert!(ctx.next_repaint().is_none());
        assert!(!read(&mut ctx, start + ms(100_000), id).just_completed);
    }
    let tween = Tween::new(vec2(2.0, 3.0), vec2(8.0, 9.0), ms(200));
    assert_eq!(tween.sample(ms(86_400_000)).value, vec2(8.0, 9.0));
}

#[test]
fn transitions_retarget_continuously_and_same_target_never_restarts() {
    let (mut ctx, start) = clock();
    let id = Id::new("color");
    let options = TweenOptions::new(ms(1000));
    let mut sample = None;
    let mut pass = |ctx: &mut Context, time, target| {
        ctx.run_at(start + ms(time), |ctx| {
            sample = Some(ctx.transition(id, target, options.clone()))
        });
        sample.take().unwrap()
    };
    assert_eq!(pass(&mut ctx, 0, 0.0_f32).value, 0.0);
    assert!(!pass(&mut ctx, 0, 0.0).just_completed);
    assert_eq!(pass(&mut ctx, 0, 10.0).value, 0.0);
    assert_eq!(pass(&mut ctx, 250, 10.0).value, 2.5);
    assert_eq!(pass(&mut ctx, 250, -10.0).value, 2.5);
    assert_eq!(pass(&mut ctx, 750, -10.0).value, -3.75);
    let end = pass(&mut ctx, 1250, -10.0);
    assert_eq!(end.value, -10.0);
    assert!(end.just_completed);
    assert!(!pass(&mut ctx, 10_000, -10.0).just_completed);
    assert!(ctx.next_repaint().is_none());
}

#[test]
fn delay_sleeps_until_deadline_and_zero_duration_finishes_once() {
    let (mut ctx, start) = clock();
    let id = Id::new("delayed");
    ctx.run_at(start, |ctx| {
        let value = ctx.animate(id, || {
            Tween::new(3.0_f32, 7.0, Duration::ZERO)
                .delay(ms(500))
                .repeat(Repeat::Forever)
        });
        assert_eq!(value.value, 3.0);
        assert!(value.running());
    });
    assert_eq!(ctx.next_repaint(), Some(start + ms(500)));
    assert!(!ctx.needs_repaint_at(start + ms(499)));
    assert!(ctx.needs_repaint_at(start + ms(500)));
    assert_eq!(read(&mut ctx, start + ms(250), id).value, 3.0);
    assert_eq!(ctx.next_repaint(), Some(start + ms(500)));
    let end = read(&mut ctx, start + ms(500), id);
    assert_eq!(end.value, 7.0);
    assert!(end.just_completed);
    assert!(ctx.next_repaint().is_none());
    ctx.run_at(start + ms(501), |ctx| {
        assert!(!ctx.sample_animation::<f32>(id).unwrap().just_completed);
        assert!(!ctx.sample_animation::<f32>(id).unwrap().just_completed);
    });
}

#[test]
fn repeat_and_reverse_boundaries_use_complete_cycles_without_frame_loops() {
    let t = Tween::new(0.0_f32, 10.0, ms(100))
        .repeat(Repeat::Count(2))
        .auto_reverse(true);
    for (at, value, completed) in [
        (0, 0.0, false),
        (50, 5.0, false),
        (100, 10.0, false),
        (150, 5.0, false),
        (200, 0.0, false),
        (300, 10.0, false),
        (400, 0.0, true),
    ] {
        let s = t.sample(ms(at));
        assert_eq!((s.value, s.completed), (value, completed));
    }
    let repeat = Tween::new(1.0_f32, 2.0, ms(100)).repeat(Repeat::Count(2));
    assert_eq!(repeat.sample(ms(100)).value, 1.0);
    assert_eq!(repeat.sample(ms(200)).value, 2.0);
    assert!(repeat.sample(ms(200)).completed);
    assert!(
        !Tween::new(0.0_f32, 1.0, ms(100))
            .repeat(Repeat::Forever)
            .sample(ms(1_000_050))
            .completed
    );
}

#[test]
fn keyframes_apply_arriving_segment_easing_and_exact_knots() {
    let keys = Keyframes::new([
        Keyframe::new(ms(0), 0.0_f32),
        Keyframe::new(ms(100), 10.0).easing(Easing::QuadOut),
        Keyframe::new(ms(300), 20.0).easing(Easing::QuintIn),
    ])
    .delay(ms(50));
    assert_eq!(keys.sample(ms(0)).value, 0.0);
    assert_eq!(keys.sample(ms(100)).value, 7.5);
    assert_eq!(keys.sample(ms(150)).value, 10.0);
    assert_eq!(keys.sample(ms(250)).value, 10.3125);
    assert_eq!(keys.sample(ms(350)).value, 20.0);
    assert!(keys.sample(ms(350)).completed);
    let reverse = Keyframes::new([Keyframe::new(ms(0), 4.0_f32), Keyframe::new(ms(100), 8.0)])
        .auto_reverse(true);
    assert_eq!(reverse.sample(ms(150)).value, 6.0);
    assert_eq!(reverse.sample(ms(200)).value, 4.0);
}

#[test]
fn pause_preserves_delay_resume_excludes_paused_time_and_cancel_is_terminal() {
    let (mut ctx, start) = clock();
    let id = Id::new("pause");
    ctx.run_at(start, |ctx| {
        ctx.animate(id, || Tween::new(0.0_f32, 10.0, ms(1000)).delay(ms(200)));
    });
    ctx.run_at(start + ms(100), |ctx| {
        assert!(ctx.pause_animation(id));
        assert_eq!(
            ctx.sample_animation::<f32>(id).unwrap().status,
            AnimationStatus::Paused
        );
    });
    assert!(ctx.next_repaint().is_none());
    ctx.run_at(start + ms(10_100), |ctx| {
        ctx.resume_animation(id);
        ctx.sample_animation::<f32>(id);
    });
    assert_eq!(ctx.next_repaint(), Some(start + ms(10_200)));
    assert_eq!(read(&mut ctx, start + ms(10_700), id).value, 5.0);
    ctx.run_at(start + ms(10_800), |ctx| {
        ctx.cancel_animation(id);
        let s = ctx.animate(id, panic_track);
        assert_eq!(s.status, AnimationStatus::Cancelled);
        assert_eq!(s.value, 6.0);
        assert!(!s.just_completed);
        ctx.resume_animation(id);
    });
    assert!(ctx.next_repaint().is_none());
    ctx.run_at(start + ms(20_000), |ctx| {
        ctx.restart_animation(id, Tween::new(3.0_f32, 9.0, Duration::ZERO));
        let s = ctx.sample_animation::<f32>(id).unwrap();
        assert_eq!(s.value, 9.0);
        assert!(s.just_completed);
    });
}
fn panic_track() -> Tween<f32> {
    panic!("factory restarted a retained channel")
}

#[test]
fn paused_retarget_and_explicit_finish_keep_exact_values() {
    let (mut ctx, start) = clock();
    let id = Id::new("retarget-paused");
    ctx.run_at(start, |ctx| {
        ctx.transition_from(id, 0.0_f32, 10.0, TweenOptions::new(ms(1000)));
    });
    ctx.run_at(start + ms(300), |ctx| {
        ctx.pause_animation(id);
        ctx.sample_animation::<f32>(id);
    });
    ctx.run_at(start + ms(5000), |ctx| {
        assert_eq!(
            ctx.transition(id, 20.0_f32, TweenOptions::new(ms(1000)))
                .value,
            3.0
        );
        ctx.pause_animation(id);
        ctx.finish_animation(id);
        let s = ctx.sample_animation::<f32>(id).unwrap();
        assert_eq!(s.value, 20.0);
        assert!(s.just_completed);
        ctx.finish_animation(id);
        assert!(!ctx.sample_animation::<f32>(id).unwrap().just_completed);
    });
    assert!(ctx.next_repaint().is_none());
}

#[test]
fn disappearing_channels_remove_deadlines_and_reappearing_transitions_snap() {
    let (mut ctx, start) = clock();
    let id = Id::new("gone");
    ctx.run_at(start, |ctx| {
        ctx.transition_from(
            id,
            0.0_f32,
            10.0,
            TweenOptions::new(ms(200)).delay(ms(1000)),
        );
    });
    assert_eq!(ctx.next_repaint(), Some(start + ms(1000)));
    ctx.run_at(start + ms(10), |_| {});
    assert_eq!(ctx.animation_status(id), None);
    assert_eq!(ctx.next_repaint(), None);
    assert!(!ctx.needs_repaint_at(start + ms(10000)));
    ctx.run_at(start + ms(10000), |ctx| {
        let s = ctx.transition(id, 10.0_f32, TweenOptions::new(ms(200)));
        assert_eq!(s.value, 10.0);
        assert!(s.completed() && !s.just_completed);
    });
}

#[test]
fn animation_deadlines_do_not_destroy_unrelated_timers() {
    let (mut ctx, start) = clock();
    let id = Id::new("timer");
    ctx.run_at(start, |ctx| {
        ctx.request_repaint_after(ms(10_000));
        ctx.animate(id, || Tween::new(0.0_f32, 1.0, ms(1000)));
    });
    assert_eq!(ctx.next_repaint(), Some(start + ms(16)));
    ctx.cancel_animation(id);
    assert_eq!(ctx.next_repaint(), Some(start + ms(10_000)));
}

#[test]
fn tiny_delays_are_exact_and_cancellation_at_endpoint_emits_no_completion() {
    let (mut ctx, start) = clock();
    let id = Id::new("tiny");
    ctx.run_at(start, |ctx| {
        ctx.animate(id, || {
            Tween::new(0.0_f32, 1.0, ms(100)).delay(Duration::from_nanos(7))
        });
    });
    assert_eq!(ctx.next_repaint(), Some(start + Duration::from_nanos(7)));
    ctx.run_at(start + ms(101), |ctx| {
        ctx.cancel_animation(id);
        let s = ctx.sample_animation::<f32>(id).unwrap();
        assert_eq!(s.status, AnimationStatus::Cancelled);
        assert!(!s.just_completed);
    });
    assert_eq!(ctx.next_repaint(), None);
}

#[test]
fn reduced_motion_switch_removes_deadlines_immediately_in_the_same_pass() {
    let (mut ctx, start) = clock();
    let id = Id::new("toggle");
    ctx.run_at(start, |ctx| {
        ctx.animate(id, || Tween::new(0.0_f32, 1.0, ms(1000)));
        let mut style = ctx.style().clone();
        style.motion.reduced_motion = true;
        ctx.set_style(style);
        assert_eq!(ctx.next_repaint(), None);
        let s = ctx.sample_animation::<f32>(id).unwrap();
        assert_eq!(s.value, 1.0);
        assert!(s.just_completed);
    });
}

#[test]
fn vsync_frame_requests_exclude_delay_pause_completion_and_disappearance() {
    let (mut ctx, start) = clock();
    let id = Id::new("vsync");
    ctx.run_at(start, |ctx| {
        ctx.animate(id, || Tween::new(0.0_f32, 1.0, ms(100)).delay(ms(200)));
    });
    assert!(!ctx.wants_animation_frame());
    read(&mut ctx, start + ms(200), id);
    assert!(ctx.wants_animation_frame());
    ctx.pause_animation(id);
    assert!(!ctx.wants_animation_frame());
    ctx.run_at(start + ms(210), |ctx| {
        ctx.resume_animation(id);
        ctx.sample_animation::<f32>(id);
    });
    assert!(ctx.wants_animation_frame());
    read(&mut ctx, start + ms(310), id);
    assert!(!ctx.wants_animation_frame());
    ctx.run_at(start + ms(311), |_| {});
    assert!(!ctx.wants_animation_frame());
}

#[test]
fn one_time_per_pass_and_backwards_clock_clamping() {
    let (mut ctx, start) = clock();
    let id = Id::new("clock");
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    ctx.run_at(start, |ctx| {
        ctx.animate(id, || {
            Procedural::new(1.0_f32, move |elapsed: Duration| {
                calls.set(calls.get() + 1);
                AnimationSample::running(elapsed.as_secs_f32())
            })
        });
        ctx.sample_animation::<f32>(id);
    });
    assert_eq!(count.get(), 1);
    assert_eq!(read(&mut ctx, start + ms(500), id).value, 0.5);
    assert_eq!(read(&mut ctx, start + ms(100), id).value, 0.5);
    assert_eq!(count.get(), 2);
    assert_eq!(ctx.frame_time(), start + ms(500));
}

#[test]
fn reduced_motion_finishes_running_delayed_and_paused_decorative_tracks() {
    let (mut ctx, start) = clock();
    let id = Id::new("reduced");
    let essential = id.with("essential");
    ctx.run_at(start, |ctx| {
        ctx.animate(id, || {
            Tween::new(Color::BLACK, Color::WHITE, ms(1000)).delay(ms(500))
        });
        ctx.animate_with(essential, AnimationOptions { decorative: false }, || {
            Tween::new(0.0_f32, 1.0, ms(1000))
        });
        ctx.pause_animation(id);
    });
    let mut style = ctx.style().clone();
    style.motion.reduced_motion = true;
    ctx.set_style(style);
    ctx.run_at(start + ms(100), |ctx| {
        let s = ctx.sample_animation::<Color>(id).unwrap();
        assert_eq!(s.value, Color::WHITE);
        assert!(s.just_completed);
        assert!(ctx.sample_animation::<f32>(essential).unwrap().running());
    });
    ctx.run_at(start + ms(200), |ctx| {
        ctx.sample_animation::<Color>(id);
        ctx.cancel_animation(essential);
        ctx.sample_animation::<f32>(essential);
    });
    assert_eq!(ctx.next_repaint(), None);
}

#[test]
fn linear_light_color_and_composite_endpoints() {
    assert_eq!(
        Color::BLACK.interpolate(&Color::WHITE, 0.5),
        Color::gray(188)
    );
    assert_eq!(
        Color::rgba(255, 0, 0, 0).interpolate(&Color::rgba(255, 0, 0, 255), 0.5),
        Color::rgba(255, 0, 0, 128)
    );
    let target = (
        zaxis::Border::new(3.7, Color::rgba(17, 83, 171, 49)),
        vec2(11.3, 1.7),
    );
    let t = Tween::new((zaxis::Border::NONE, Vec2::ZERO), target, ms(37))
        .easing(Easing::custom(|t| t * t));
    assert_eq!(t.sample(ms(37)).value, target);
    assert_eq!(
        Tween::new(0.0_f64, 1.234567890123, ms(1))
            .sample(ms(1))
            .value,
        1.234567890123
    );
    for curve in [
        Easing::Linear,
        Easing::QuadIn,
        Easing::QuadOut,
        Easing::QuadInOut,
        Easing::CubicIn,
        Easing::CubicOut,
        Easing::CubicInOut,
        Easing::QuintIn,
        Easing::QuintOut,
        Easing::QuintInOut,
        Easing::SineIn,
        Easing::SineOut,
        Easing::SineInOut,
    ] {
        assert_eq!(curve.sample(0.0), 0.0);
        assert_eq!(curve.sample(1.0), 1.0);
        let mut previous = 0.0;
        for i in 1..=100 {
            let value = curve.sample(i as f32 / 100.0);
            assert!(value >= previous);
            previous = value;
        }
    }
}

// This type and spring live entirely outside zaxis; no private enum registration.
#[derive(Clone, Debug, PartialEq)]
struct Dial {
    angle: f32,
    radius: f32,
}
impl Interpolate for Dial {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        Self {
            angle: self.angle.interpolate(&to.angle, t),
            radius: self.radius.interpolate(&to.radius, t),
        }
    }
}
struct Spring {
    target: Dial,
}
impl Animation<Dial> for Spring {
    fn sample(&self, elapsed: Duration) -> AnimationSample<Dial> {
        if elapsed >= ms(1000) {
            return AnimationSample::completed(self.finish());
        }
        let t = elapsed.as_secs_f32();
        AnimationSample::running(Dial {
            angle: self.target.angle * (1.0 - (-8.0 * t).exp() * (12.0 * t).cos()),
            radius: self.target.radius,
        })
    }
    fn finish(&self) -> Dial {
        self.target.clone()
    }
}
#[test]
fn external_type_curve_and_track_work_through_public_api() {
    let (mut ctx, start) = clock();
    let id = Id::new("external");
    ctx.run_at(start, |ctx| {
        ctx.animate(id, || Spring {
            target: Dial {
                angle: 2.0,
                radius: 7.0,
            },
        });
        ctx.animate(id.with("tween"), || {
            Tween::new(
                Dial {
                    angle: 0.0,
                    radius: 3.0,
                },
                Dial {
                    angle: 1.0,
                    radius: 7.0,
                },
                ms(500),
            )
            .easing(Easing::custom(|t| t * t * (3.0 - 2.0 * t)))
        });
    });
    ctx.run_at(start + ms(1000), |ctx| {
        let s = ctx.animate(id, || -> Spring { panic!("factory called again") });
        assert_eq!(
            s.value,
            Dial {
                angle: 2.0,
                radius: 7.0
            }
        );
        assert!(s.just_completed);
        assert!(ctx
            .sample_animation::<Dial>(id.with("tween"))
            .unwrap()
            .completed());
    });
    assert!(ctx.next_repaint().is_none());
}
