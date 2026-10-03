use std::time::{Duration, Instant};
use zaxis::{animation::*, vec2, Context, Id, Vec2};
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn setup() -> (Context, Instant) {
    let c = Context::new();
    let t = c.frame_time() + ms(1000);
    (c, t)
}

#[test]
fn spring_is_physical_rate_independent_and_stable_after_large_gaps() {
    for ratio in [0.35, 1.0, 1.8] {
        let spring = Spring::with_options(
            vec2(0.0, 4.0),
            vec2(10.0, 0.0),
            SpringOptions::frequency(3.0, ratio),
        )
        .velocity(vec2(7.0, -2.0));
        let reference = spring.state(ms(500)).value;
        for hz in [30, 60, 144] {
            let mut previous = spring.state(Duration::ZERO).value;
            for frame in 1..=hz {
                let at = Duration::from_secs_f64(frame as f64 / hz as f64);
                let sample = spring.state(at);
                assert!(sample.value.value.is_finite() && sample.value.velocity.is_finite());
                if ratio >= 1.0 {
                    assert!(sample.value.value.x >= previous.value.x - 0.001);
                }
                previous = sample.value;
            }
            assert_eq!(spring.state(ms(500)).value, reference);
        }
        let end = spring.state(Duration::from_secs(86_400));
        assert!(end.completed);
        assert_eq!(
            end.value,
            SpringState {
                value: vec2(10.0, 0.0),
                velocity: Vec2::ZERO
            }
        );
        // Numerical derivative agrees with physical velocity away from settling.
        let at = ms(35);
        let h = Duration::from_micros(10);
        let dx = (spring.state(at + h).value.value - spring.state(at - h).value.value) / 0.00002;
        assert!((dx - spring.state(at).value.velocity).length() < 0.05);
    }
    let undamped = Spring::with_options(0.0_f64, 1.0, SpringOptions::frequency(1.0, 0.0));
    assert!(!undamped.state(Duration::from_secs(86_400)).completed);
    let snap =
        Spring::with_options(0.0_f32, 5.0, SpringOptions::frequency(0.0, 0.0)).velocity(10.0);
    assert_eq!(
        snap.state(Duration::ZERO).value,
        SpringState {
            value: 5.0,
            velocity: 0.0
        }
    );
}

#[test]
fn spring_retarget_preserves_position_and_velocity_with_shared_controls() {
    for hz in [30, 60, 144] {
        let (mut c, t) = setup();
        let id = Id::new("spring");
        let options = SpringOptions::frequency(2.0, 0.6);
        let mut last = None;
        c.run_at(t, |c| {
            c.spring_transition_from(
                id,
                SpringState {
                    value: 0.0_f64,
                    velocity: 3.0,
                },
                10.0,
                options,
            );
        });
        for frame in 1..hz / 4 {
            c.run_at(t + Duration::from_secs_f64(frame as f64 / hz as f64), |c| {
                c.spring_transition(id, 10.0_f64, options);
            });
        }
        c.run_at(t + ms(250), |c| {
            let a = c.spring_transition(id, 10.0_f64, options);
            let b = c.spring_transition(id, -5.0_f64, options);
            assert!((a.value.value - b.value.value).abs() < 1e-12);
            assert!((a.value.velocity - b.value.velocity).abs() < 1e-12);
            let same = c.spring_transition(id, -5.0_f64, options);
            assert_eq!(same.value, b.value);
            c.pause_animation(id);
            last = c.sample_animation::<SpringState<f64>>(id);
        });
        c.run_at(t + ms(20_000), |c| {
            assert_eq!(
                c.spring_transition(id, -5.0_f64, options).value,
                last.as_ref().unwrap().value
            );
        });
        assert_eq!(c.next_repaint(), None);
        c.run_at(t + ms(20_001), |c| {
            c.resume_animation(id);
            assert_eq!(
                c.sample_animation::<SpringState<f64>>(id).unwrap().value,
                last.unwrap().value
            );
        });
        c.run_at(t + ms(20_101), |c| {
            c.cancel_animation(id);
            assert_eq!(
                c.spring_transition(id, -5.0_f64, options).status,
                AnimationStatus::Cancelled
            );
        });
        assert!(!c.wants_animation_frame());
        c.run_at(t + ms(20_102), |c| {
            c.restart_animation(id, Spring::new(0.0_f64, 1.0));
            c.sample_animation::<SpringState<f64>>(id);
        });
        c.run_at(t + ms(40_000), |c| {
            let done = c.sample_animation::<SpringState<f64>>(id).unwrap();
            assert_eq!(
                done.value,
                SpringState {
                    value: 1.0,
                    velocity: 0.0
                }
            );
            assert!(done.just_completed);
            assert!(
                !c.sample_animation::<SpringState<f64>>(id)
                    .unwrap()
                    .just_completed
            );
        });
        assert_eq!(c.next_repaint(), None);
        c.run_at(t + ms(40_001), |_| {});
        assert_eq!(c.animation_status(id), None);
    }
}

#[test]
#[should_panic]
fn invalid_spring_parameters_are_rejected() {
    Spring::with_options(
        0.0_f32,
        1.0,
        SpringOptions {
            mass: 0.0,
            ..SpringOptions::default()
        },
    );
}

