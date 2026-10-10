use super::*;

#[test]
fn public_input_transform_moves_an_application_object_with_the_pointer_at_every_zoom() {
    for dpi in [1.0, 1.25, 1.5, 2.0] {
        for scale in [0.1, 0.5, 1.0, 2.0, 8.0] {
            let mut c = setup(dpi);
            let mut state = PanZoomState {
                scale,
                translation: vec2(100.0, 100.0),
            };
            let mut center = Vec2::ZERO;
            let make = |c: &mut Context, state: &mut PanZoomState, center: &mut Vec2| {
                draw(c, state, |ui, _| {
                    let rect = Rect::from_min_size(*center - Vec2::splat(10.0), Vec2::splat(20.0));
                    let response = ui.interact(rect, "object", Sense::DRAG);
                    if response.dragged() {
                        *center += ui
                            .context()
                            .input_transform(response.id)
                            .inverse()
                            .vector(response.drag_delta());
                    }
                    ui.paint(Shape::rect(
                        Rect::from_min_size(*center - Vec2::splat(10.0), Vec2::splat(20.0)),
                        Color::WHITE,
                    ));
                    response
                })
            };
            make(&mut c, &mut state, &mut center);
            let origin = vec2(100.0, 100.0);
            pointer(&mut c, origin);
            assert!(button(&mut c, ElementState::Pressed).consumed);
            for step in 1..=5 {
                let point = origin + vec2(15.0, 8.0) * step as f32;
                assert!(pointer(&mut c, point).consumed);
                let out = make(&mut c, &mut state, &mut center);
                near(out.local_to_screen(&c, center), point);
                assert!(!out.changed);
            }
            button(&mut c, ElementState::Released);
            make(&mut c, &mut state, &mut center);
        }
    }
}

#[test]
fn visual_inside_negative_local_rect_keeps_paint_hit_and_click_in_the_same_units() {
    let mut c = setup(1.5);
    let mut state = PanZoomState {
        scale: 2.0,
        translation: vec2(200.0, 180.0),
    };
    let make = |c: &mut Context, state: &mut PanZoomState| {
        draw(c, state, |ui, _| {
            ui.at(
                "negative",
                Rect::from_min_size(vec2(-80.0, -70.0), vec2(140.0, 50.0)),
                |ui| {
                    ui.visual(
                        "inner",
                        Transform::around(Vec2::ZERO, 0.75, Vec2::ZERO),
                        1.0,
                        |ui| ui.button("Click"),
                    )
                },
            )
        })
    };
    let out = make(&mut c, &mut state);
    let hit = c
        .probe()
        .previous_hits
        .iter()
        .find(|hit| hit.id == out.inner.id)
        .unwrap();
    assert!(!hit.clip.intersect(hit.rect).is_empty());
    let p = hit.rect.intersect(hit.clip).center();
    pointer(&mut c, p);
    assert!(button(&mut c, ElementState::Pressed).consumed);
    button(&mut c, ElementState::Released);
    let out = make(&mut c, &mut state);
    assert!(out.inner.clicked() && !out.changed);
}

#[test]
fn scrolling_a_camera_ancestor_invalidates_its_geometry_until_publication() {
    let mut c = setup(1.0);
    let mut state = PanZoomState::default();
    let make = |c: &mut Context, state: &mut PanZoomState| {
        let mut out = None;
        c.run(|c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                ScrollArea::vertical()
                    .id_source("parent")
                    .max_height(350.0)
                    .show(ui, |ui| {
                        out = Some(PanZoom::new("scene", vec2(300.0, 200.0)).show(
                            ui,
                            state,
                            |ui, _| {
                                ui.at(
                                    "object",
                                    Rect::from_min_size(vec2(10.0, 150.0), vec2(120.0, 40.0)),
                                    |ui| ui.button("Object"),
                                )
                            },
                        ));
                        ui.allocate_space(vec2(300.0, 1000.0));
                    });
            })
        });
        out.unwrap()
    };
    make(&mut c, &mut state);
    assert!(wheel(
        &mut c,
        vec2(20.0, 250.0),
        -50.0,
        ModifiersState::empty()
    ));
    wheel(&mut c, vec2(20.0, 100.0), -60.0, ModifiersState::CONTROL);
    assert_eq!(
        c.probe().camera_routing_counts.1,
        0,
        "stale moved viewport must not own zoom"
    );
    let out = make(&mut c, &mut state);
    assert_eq!(state, PanZoomState::default());
    let screen = c.visual_rect(out.inner.id, out.inner.rect);
    assert!(out.viewport.min.y < -100.0);
    near(screen.min, out.viewport.min + out.inner.rect.min);
}

