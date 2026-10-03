use super::*;

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