#[test]
fn compositions_use_residual_time_exact_boundaries_and_sleeping_delays() {
    let sequence = Sequence::new(0.0_f32)
        .then(Tween::new(0.0, 1.0, ms(100)))
        .delay(1.0, ms(500))
        .then(Tween::new(1.0, 3.0, ms(200)));
    assert_eq!(sequence.duration(), Some(ms(800)));
    assert_eq!(sequence.sample(ms(100)).value, 1.0);
    assert_eq!(sequence.sample(ms(100)).wake, Wake::After(ms(500)));
    assert_eq!(sequence.sample(ms(350)).wake, Wake::After(ms(250)));
    assert_eq!(sequence.sample(ms(700)).value, 2.0);
    assert_eq!(sequence.sample(ms(800)), AnimationSample::completed(3.0));
    assert!(sequence.sample(Duration::from_secs(86_400)).completed);
    assert!(Sequence::new(4).sample(Duration::ZERO).completed);
    let zeros = Sequence::new(0.0_f32)
        .then(Tween::new(0.0, 2.0, Duration::ZERO))
        .then(Tween::new(2.0, 3.0, Duration::ZERO));
    assert_eq!(zeros.sample(Duration::ZERO).value, 3.0);
    let infinite = Sequence::new(0.0_f32)
        .then(Rotation::new(ms(100)))
        .then(Tween::new(0.0, 1.0, ms(100)));
    assert_eq!(infinite.duration(), None);
    assert!(!infinite.sample(ms(10_000)).completed);
    let bounded = Sequence::new(0.0_f32)
        .then_for(ms(100), Delay::new(0.0, ms(10_000)))
        .then(Tween::new(0.0, 1.0, ms(100)));
    assert_eq!(bounded.sample(ms(50)).wake, Wake::After(ms(50)));
    assert_eq!(bounded.sample(ms(150)).value, 0.5);
    let parallel = Parallel::new()
        .with(Delay::new(0.0_f32, ms(100)))
        .with(Tween::new(1.0, 2.0, ms(200)));
    assert_eq!(parallel.sample(ms(100)).value, vec![0.0, 1.5]);
    assert!(!parallel.sample(ms(100)).completed);
    assert!(parallel.sample(ms(200)).completed);
    assert!(Parallel::<f32>::new().sample(Duration::ZERO).completed);
    assert!(
        !Parallel::new()
            .with(Rotation::new(ms(100)))
            .sample(ms(10_000))
            .completed
    );
}

#[test]
fn composition_controls_and_stagger_keep_identity_schedules() {
    let (mut c, t) = setup();
    let id = Id::new("sequence");
    c.run_at(t, |c| {
        c.animate(id, || {
            Sequence::new(0.0_f32)
                .delay(0.0, ms(500))
                .then(Tween::new(0.0, 1.0, ms(100)))
        });
    });
    assert_eq!(c.next_repaint(), Some(t + ms(500)));
    assert!(!c.wants_animation_frame());
    c.run_at(t + ms(100), |c| {
        c.pause_animation(id);
        c.sample_animation::<f32>(id);
    });
    c.run_at(t + ms(5000), |c| {
        c.resume_animation(id);
        c.sample_animation::<f32>(id);
    });
    assert_eq!(c.next_repaint(), Some(t + ms(5400)));
    c.run_at(t + ms(5500), |c| {
        assert!(c.sample_animation::<f32>(id).unwrap().just_completed);
    });
    assert_eq!(c.next_repaint(), None);
    let mut stagger = Stagger::new(ms(100)).max_delay(ms(240));
    for n in 0..100 {
        stagger.insert(
            Id::new(n),
            Duration::ZERO,
            Tween::new(0.0_f32, 1.0, ms(100)),
        );
    }
    let before = stagger.sample(ms(150));
    let ids: Vec<_> = (0..100).rev().map(Id::new).collect();
    stagger.reorder(&ids);
    let after = stagger.sample(ms(150));
    for (id, value) in before.value {
        assert_eq!(
            after.value.iter().find(|(key, _)| *key == id).unwrap().1,
            value
        );
    }
    assert!(stagger.sample(ms(340)).completed);
    stagger.remove(Id::new(0));
    stagger.insert(Id::new(100), ms(500), Tween::new(0.0_f32, 1.0, ms(100)));
    assert!(!stagger.sample(ms(600)).completed);
    assert!(stagger.sample(ms(840)).completed);
}

#[test]
fn decorative_cycles_are_continuous_at_wrap_and_stop_when_reduced_or_removed() {
    let rotation = Rotation::new(ms(1000));
    let a = rotation.sample(Duration::from_micros(999_999)).value;
    let b = rotation.sample(Duration::from_micros(1_000_001)).value;
    assert!((vec2(a.cos(), a.sin()) - vec2(b.cos(), b.sin())).length() < 0.0001);
    assert_eq!(rotation.sample(ms(1000)).value, 0.0);
    let pulse = Pulse::new(ms(1000));
    assert_eq!(pulse.sample(ms(1000)).value, 1.0);
    assert_eq!(pulse.sample(ms(500)).value, 0.65);
    let (mut c, t) = setup();
    let id = Id::new("spin");
    c.run_at(t, |c| {
        c.animate(id, || Rotation::new(ms(1000)));
    });
    assert!(c.wants_animation_frame());
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c.run_at(t + ms(1), |c| {
        assert_eq!(c.animate(id, || Rotation::new(ms(1000))).value, 0.0);
    });
    assert_eq!(c.next_repaint(), None);
    c.run_at(t + ms(2), |_| {});
    assert_eq!(c.animation_status(id), None);
}
