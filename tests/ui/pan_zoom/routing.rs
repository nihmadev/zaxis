use super::*;

#[test]
fn ordered_zoom_events_keep_each_anchor_and_pan_interleaves_without_double_application() {
    let mut c = setup(1.0);
    let mut state = PanZoomState::default();
    empty(&mut c, &mut state);
    assert!(wheel(
        &mut c,
        vec2(50.0, 60.0),
        100.0,
        ModifiersState::CONTROL
    ));
    assert!(wheel(
        &mut c,
        vec2(200.0, 180.0),
        -100.0,
        ModifiersState::CONTROL
    ));
    c.on_input(InputEvent::Modifiers(ModifiersState::empty()));
    let out = empty(&mut c, &mut state);
    near(
        state.translation,
        (vec2(50.0, 60.0) - vec2(200.0, 180.0)) * ((-0.2_f32).exp() - 1.0),
    );
    assert!((state.scale - 1.0).abs() < 0.0001 && out.zoomed);
    assert!(!empty(&mut c, &mut state).changed);
    pointer(&mut c, vec2(50.0, 50.0));
    button(&mut c, ElementState::Pressed);
    pointer(&mut c, vec2(80.0, 60.0));
    let before = state;
    assert!(wheel(
        &mut c,
        vec2(80.0, 60.0),
        -100.0,
        ModifiersState::CONTROL
    ));
    pointer(&mut c, vec2(100.0, 70.0));
    button(&mut c, ElementState::Released);
    empty(&mut c, &mut state);
    let expected = (before.translation + vec2(30.0, 10.0) - vec2(80.0, 60.0)) * (-0.2_f32).exp()
        + vec2(80.0, 60.0)
        + vec2(20.0, 10.0);
    near(state.translation, expected);
}

#[test]
fn nested_scroll_receives_plain_wheel_zoom_reserves_modified_wheel_and_host_gets_unused() {
    let mut c = setup(1.0);
    let mut state = PanZoomState::default();
    let make = |c: &mut Context, state: &mut PanZoomState| {
        draw(c, state, |ui, _| {
            ui.at(
                "scroll",
                Rect::from_min_size(vec2(30.0, 30.0), vec2(250.0, 170.0)),
                |ui| {
                    ScrollArea::vertical()
                        .id_source("inner")
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for i in 0..30 {
                                ui.label(format!("Row {i}"));
                            }
                        })
                },
            )
        })
    };
    let out = make(&mut c, &mut state);
    let p = out.local_to_screen(&c, out.inner.viewport.center());
    assert!(wheel(&mut c, p, -30.0, ModifiersState::empty()));
    assert!(wheel(&mut c, p, -100.0, ModifiersState::CONTROL));
    let out = make(&mut c, &mut state);
    assert!(out.inner.offset.y > 0.0 && out.zoomed);
    let offset = out.inner.offset;
    assert!(wheel(&mut c, p, -100.0, ModifiersState::CONTROL));
    let out = make(&mut c, &mut state);
    assert!(out.zoomed);
    assert_eq!(out.inner.offset, offset);
    assert!(!wheel(
        &mut c,
        vec2(390.0, 290.0),
        -20.0,
        ModifiersState::empty()
    ));
}

#[test]
fn child_slider_text_and_custom_drags_keep_capture_and_response_screen_units() {
    let mut c = setup(1.0);
    let mut state = PanZoomState {
        scale: 2.0,
        translation: Vec2::ZERO,
    };
    let mut value = 0.0_f32;
    let make = |c: &mut Context, camera: &mut PanZoomState, value: &mut f32| {
        draw(c, camera, |ui, _| {
            let slider = ui.at(
                "slider",
                Rect::from_min_size(vec2(10.0, 10.0), vec2(150.0, 40.0)),
                |ui| ui.add(Slider::new(value, 0.0..=1.0).width(140.0)),
            );
            let drag = ui.interact(
                Rect::from_min_size(vec2(10.0, 60.0), vec2(100.0, 40.0)),
                "drag",
                Sense::DRAG,
            );
            (slider, drag)
        })
    };
    let out = make(&mut c, &mut state, &mut value);
    let p = out.local_to_screen(&c, out.inner.0.rect.center());
    pointer(&mut c, p);
    button(&mut c, ElementState::Pressed);
    pointer(&mut c, vec2(270.0, 50.0));
    button(&mut c, ElementState::Released);
    let out = make(&mut c, &mut state, &mut value);
    assert!(value > 0.8 && !out.changed);
    let p = out.local_to_screen(&c, out.inner.1.rect.center());
    pointer(&mut c, p);
    button(&mut c, ElementState::Pressed);
    pointer(&mut c, p + vec2(25.0, 15.0));
    button(&mut c, ElementState::Released);
    let out = make(&mut c, &mut state, &mut value);
    assert!(!out.changed);
    near(out.inner.1.drag_delta(), vec2(25.0, 15.0));
}

#[test]
fn disabled_removed_focus_loss_clipped_and_bounded_queue_drop_transient_input() {
    let mut c = setup(1.0);
    let mut state = PanZoomState::default();
    empty(&mut c, &mut state);
    for _ in 0..300 {
        wheel(&mut c, vec2(100.0, 100.0), 1.0, ModifiersState::CONTROL);
    }
    assert_eq!(c.probe().camera_routing_counts.1, 256);
    pointer(&mut c, vec2(20.0, 20.0));
    button(&mut c, ElementState::Pressed);
    pointer(&mut c, vec2(40.0, 40.0));
    assert!(c.probe().camera_routing_counts.1 <= 257);
    c.on_input(InputEvent::Focus(false));
    assert!(c.probe().capture.is_none());
    assert_eq!(c.probe().camera_routing_counts.1, 0);
    empty(&mut c, &mut state);
    assert_eq!(state, PanZoomState::default());
    c.on_input(InputEvent::Focus(true));
    assert!(wheel(
        &mut c,
        vec2(100.0, 100.0),
        10.0,
        ModifiersState::CONTROL
    ));
    c.run(|c| {
        Root::new().padding(Padding::all(0.0)).show(c, |ui| {
            PanZoom::new("scene", vec2(400.0, 300.0))
                .enabled(false)
                .show(ui, &mut state, |_, _| {});
        })
    });
    assert_eq!(state, PanZoomState::default());
    assert!(!wheel(
        &mut c,
        vec2(100.0, 100.0),
        10.0,
        ModifiersState::CONTROL
    ));
    empty(&mut c, &mut state);
    pointer(&mut c, vec2(20.0, 20.0));
    button(&mut c, ElementState::Pressed);
    c.run(|_| {});
    assert!(c.probe().capture.is_none());
    assert_eq!(c.probe().camera_routing_counts, (0, 0));
    button(&mut c, ElementState::Released);
    for i in 0..100 {
        c.run(|c| {
            Root::new().show(c, |ui| {
                ui.at(
                    "hidden",
                    Rect::from_min_size(vec2(1000.0, 1000.0), vec2(100.0, 100.0)),
                    |ui| {
                        PanZoom::new(i, vec2(100.0, 100.0)).show(ui, &mut state, |_, _| {});
                    },
                );
            })
        });
        assert!(c.probe().camera_routing_counts.0 <= 1);
    }
    c.run(|_| {});
    assert_eq!(c.probe().camera_routing_counts, (0, 0));
}
