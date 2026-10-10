use super::*;

#[test]
fn public_round_trip_negative_coordinates_and_zoom_anchor_at_every_dpi() {
    for dpi in [1.0, 1.25, 1.5, 2.0] {
        for scale in [0.5, 1.0, 2.0, 8.0] {
            let mut c = setup(dpi);
            let mut state = PanZoomState {
                scale,
                translation: vec2(70.0, 55.0),
            };
            let before = empty(&mut c, &mut state);
            for local in [vec2(-40.0, -20.0), Vec2::ZERO, vec2(15.0, 10.0)] {
                near(
                    before.screen_to_local(&c, before.local_to_screen(&c, local)),
                    local,
                );
            }
            for anchor in [vec2(1.0, 1.0), vec2(200.0, 150.0), vec2(399.0, 299.0)] {
                let local = empty(&mut c, &mut state).screen_to_local(&c, anchor);
                assert!(wheel(&mut c, anchor, -100.0, ModifiersState::CONTROL));
                let out = empty(&mut c, &mut state);
                near(out.local_to_screen(&c, local), anchor);
                assert_eq!(out.zoomed, out.changed);
            }
        }
    }
}

#[test]
fn pan_uses_screen_displacement_independent_of_camera_zoom_and_captures_outside() {
    for dpi in [1.0, 1.25, 1.5, 2.0] {
        for scale in [0.5, 1.0, 2.0] {
            let mut c = setup(dpi);
            let mut state = PanZoomState {
                scale,
                translation: vec2(20.0, -10.0),
            };
            empty(&mut c, &mut state);
            pointer(&mut c, vec2(100.0, 100.0));
            assert!(button(&mut c, ElementState::Pressed).consumed);
            assert!(pointer(&mut c, vec2(520.0, 400.0)).consumed);
            let out = empty(&mut c, &mut state);
            near(state.translation, vec2(440.0, 290.0));
            assert!(out.panned && out.changed && out.response.dragged());
            assert!(button(&mut c, ElementState::Released).consumed);
            let out = empty(&mut c, &mut state);
            assert!(out.response.drag_stopped());
            let out = empty(&mut c, &mut state);
            assert!(!out.changed && !out.response.dragged());
            assert!(!c.needs_repaint() && !c.wants_animation_frame());
        }
    }
}

#[test]
fn fit_reset_limits_invalid_inputs_and_external_mutation() {
    let mut state = PanZoomState::default();
    let content = Rect::from_min_size(vec2(-100.0, -50.0), vec2(200.0, 100.0));
    assert!(state.fit(content, vec2(400.0, 300.0), 20.0, 0.1..=8.0));
    assert_eq!(state.scale, 1.8);
    near(
        state.local_to_viewport(content.center()),
        vec2(200.0, 150.0),
    );
    assert!(!state.fit(content, Vec2::ZERO, 0.0, 0.1..=8.0));
    assert!(!state.fit(Rect::default(), vec2(400.0, 300.0), 0.0, 0.1..=8.0));
    assert!(!state.fit(content, vec2(20.0, 20.0), 20.0, 0.1..=8.0));
    state.zoom_at(99.0, vec2(10.0, 20.0), 0.5..=2.0);
    assert_eq!(state.scale, 2.0);
    state.zoom_at(0.001, vec2(10.0, 20.0), 0.5..=2.0);
    assert_eq!(state.scale, 0.5);
    state.reset();
    assert_eq!(state, PanZoomState::default());
    let mut c = setup(1.0);
    empty(&mut c, &mut state);
    state.scale = 2.0;
    state.translation = vec2(15.0, 35.0);
    assert!(empty(&mut c, &mut state).changed);
    assert!(!empty(&mut c, &mut state).changed);
    state.scale = f32::NAN;
    state.translation = Vec2::splat(f32::INFINITY);
    assert!(empty(&mut c, &mut state).changed);
    assert_eq!(state, PanZoomState::default());
    state.translation = Vec2::splat(f32::MAX);
    empty(&mut c, &mut state);
    assert!(state.translation.abs().max_element() <= 1.0e6);
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidValue));
    assert!(!state.fit(
        Rect {
            min: Vec2::splat(f32::NEG_INFINITY),
            max: Vec2::ZERO
        },
        Vec2::ONE,
        0.0,
        0.1..=8.0
    ));
}

#[test]
fn resize_and_empty_viewport_do_not_move_the_local_origin_or_grow_parent_layout() {
    let mut c = setup(1.0);
    let mut state = PanZoomState::default();
    for size in [vec2(320.0, 200.0), Vec2::ZERO, vec2(420.0, 250.0)] {
        let mut calls = 0;
        let mut out = None;
        let mut next = Rect::default();
        c.run(|c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                ui.add_space(50.0);
                out = Some(PanZoom::new("scene", size).show(ui, &mut state, |ui, _| {
                    calls += 1;
                    ui.at(
                        "negative",
                        Rect::from_min_size(vec2(-70.0, -50.0), vec2(100.0, 70.0)),
                        |ui| ui.button("Local"),
                    );
                    ui.paint(Shape::rect(
                        Rect::from_min_size(vec2(50000.0, 20000.0), Vec2::ONE),
                        Color::WHITE,
                    ));
                }));
                next = ui.allocate_space(Vec2::ONE);
            })
        });
        let out = out.unwrap();
        assert_eq!(calls, 1);
        assert_eq!(out.viewport.size(), size);
        assert!(next.min.y < out.viewport.max.y + 30.0);
        near(out.local_to_screen(&c, Vec2::ZERO), out.viewport.min);
    }
}
