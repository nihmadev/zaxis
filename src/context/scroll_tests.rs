use super::*;
use crate::{vec2, Rect, ScrollArea, ScrollAreaOutput, Shape, Window};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::KeyCode,
};

fn setup(scale: f64) -> Context {
    let mut c = Context::new();
    c.set_viewport(
        PhysicalSize::new((700.0 * scale) as u32, (600.0 * scale) as u32),
        scale,
    );
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
fn area(
    c: &mut Context,
    height: f32,
    count: usize,
    offset: Option<Vec2>,
) -> ScrollAreaOutput<Vec<crate::Response>> {
    let mut out = None;
    c.run(|c| {
        Window::new("Scroll test")
            .default_size(vec2(600.0, 500.0))
            .show(c, |ui| {
                let mut scroll = ScrollArea::vertical().id_source("list").max_height(height);
                if let Some(offset) = offset {
                    scroll = scroll.scroll_offset(offset);
                }
                out = Some(scroll.show(ui, |ui| {
                    (0..count)
                        .map(|i| ui.push_id(i, |ui| ui.button(format!("Row {i}"))))
                        .collect()
                }));
            });
    });
    out.unwrap()
}
fn wheel(c: &mut Context, pointer: Vec2, delta: Vec2) -> EventResponse {
    c.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new((pointer.x * c.scale) as f64, (pointer.y * c.scale) as f64),
    });
    c.on_window_event(&WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            (-delta.x * c.scale) as f64,
            (-delta.y * c.scale) as f64,
        )),
        phase: TouchPhase::Moved,
    })
}
#[test]
fn pixel_deltas_dpi_clipping_hits_and_idle_cache() {
    for scale in [1.0, 1.25, 2.0] {
        let mut c = setup(scale);
        let first = area(&mut c, 130.0, 30, None);
        let pointer = first.viewport.center();
        assert!(wheel(&mut c, pointer, vec2(0.0, 17.375)).consumed);
        // A burst must not activate the old, untranslated button rectangles.
        assert!(!c.previous_hits.iter().any(|h| h.action.focusable()));
        let scrolled = area(&mut c, 130.0, 30, None);
        assert_eq!(scrolled.offset, vec2(0.0, 17.375));
        assert_eq!(
            first.inner[0].rect.min.y - scrolled.inner[0].rect.min.y,
            17.375
        );
        assert!(!scrolled.inner[10].has_focus);
        assert!(!c
            .previous_hits
            .iter()
            .any(|h| h.id == scrolled.inner[10].id));
        for r in &scrolled.inner {
            if let Some(hit) = c.previous_hits.iter().find(|h| h.id == r.id) {
                assert_eq!(hit.rect, r.rect);
                assert_eq!(hit.clip, scrolled.viewport);
            }
        }
        c.move_pointer(scrolled.inner[1].rect.center());
        c.primary_button(ElementState::Pressed);
        c.primary_button(ElementState::Released);
        assert!(area(&mut c, 130.0, 30, None).inner[1].clicked());
        area(&mut c, 130.0, 30, None);
        let stats = c.cache_stats();
        let revision = c.draw_data().revision;
        area(&mut c, 130.0, 30, None);
        assert_eq!(c.draw_data().revision, revision);
        assert_eq!(
            c.cache_stats().tessellated_elements,
            stats.tessellated_elements
        );
        assert!(!c.needs_repaint());
        assert!(c
            .draw_data()
            .commands
            .iter()
            .any(|cmd| cmd.clip_rect == scrolled.viewport));
    }
}
#[test]
fn shrink_resize_correct_paint_hits_and_focus_on_the_same_pass() {
    let mut c = setup(1.0);
    let initial = area(&mut c, 130.0, 40, Some(vec2(0.0, 700.0)));
    let visible = initial
        .inner
        .iter()
        .find(|r| !r.rect.intersect(initial.viewport).is_empty())
        .unwrap();
    c.move_pointer(visible.rect.intersect(initial.viewport).center());
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    assert_eq!(c.focused_widget, Some(visible.id));
    let small = area(&mut c, 240.0, 2, None);
    assert_eq!(small.offset, Vec2::ZERO);
    assert_eq!(c.focused_widget, None);
    let hit = c
        .previous_hits
        .iter()
        .find(|h| h.id == small.inner[0].id)
        .unwrap();
    assert_eq!(hit.rect.min, small.viewport.min);
    let cached = &c.cache[&hit.id.with("body")];
    // The final cache description, not just scissor metadata, has corrected coordinates.
    assert!(cached.paint.iter().any(
        |p| matches!(p, Paint::Shape(Shape::Rect { rect, .. }) if rect.min == small.viewport.min)
    ));
    area(&mut c, 130.0, 30, Some(vec2(0.0, 900.0)));
    let bigger = area(&mut c, 350.0, 30, None);
    assert!(bigger.offset.y <= bigger.content_size.y - bigger.viewport.size().y);
    area(&mut c, 130.0, 30, Some(vec2(0.0, 600.0)));
    for _ in 0..10 {
        c.key(KeyCode::Tab, ElementState::Pressed, false);
    }
    let before = c.focused_widget.unwrap();
    let focused_hit = *c.previous_hits.iter().find(|h| h.id == before).unwrap();
    assert!(!focused_hit.rect.intersect(focused_hit.clip).is_empty());
    area(&mut c, 130.0, 30, Some(Vec2::ZERO));
    assert_eq!(c.focused_widget, None);
}
fn nested(
    c: &mut Context,
    inner_offset: Option<f32>,
) -> (ScrollAreaOutput<()>, ScrollAreaOutput<()>) {
    let mut outer = None;
    let mut inner = None;
    c.run(|c| {
        Window::new("Nested")
            .default_size(vec2(600.0, 500.0))
            .show(c, |ui| {
                outer = Some(
                    ScrollArea::vertical()
                        .id_source("outer")
                        .max_height(250.0)
                        .show(ui, |ui| {
                            let mut area =
                                ScrollArea::vertical().id_source("inner").max_height(100.0);
                            if let Some(y) = inner_offset {
                                area = area.scroll_offset(vec2(0.0, y));
                            }
                            inner = Some(area.show(ui, |ui| {
                                ui.allocate_space(vec2(250.0, 400.0));
                            }));
                            ui.allocate_space(vec2(300.0, 900.0));
                        }),
                );
            })
    });
    (outer.unwrap(), inner.unwrap())
}
#[test]
fn nested_routes_to_pointer_and_bubbles_only_unused_delta() {
    let mut c = setup(1.0);
    let (outer, inner) = nested(&mut c, Some(300.0));
    let remaining = inner.content_size.y - inner.viewport.size().y - inner.offset.y;
    assert!(
        wheel(
            &mut c,
            inner.viewport.center(),
            vec2(0.0, remaining + 19.25)
        )
        .consumed
    );
    let (outer2, inner2) = nested(&mut c, None);
    assert_eq!(
        inner2.offset.y,
        inner2.content_size.y - inner2.viewport.size().y
    );
    assert!((outer2.offset.y - 19.25).abs() < 0.001);
    // Upward delta stays in the deepest area while it has space.
    let point = c.scrolling.states[&inner.id].clip.center();
    assert!(wheel(&mut c, point, vec2(0.0, -4.5)).consumed);
    let (outer3, inner3) = nested(&mut c, None);
    assert_eq!(outer3.offset.y, outer2.offset.y);
    assert_eq!(inner3.offset.y, inner2.offset.y - 4.5);
    let outside_inner = vec2(outer.viewport.min.x + 10.0, outer.viewport.max.y - 10.0);
    wheel(&mut c, outside_inner, vec2(0.0, 5.0));
    let (outer4, inner4) = nested(&mut c, None);
    assert_eq!(outer4.offset.y, outer3.offset.y + 5.0);
    assert_eq!(inner4.offset.y, inner3.offset.y);
    // Scrollbar gutter also routes wheel input to its owner.
    let state = c.scrolling.states[&inner.id].clone();
    let thumb = c
        .previous_hits
        .iter()
        .find(|h| matches!(h.action, HitAction::ScrollThumb { area, .. } if area == inner.id))
        .unwrap();
    let p = thumb.rect.center();
    assert!(wheel(&mut c, p, vec2(0.0, -1.0)).consumed);
    assert_eq!(c.scrolling.states[&inner.id].offset.y, state.offset.y - 1.0);
}
#[test]
fn thumbs_capture_outside_release_and_disabled_areas() {
    let mut c = setup(1.0);
    let first = area(&mut c, 130.0, 30, None);
    let thumb = *c
        .previous_hits
        .iter()
        .find(|h| matches!(h.action, HitAction::ScrollThumb { area, axis: 1 } if area == first.id))
        .unwrap();
    c.move_pointer(thumb.rect.center());
    c.primary_button(ElementState::Pressed);
    c.move_pointer(thumb.rect.center() + vec2(100.0, 1000.0));
    c.primary_button(ElementState::Released);
    let bottom = area(&mut c, 130.0, 30, None);
    assert_eq!(
        bottom.offset.y,
        bottom.content_size.y - bottom.viewport.size().y
    );
    assert!(c.capture.is_none());
    assert!(!wheel(&mut c, bottom.viewport.center(), vec2(0.0, 20.0)).consumed);
    assert!(wheel(&mut c, bottom.viewport.center(), vec2(0.0, -0.125)).consumed);
    c.run(|c| {
        Window::new("Scroll test").show(c, |ui| {
            ui.add_enabled_ui(false, |ui| {
                ScrollArea::vertical()
                    .id_source("list")
                    .max_height(130.0)
                    .show(ui, |ui| {
                        ui.allocate_space(vec2(200.0, 1000.0));
                    })
            });
        })
    });
    assert!(!wheel(&mut c, bottom.viewport.center(), vec2(0.0, -20.0)).consumed);
}
#[test]
fn both_axes_targets_measurement_and_virtual_rows() {
    let mut c = setup(1.0);
    let mut calls = Vec::new();
    let mut output = None;
    c.run(|c| {
        Window::new("Virtual")
            .default_size(vec2(500.0, 400.0))
            .show(c, |ui| {
                output = Some(
                    ScrollArea::vertical()
                        .id_source("rows")
                        .max_height(120.0)
                        .scroll_offset(vec2(0.0, 222.5))
                        .show_rows(ui, 22.0, 10000, |ui, row| {
                            calls.push(row);
                            ui.label(format!("Row {row}"));
                        }),
                );
            })
    });
    let output = output.unwrap();
    assert_eq!(output.content_size.y, 220000.0);
    assert_eq!(calls, output.inner.clone().collect::<Vec<_>>());
    assert_eq!(calls[0], 10);
    assert!(calls.len() <= 7);
    let mut two = None;
    c.run(|c| {
        Window::new("Both")
            .default_size(vec2(500.0, 400.0))
            .show(c, |ui| {
                two = Some(
                    ScrollArea::both()
                        .id_source("both")
                        .max_height(120.0)
                        .content_width(900.0)
                        .scroll_to_rect(Rect::from_min_size(vec2(700.0, 500.0), vec2(20.0, 20.0)))
                        .show(ui, |ui| {
                            ui.allocate_space(vec2(900.0, 700.0));
                        }),
                );
            })
    });
    let two = two.unwrap();
    assert!(two.offset.x > 0.0 && two.offset.y > 0.0);
    assert!(wheel(&mut c, two.viewport.center(), vec2(0.375, -1.25)).consumed);
    assert_eq!(
        c.scrolling.states[&two.id].offset,
        two.offset + vec2(0.375, -1.25)
    );
}
#[test]
fn reveal_element_and_stable_ids_survive_hidden_frames() {
    let mut c = setup(1.0);
    let mut out = None;
    c.run(|c| {
        Window::new("Reveal")
            .default_size(vec2(500.0, 400.0))
            .show(c, |ui| {
                out = Some(
                    ScrollArea::vertical()
                        .id_source("items")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for n in 0..30 {
                                let r = ui.push_id(n, |ui| ui.button(format!("Item {n}")));
                                if n == 20 {
                                    ui.scroll_to_response(&r);
                                }
                            }
                        }),
                );
            })
    });
    let out = out.unwrap();
    let last = c
        .previous_hits
        .iter()
        .rfind(|h| h.action == HitAction::Activate)
        .unwrap();
    assert!(!last.rect.intersect(out.viewport).is_empty());
    assert!(out.offset.y > 0.0);
    c.run(|_| {});
    assert_eq!(c.scrolling.states[&out.id].offset, out.offset);
    assert!(!c.scroll_wheel(vec2(0.0, 50.0)));
}
#[test]
fn horizontal_and_sibling_areas_keep_offsets_independent() {
    let mut c = setup(1.0);
    let mut outputs = None;
    let draw = |c: &mut Context, outputs: &mut Option<_>| {
        c.run(|c| {
            Window::new("Siblings")
                .default_size(vec2(600.0, 500.0))
                .show(c, |ui| {
                    let mut left = None;
                    let mut right = None;
                    ui.horizontal(|ui| {
                        left = Some(
                            ScrollArea::horizontal()
                                .id_source("left")
                                .max_width(200.0)
                                .max_height(90.0)
                                .show(ui, |ui| {
                                    ui.allocate_space(vec2(700.0, 40.0));
                                    ui.allocate_space(vec2(200.0, 50.0));
                                }),
                        );
                        right = Some(
                            ScrollArea::vertical()
                                .id_source("right")
                                .max_width(200.0)
                                .max_height(90.0)
                                .show(ui, |ui| {
                                    ui.allocate_space(vec2(190.0, 600.0));
                                }),
                        );
                    });
                    *outputs = Some((left.unwrap(), right.unwrap()));
                });
        })
    };
    draw(&mut c, &mut outputs);
    let (left, right) = outputs.take().unwrap();
    assert_eq!(left.content_size.x, 908.0);
    assert_eq!(left.content_size.y, 50.0);
    wheel(&mut c, left.viewport.center(), vec2(19.75, 50.0));
    draw(&mut c, &mut outputs);
    let (left2, right2) = outputs.take().unwrap();
    assert_eq!(left2.offset, vec2(19.75, 0.0));
    assert_eq!(right2.offset, Vec2::ZERO);
    wheel(&mut c, right.viewport.center(), vec2(15.0, 6.5));
    draw(&mut c, &mut outputs);
    let (left3, right3) = outputs.take().unwrap();
    assert_eq!(left3.offset, left2.offset);
    assert_eq!(right3.offset, vec2(0.0, 6.5));
}

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
            assert_eq!(c.scrolling.states[&id.unwrap()].offset, Vec2::ZERO);
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
    let outer = &c.scrolling.states[&outer_id.unwrap()];
    let inner = &c.scrolling.states[&inner_id.unwrap()];
    assert_eq!(outer.offset, Vec2::ZERO);
    let hit = c
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
    assert_eq!(c.scrolling.states[&outer_id.unwrap()].offset, Vec2::ZERO);
}

