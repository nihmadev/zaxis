use super::*;
use std::time::Duration;

#[test]
fn virtual_rows_clamp_before_shrink_and_skip_fully_clipped_lists() {
    let mut c = setup(1.0);
    let mut range = 0..0;
    let mut id = None;
    for (count, offset) in [(10000, Some(220000.0)), (3, None)] {
        let mut calls = 0;
        c.run(|c| {
            Window::new("Rows").show(c, |ui| {
                let mut area = ScrollArea::vertical().id_source("rows").max_height(120.0);
                if let Some(y) = offset {
                    area = area.scroll_offset(vec2(0.0, y));
                }
                let out = area.show_rows(ui, 22.0, count, |ui, index| {
                    calls += 1;
                    ui.label(format!("Row {index}"));
                });
                range = out.inner;
                id = Some(out.id);
            })
        });
        assert_eq!(calls, range.len());
        if count == 3 {
            assert_eq!(range, 0..3);
            assert_eq!(c.probe().scrolling.states[&id.unwrap()].offset, Vec2::ZERO);
        }
    }
    let mut calls = 0;
    c.run(|c| {
        Window::new("Rows").show(c, |ui| {
            ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                ui.allocate_space(vec2(100.0, 400.0));
                ScrollArea::vertical()
                    .max_height(100.0)
                    .show_rows(ui, 22.0, 1000, |_, _| {
                        calls += 1;
                    });
            });
        })
    });
    assert_eq!(calls, 0);
}

#[test]
fn corrected_ancestor_clips_nested_geometry_and_dpi_changes_invalidate_meshes() {
    let mut c = setup(1.0);
    let mut outer_id = None;
    let mut inner_id = None;
    let mut button_id = None;
    let draw = |c: &mut Context,
                offset: Option<f32>,
                outer_id: &mut _,
                inner_id: &mut _,
                button_id: &mut _| {
        c.run(|c| {
            Window::new("Clamp tree").show(c, |ui| {
                let mut area = ScrollArea::vertical().id_source("outer").max_height(160.0);
                if let Some(y) = offset {
                    area = area.scroll_offset(vec2(0.0, y));
                }
                let outer = area.show(ui, |ui| {
                    let inner = ScrollArea::vertical()
                        .id_source("inner")
                        .max_height(100.0)
                        .show(ui, |ui| {
                            let response = ui.button("Visible after clamp");
                            *button_id = Some(response.id);
                        });
                    *inner_id = Some(inner.id);
                });
                *outer_id = Some(outer.id);
            })
        });
    };
    draw(
        &mut c,
        Some(700.0),
        &mut outer_id,
        &mut inner_id,
        &mut button_id,
    );
    let outer = &c.probe().scrolling.states[&outer_id.unwrap()];
    let inner = &c.probe().scrolling.states[&inner_id.unwrap()];
    assert_eq!(outer.offset, Vec2::ZERO);
    let hit = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.id == button_id.unwrap())
        .unwrap();
    assert_eq!(hit.rect.min, inner.viewport.min);
    assert_eq!(hit.clip, inner.viewport.intersect(outer.viewport));
    let stats = c.cache_stats();
    c.set_viewport(PhysicalSize::new(1400, 1200), 2.0);
    draw(&mut c, None, &mut outer_id, &mut inner_id, &mut button_id);
    assert!(c.cache_stats().tessellated_elements > stats.tessellated_elements);
    assert_eq!(c.draw_data().scale_factor, 2.0);
    assert_eq!(
        c.probe().scrolling.states[&outer_id.unwrap()].offset,
        Vec2::ZERO
    );
}

#[test]
fn text_clips_and_ime_move_with_post_measurement_offset_corrections() {
    let mut c = setup(1.0);
    let mut text = String::from("Edit me");
    let mut edit = None;
    let draw =
        |c: &mut Context, target: bool, text: &mut String, edit: &mut Option<zaxis::Response>| {
            c.run(|c| {
                Window::new("IME scroll").show(c, |ui| {
                    ScrollArea::vertical()
                        .id_source("ime")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            let r =
                                ui.add(zaxis::TextEdit::new(text).id_source("edit").width(200.0));
                            *edit = Some(r);
                            ui.allocate_space(vec2(200.0, 500.0));
                            if target {
                                ui.scroll_to_rect(Rect::from_min_size(
                                    r.rect.min + vec2(0.0, 300.0),
                                    vec2(1.0, 30.0),
                                ));
                            }
                        });
                })
            });
        };
    draw(&mut c, false, &mut text, &mut edit);
    let r = edit.unwrap();
    c.move_pointer(r.rect.center());
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    draw(&mut c, false, &mut text, &mut edit);
    assert!(c.probe().ime_area.is_some());
    draw(&mut c, true, &mut text, &mut edit);
    assert!(c.probe().ime_area.is_none());
    assert!(c.probe().focused_widget.is_none());
    assert!(!c.probe().previous_hits.iter().any(|h| h.id == r.id));
}

