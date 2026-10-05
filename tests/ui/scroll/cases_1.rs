use super::*;

#[test]
fn pixel_deltas_dpi_clipping_hits_and_idle_cache() {
    for scale in [1.0, 1.25, 2.0] {
        let mut c = setup(scale);
        let first = area(&mut c, 130.0, 30, None);
        let pointer = first.viewport.center();
        assert!(wheel(&mut c, pointer, vec2(0.0, 17.375)).consumed);
        // A burst must not activate the old, untranslated button rectangles.
        assert!(!c.probe().previous_hits.iter().any(|h| h.action.focusable()));
        let scrolled = area(&mut c, 130.0, 30, None);
        assert_eq!(scrolled.offset, vec2(0.0, 17.375));
        assert_eq!(
            first.inner[0].rect.min.y - scrolled.inner[0].rect.min.y,
            17.375
        );
        assert!(!scrolled.inner[10].has_focus);
        assert!(!c
            .probe()
            .previous_hits
            .iter()
            .any(|h| h.id == scrolled.inner[10].id));
        for r in &scrolled.inner {
            if let Some(hit) = c.probe().previous_hits.iter().find(|h| h.id == r.id) {
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
    assert_eq!(c.probe().focused_widget, Some(visible.id));
    let small = area(&mut c, 240.0, 2, None);
    assert_eq!(small.offset, Vec2::ZERO);
    assert_eq!(c.probe().focused_widget, None);
    let hit = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.id == small.inner[0].id)
        .unwrap();
    assert_eq!(hit.rect.min, small.viewport.min);
    let cached = &c.probe().cache[&hit.id.with("body")];
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
    let before = c.probe().focused_widget.unwrap();
    let focused_hit = *c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.id == before)
        .unwrap();
    assert!(!focused_hit.rect.intersect(focused_hit.clip).is_empty());
    area(&mut c, 130.0, 30, Some(Vec2::ZERO));
    assert_eq!(c.probe().focused_widget, None);
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
    let point = c.probe().scrolling.states[&inner.id].clip.center();
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
    let state = c.probe().scrolling.states[&inner.id].clone();
    let thumb = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| matches!(h.action, HitAction::ScrollThumb { area, .. } if area == inner.id))
        .unwrap();
    let p = thumb.rect.center();
    assert!(wheel(&mut c, p, vec2(0.0, -1.0)).consumed);
    assert_eq!(
        c.probe().scrolling.states[&inner.id].offset.y,
        state.offset.y - 1.0
    );
}

#[test]
fn thumbs_capture_outside_release_and_disabled_areas() {
    let mut c = setup(1.0);
    let first = area(&mut c, 130.0, 30, None);
    let thumb = *c
        .probe()
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
    assert!(c.probe().capture.is_none());
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
        c.probe().scrolling.states[&two.id].offset,
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
        .probe()
        .previous_hits
        .iter()
        .rfind(|h| h.action == HitAction::Activate)
        .unwrap();
    assert!(!last.rect.intersect(out.viewport).is_empty());
    assert!(out.offset.y > 0.0);
    c.run(|_| {});
    assert_eq!(c.probe().scrolling.states[&out.id].offset, out.offset);
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