#[test]
fn text_clips_and_ime_move_with_post_measurement_offset_corrections() {
    let mut c = setup(1.0);
    let mut text = String::from("Edit me");
    let mut edit = None;
    let draw =
        |c: &mut Context, target: bool, text: &mut String, edit: &mut Option<crate::Response>| {
            c.run(|c| {
                Window::new("IME scroll").show(c, |ui| {
                    ScrollArea::vertical()
                        .id_source("ime")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            let r =
                                ui.add(crate::TextEdit::new(text).id_source("edit").width(200.0));
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
    assert!(c.ime_area.is_some());
    draw(&mut c, true, &mut text, &mut edit);
    assert!(c.ime_area.is_none());
    assert!(c.focused_widget.is_none());
    assert!(!c.previous_hits.iter().any(|h| h.id == r.id));
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
    assert_eq!(c.scrolling.states[&id.unwrap()].offset, vec2(0.0, 32.0));
    c.input.modifiers = winit::keyboard::ModifiersState::SHIFT;
    assert!(c.on_window_event(&event).consumed);
    assert_eq!(c.scrolling.states[&id.unwrap()].offset, vec2(32.0, 32.0));
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
    let before = c.scrolling.states[&id.unwrap()].offset;
    assert!(!c.on_window_event(&event).consumed);
    assert_eq!(c.scrolling.states[&id.unwrap()].offset, before);
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
                                crate::Color::WHITE,
                            )
                            .corner_radius(4.0)
                            .into(),
                        ),
                        Paint::Text {
                            text: "Glyph phases: ffi е́ 🙂".into(),
                            position: vec2(91.137, 121.234) + shift,
                            size: 16.0,
                            wrap_width: f32::INFINITY,
                            color: crate::Color::BLACK,
                        },
                    ],
                );
            });
        };
        draw(&mut cached, Vec2::ZERO);
        draw(&mut fresh, Vec2::ZERO);
        for y in [16.0 / scale as f32, 17.375, 24.0 / scale as f32, 0.0] {
            draw(&mut cached, vec2(0.0, y));
            fresh.cache.clear();
            draw(&mut fresh, vec2(0.0, y));
            let a = &cached.cache[&id].mesh;
            let b = &fresh.cache[&id].mesh;
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
    let original_mesh = std::sync::Arc::clone(&c.cache[&paint_id].mesh);
    assert!(
        c.draw_data().vertices.len() * 4
            < c.cache
                .values()
                .map(|cached| cached.mesh.vertices.len())
                .sum::<usize>()
    );
    draw(&mut c, 17.375, &mut viewport, &mut hidden_id);
    assert!(std::sync::Arc::ptr_eq(
        &original_mesh,
        &c.cache[&paint_id].mesh
    ));
    assert!(!c.previous_hits.iter().any(|hit| hit.id == id));

    let target = c.cache[&paint_id].bounds.unwrap().min.y - viewport.min.y;
    draw(&mut c, target, &mut viewport, &mut hidden_id);
    assert!(c.previous_hits.iter().any(|hit| hit.id == id));
    assert!(c.elements.iter().any(|e| e.id == paint_id
        && !e
            .clip
            .intersect(c.cache[&paint_id].bounds.unwrap())
            .is_empty()));
    let revision = c.draw_data().revision;
    draw(&mut c, target, &mut viewport, &mut hidden_id);
    assert_eq!(c.draw_data().revision, revision);
}