#[test]
fn line_wheel_shift_and_front_panels_respect_axis_and_layer_routing() {
    let mut c = setup(2.0);
    let mut id = None;
    let mut pointer = Vec2::ZERO;
    c.run(|c| {
        Window::new("Wheel lines").show(c, |ui| {
            let out = ScrollArea::both().max_height(120.0).show(ui, |ui| {
                ui.allocate_space(vec2(800.0, 800.0));
            });
            id = Some(out.id);
            pointer = out.viewport.center();
        })
    });
    c.move_pointer(pointer);
    let event = WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::LineDelta(0.0, -2.0),
        phase: TouchPhase::Moved,
    };
    assert!(c.on_window_event(&event).consumed);
    let wheel = 2.0 * c.style().font_size * 4.5;
    assert_eq!(
        c.probe().scrolling.states[&id.unwrap()].offset,
        vec2(0.0, wheel)
    );
    c.set_modifiers(winit::keyboard::ModifiersState::SHIFT);
    assert!(c.on_window_event(&event).consumed);
    assert_eq!(
        c.probe().scrolling.states[&id.unwrap()].offset,
        vec2(wheel, wheel)
    );
    c.run(|c| {
        Window::new("Wheel lines").show(c, |ui| {
            ScrollArea::both().max_height(120.0).show(ui, |ui| {
                ui.allocate_space(vec2(800.0, 800.0));
            });
        });
        Window::new("Cover")
            .default_position(pointer - vec2(20.0, 20.0))
            .show(c, |ui| {
                ui.label("Cover");
            });
    });
    c.move_pointer(pointer);
    let before = c.probe().scrolling.states[&id.unwrap()].offset;
    assert!(!c.on_window_event(&event).consumed);
    assert_eq!(c.probe().scrolling.states[&id.unwrap()].offset, before);
}

#[test]
fn translated_mesh_matches_fresh_tessellation_including_fractional_glyph_phases() {
    for scale in [1.0, 1.25, 2.0] {
        let mut cached = setup(scale);
        let mut fresh = setup(scale);
        let id = Id::new("translation probe");
        let draw = |c: &mut Context, shift: Vec2| {
            c.run(|c| {
                c.paint(
                    id,
                    Id::new("probe layer"),
                    c.viewport(),
                    vec![
                        Paint::Shape(
                            Shape::rect(
                                Rect::from_min_size(
                                    vec2(80.137, 110.234) + shift,
                                    vec2(300.0, 45.0),
                                ),
                                zaxis::Color::WHITE,
                            )
                            .corner_radius(4.0)
                            .into(),
                        ),
                        Paint::Text {
                            text: "Glyph phases: ffi е́ 🙂".into(),
                            position: vec2(91.137, 121.234) + shift,
                            size: 16.0,
                            weight: zaxis::FontWeight::REGULAR,
                            wrap_width: f32::INFINITY,
                            color: zaxis::Color::BLACK,
                        },
                    ],
                );
            });
        };
        draw(&mut cached, Vec2::ZERO);
        draw(&mut fresh, Vec2::ZERO);
        for y in [16.0 / scale as f32, 17.375, 24.0 / scale as f32, 0.0] {
            draw(&mut cached, vec2(0.0, y));
            fresh.probe_mut().cache.clear();
            draw(&mut fresh, vec2(0.0, y));
            let a = &cached.probe().cache[&id].mesh;
            let b = &fresh.probe().cache[&id].mesh;
            assert_eq!(a.indices, b.indices);
            assert_eq!(a.batches, b.batches);
            assert_eq!(a.vertices.len(), b.vertices.len());
            for (a, b) in a.vertices.iter().zip(&b.vertices) {
                assert_eq!(a.uv, b.uv, "glyph raster phase at DPI {scale}");
                assert_eq!(a.color, b.color);
                assert!(
                    (Vec2::from_array(a.position) - Vec2::from_array(b.position)).length() < 0.001
                );
            }
        }
    }
}