#[test]
fn resize_and_dpi_events_cancel_capture_and_queued_camera_input() {
    let mut c = setup(1.0);
    let mut state = PanZoomState::default();
    for dpi in [1.0, 1.25, 1.5, 2.0] {
        empty(&mut c, &mut state);
        pointer(&mut c, vec2(50.0, 50.0));
        button(&mut c, ElementState::Pressed);
        pointer(&mut c, vec2(80.0, 80.0));
        assert!(c.probe().capture.is_some());
        c.on_input(InputEvent::ScaleFactor(dpi));
        c.on_input(InputEvent::Resized {
            width: (800.0 * dpi) as u32,
            height: (600.0 * dpi) as u32,
        });
        assert!(c.probe().capture.is_none());
        assert_eq!(c.probe().camera_routing_counts, (0, 0));
        button(&mut c, ElementState::Released);
        let out = empty(&mut c, &mut state);
        assert!(!out.changed);
        assert_eq!(
            out.displayed_viewport(&c),
            Rect::from_min_size(Vec2::ZERO, vec2(400.0, 300.0))
        );
        assert!(wheel(
            &mut c,
            vec2(50.0, 50.0),
            60.0,
            ModifiersState::CONTROL
        ));
        assert!(empty(&mut c, &mut state).zoomed);
        state.reset();
        empty(&mut c, &mut state);
    }
    let before = state;
    assert!(!wheel(
        &mut c,
        vec2(50.0, 50.0),
        f32::NAN,
        ModifiersState::CONTROL
    ));
    assert!(!wheel(
        &mut c,
        vec2(50.0, 50.0),
        f32::INFINITY,
        ModifiersState::CONTROL
    ));
    empty(&mut c, &mut state);
    assert_eq!(state, before);
    assert!(c.input().scroll_delta.is_finite());
}

#[test]
fn drag_source_on_passive_content_takes_precedence_over_background_pan() {
    let mut c = setup(1.0);
    let mut state = PanZoomState {
        scale: 2.0,
        translation: vec2(20.0, 20.0),
    };
    let make = |c: &mut Context, state: &mut PanZoomState| {
        draw(c, state, |ui, _| {
            ui.at(
                "source",
                Rect::from_min_size(Vec2::ZERO, vec2(120.0, 40.0)),
                |ui| ui.drag_source(Id::new("item"), 1_usize, |ui| ui.label("Drag item")),
            )
        })
    };
    let out = make(&mut c, &mut state);
    let p = out.local_to_screen(&c, out.inner.response.rect.center());
    pointer(&mut c, p);
    button(&mut c, ElementState::Pressed);
    pointer(&mut c, p + vec2(40.0, 20.0));
    assert!(c.probe().drag.session.is_some());
    let before = state;
    assert!(!make(&mut c, &mut state).panned);
    assert!(c.probe().drag.session.as_ref().is_some_and(|s| s.started));
    assert_eq!(state, before);
    button(&mut c, ElementState::Released);
}

#[test]
fn popup_scroll_and_ime_use_the_fitted_panel_geometry_at_zoom() {
    let mut c = setup(1.5);
    let mut state = PanZoomState {
        scale: 2.0,
        translation: vec2(120.0, 130.0),
    };
    let mut text = "Editable popup".to_owned();
    let mut open = true;
    let make = |c: &mut Context, state: &mut PanZoomState, text: &mut String, open: &mut bool| {
        draw(c, state, |ui, _| {
            let anchor = ui.at(
                "anchor",
                Rect::from_min_size(vec2(-30.0, -20.0), vec2(120.0, 40.0)),
                |ui| ui.button("Popup"),
            );
            Popup::new("panel", anchor.rect)
                .size(vec2(200.0, 160.0))
                .show(ui, open, |ui| {
                    let field = ui.add(TextEdit::new(text).width(170.0));
                    let scroll = ScrollArea::vertical()
                        .id_source("popup-scroll")
                        .max_height(90.0)
                        .show(ui, |ui| {
                            for i in 0..20 {
                                ui.label(format!("Row {i}"));
                            }
                        });
                    (field, scroll)
                })
                .unwrap()
        })
    };
    let out = make(&mut c, &mut state, &mut text, &mut open);
    let field = &out.inner.inner.0;
    let p = c.visual_rect(field.id, field.rect).center();
    pointer(&mut c, p);
    button(&mut c, ElementState::Pressed);
    button(&mut c, ElementState::Released);
    let out = make(&mut c, &mut state, &mut text, &mut open);
    assert!(out.inner.inner.0.has_focus);
    assert!(
        c.visual_rect(out.inner.inner.0.id, out.inner.inner.0.rect)
            .contains(c.probe().ime_area.unwrap().center()),
        "field={:?}, ime={:?}, popup={:?}",
        c.visual_rect(out.inner.inner.0.id, out.inner.inner.0.rect),
        c.probe().ime_area,
        c.probe().popup.as_ref().map(|p| p.rect)
    );
    let p = c.probe().scrolling.states[&out.inner.inner.1.id]
        .viewport
        .center();
    assert!(c.probe().popup.as_ref().unwrap().rect.contains(p));
    assert!(wheel(&mut c, p, -60.0, ModifiersState::empty()));
    let out = make(&mut c, &mut state, &mut text, &mut open);
    assert!(out.inner.inner.1.offset.y > 0.0 && !out.changed);
}
