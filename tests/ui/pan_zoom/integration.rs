use super::*;

#[test]
fn nested_pan_zoom_and_parent_visual_compose_and_remove_only_ancestor_scale_from_pan() {
    let mut c = setup(1.0);
    let mut outer = PanZoomState {
        scale: 1.5,
        translation: vec2(10.0, 15.0),
    };
    let mut inner = PanZoomState {
        scale: 0.5,
        translation: vec2(4.0, 6.0),
    };
    let make = |c: &mut Context, outer: &mut PanZoomState, inner: &mut PanZoomState| {
        let mut out = None;
        c.run(|c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                ui.visual(
                    "parent",
                    Transform {
                        scale: 1.25,
                        translation: vec2(20.0, 25.0),
                        angle: 0.0,
                    },
                    1.0,
                    |ui| {
                        out = Some(PanZoom::new("outer", vec2(400.0, 300.0)).show(
                            ui,
                            outer,
                            |ui, _| {
                                ui.at(
                                    "inner-at",
                                    Rect::from_min_size(vec2(30.0, 40.0), vec2(160.0, 120.0)),
                                    |ui| {
                                        PanZoom::new("inner", vec2(160.0, 120.0)).show(
                                            ui,
                                            inner,
                                            |_, _| {},
                                        )
                                    },
                                )
                            },
                        ));
                    },
                );
            })
        });
        out.unwrap()
    };
    let out = make(&mut c, &mut outer, &mut inner);
    let origin =
        vec2(20.0, 25.0) + (vec2(10.0, 15.0) + (vec2(30.0, 40.0) + vec2(4.0, 6.0)) * 1.5) * 1.25;
    near(out.inner.local_to_screen(&c, Vec2::ZERO), origin);
    let p = out.inner.displayed_viewport(&c).center();
    let anchored = out.inner.screen_to_local(&c, p);
    let outer_before = outer;
    assert!(wheel(&mut c, p, -100.0, ModifiersState::CONTROL));
    let out = make(&mut c, &mut outer, &mut inner);
    assert!(!out.zoomed && out.inner.zoomed);
    assert_eq!(outer, outer_before);
    near(out.inner.local_to_screen(&c, anchored), p);
    pointer(&mut c, p);
    button(&mut c, ElementState::Pressed);
    pointer(&mut c, p + vec2(37.5, 18.75));
    button(&mut c, ElementState::Released);
    let before = inner.translation;
    let out = make(&mut c, &mut outer, &mut inner);
    near(inner.translation - before, vec2(20.0, 10.0));
    assert!(out.inner.panned && !out.panned);
}

#[test]
fn stable_sources_reorder_and_separate_contexts_and_rotated_parent_is_inert() {
    let mut a = setup(1.0);
    let mut b = setup(1.0);
    let mut states = [PanZoomState::default(); 2];
    let make = |c: &mut Context, states: &mut [PanZoomState; 2], reversed: bool, angle: f32| {
        let mut result = Vec::new();
        c.run(|c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                ui.visual(
                    "pose",
                    Transform {
                        angle,
                        ..Default::default()
                    },
                    1.0,
                    |ui| {
                        for i in if reversed { [1, 0] } else { [0, 1] } {
                            result.push(PanZoom::new(i, vec2(300.0, 150.0)).show(
                                ui,
                                &mut states[i],
                                |_, _| {},
                            ));
                        }
                    },
                );
            })
        });
        result
    };
    let out = make(&mut a, &mut states, false, 0.0);
    let id = out[0].response.id;
    assert!(wheel(
        &mut a,
        vec2(50.0, 50.0),
        -100.0,
        ModifiersState::CONTROL
    ));
    make(&mut a, &mut states, false, 0.0);
    assert!(states[0].scale < 1.0 && states[1].scale == 1.0);
    let out = make(&mut a, &mut states, true, 0.0);
    assert_eq!(id, out[1].response.id);
    let mut independent = [PanZoomState::default(); 2];
    make(&mut b, &mut independent, false, 0.0);
    assert_eq!(independent, [PanZoomState::default(); 2]);
    make(&mut a, &mut states, false, 0.2);
    let before = states;
    assert!(!wheel(
        &mut a,
        vec2(50.0, 50.0),
        -100.0,
        ModifiersState::CONTROL
    ));
    pointer(&mut a, vec2(50.0, 50.0));
    button(&mut a, ElementState::Pressed);
    pointer(&mut a, vec2(100.0, 100.0));
    button(&mut a, ElementState::Released);
    make(&mut a, &mut states, false, 0.2);
    assert_eq!(states, before);
}

#[test]
fn grid_and_table_deferred_placement_keep_camera_hits_paint_and_output_in_agreement() {
    for table in [false, true] {
        let mut c = setup(1.5);
        let mut state = PanZoomState {
            scale: 1.25,
            translation: vec2(60.0, 35.0),
        };
        let make = |c: &mut Context, state: &mut PanZoomState| {
            let mut result = None;
            c.run(|c| {
                Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                    let columns = [Column::fixed("label", 100.0), Column::fixed("scene", 350.0)];
                    let mut cell = |ui: &mut Ui<'_>| {
                        result = Some(PanZoom::new("scene", vec2(300.0, 220.0)).show(
                            ui,
                            state,
                            |ui, _| {
                                ui.at(
                                    "button",
                                    Rect::from_min_size(vec2(-20.0, -10.0), vec2(120.0, 45.0)),
                                    |ui| ui.button("Apply"),
                                )
                            },
                        ));
                    };
                    if table {
                        Table::new("table")
                            .columns(columns)
                            .max_height(280.0)
                            .show(ui, |body| {
                                body.row("row", |row| {
                                    row.cell(|ui| {
                                        ui.label("Scene");
                                    });
                                    row.cell(&mut cell);
                                });
                            });
                    } else {
                        Grid::new("grid").columns(columns).show(ui, |grid| {
                            grid.row("row", |row| {
                                row.cell(|ui| {
                                    ui.label("Scene");
                                });
                                row.cell(&mut cell);
                            });
                        });
                    }
                })
            });
            result.unwrap()
        };
        let out = make(&mut c, &mut state);
        let screen = out.local_to_screen(&c, out.inner.rect.center());
        let hit = c.hit_test(screen).unwrap();
        assert_eq!(
            hit.id,
            out.inner.id,
            "table={table}, screen={screen:?}, viewport={:?}, child={:?}, hits={:?}",
            out.viewport,
            out.inner.rect,
            c.probe().previous_hits
        );
        near(c.visual_rect(out.inner.id, out.inner.rect).center(), screen);
        pointer(&mut c, screen);
        button(&mut c, ElementState::Pressed);
        button(&mut c, ElementState::Released);
        let out = make(&mut c, &mut state);
        assert!(out.inner.clicked() && !out.panned);
    }
}