#[test]
fn hidden_paint_is_retained_without_tessellation_and_reappears_with_current_hits() {
    let mut c = setup(1.25);
    let mut viewport = Rect::default();
    let mut hidden_id = None;
    let draw = |c: &mut Context, offset, viewport: &mut Rect, hidden_id: &mut Option<Id>| {
        c.run(|c| {
            Window::new("Culling")
                .default_size(vec2(500.0, 450.0))
                .show(c, |ui| {
                    let out = ScrollArea::vertical()
                        .id_source("cull")
                        .max_height(120.0)
                        .scroll_offset(vec2(0.0, offset))
                        .show(ui, |ui| {
                            for index in 0..100 {
                                let response = ui.button(format!("Setting {index}"));
                                if index == 80 {
                                    *hidden_id = Some(response.id);
                                }
                            }
                        });
                    *viewport = out.viewport;
                });
        });
    };
    draw(&mut c, 0.0, &mut viewport, &mut hidden_id);
    let id = hidden_id.unwrap();
    let paint_id = id.with("body");
    let original_mesh = std::sync::Arc::clone(&c.probe().cache[&paint_id].mesh);
    assert!(
        c.draw_data().vertices.len() * 2
            < c.probe()
                .cache
                .values()
                .map(|cached| cached.mesh.vertices.len())
                .sum::<usize>()
    );
    draw(&mut c, 17.375, &mut viewport, &mut hidden_id);
    assert!(std::sync::Arc::ptr_eq(
        &original_mesh,
        &c.probe().cache[&paint_id].mesh
    ));
    assert!(!c.probe().previous_hits.iter().any(|hit| hit.id == id));

    let target = c.probe().cache[&paint_id].bounds.unwrap().min.y - viewport.min.y;
    draw(&mut c, target, &mut viewport, &mut hidden_id);
    assert!(c.probe().previous_hits.iter().any(|hit| hit.id == id));
    assert!(c.probe().elements.iter().any(|e| e.id == paint_id
        && !e
            .clip
            .intersect(c.probe().cache[&paint_id].bounds.unwrap())
            .is_empty()));
    let revision = c.draw_data().revision;
    draw(&mut c, target, &mut viewport, &mut hidden_id);
    assert_eq!(c.draw_data().revision, revision);
}

#[test]
fn a_wheel_notch_glides_to_its_target_and_settles() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(700, 600), 1.0);
    let t0 = Instant::now();
    let mut marker = 0.0;
    let mut pass = |c: &mut Context, at| {
        let mut viewport = Rect::default();
        c.run_at(at, |c| {
            Window::new("Glide").show(c, |ui| {
                let out = ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                    marker = ui.allocate_space(vec2(10.0, 10.0)).min.y;
                    ui.allocate_space(vec2(10.0, 800.0));
                });
                viewport = out.viewport;
            })
        });
        marker - viewport.min.y
    };
    pass(&mut c, t0);
    let top = pass(&mut c, t0 + Duration::from_millis(16));
    c.move_pointer(vec2(100.0, 100.0));
    // Place the pointer inside the area's viewport.
    c.move_pointer(c.probe().scrolling.states.values().next().unwrap().viewport.center());
    let event = WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::LineDelta(0.0, -1.0),
        phase: TouchPhase::Moved,
    };
    assert!(c.on_window_event(&event).consumed);
    let target = c.style().font_size * 4.5;
    assert_eq!(c.probe().scrolling.states.values().next().unwrap().offset.y, target);
    let early = pass(&mut c, t0 + Duration::from_millis(32));
    let later = pass(&mut c, t0 + Duration::from_millis(80));
    assert!(top - early > 0.0 && top - early < target, "starts moving: {early}");
    assert!(top - later > top - early && top - later < target, "keeps moving: {later}");
    assert!(c.needs_repaint_at(t0 + Duration::from_millis(100)), "asks for the next frame");
    let done = pass(&mut c, t0 + Duration::from_secs(2));
    assert_eq!(top - done, target);
    pass(&mut c, t0 + Duration::from_secs(3));
    assert!(!c.needs_repaint_at(t0 + Duration::from_secs(3)), "settled content sleeps");
}
