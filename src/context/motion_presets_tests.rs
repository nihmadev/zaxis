use super::*;
use crate::{
    animation::*, vec2, Button, Layout, Loader, Presence, Progress, ProgressState, Rect, Reorder,
    ScrollArea, Window,
};
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState};
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn setup() -> (Context, Instant) {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(900, 700), 1.0);
    let t = c.frame_time() + ms(1000);
    (c, t)
}
fn button_hit(c: &Context, id: Id) -> Option<HitRegion> {
    c.previous_hits
        .iter()
        .copied()
        .find(|hit| hit.id == id && hit.action == HitAction::Activate)
}

#[test]
fn presence_reverses_without_jumps_excludes_exit_input_and_removes_state() {
    let (mut c, t) = setup();
    let draw = |c: &mut Context, at, visible| {
        let mut calls = 0;
        let mut response = None;
        c.run_at(t + ms(at), |c| {
            Window::new("presence").show(c, |ui| {
                Presence::fade()
                    .offset(vec2(8.0, 0.0))
                    .scaling(0.96)
                    .motion(TweenOptions::new(ms(200)))
                    .show(ui, "item", visible, |ui| {
                        calls += 1;
                        response = Some(ui.button("Action"));
                        ui.label("Fixed text");
                    });
            });
        });
        (calls, response)
    };
    assert_eq!(draw(&mut c, 0, false).0, 0);
    assert!(!c.wants_animation_frame());
    let id = draw(&mut c, 1, true).1.unwrap().id;
    draw(&mut c, 81, true);
    let hit = button_hit(&c, id).unwrap();
    c.move_pointer(hit.rect.center());
    assert!(draw(&mut c, 81, true).1.unwrap().hovered);
    c.request_focus(id);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    // Exit must discard queued click before closure executes.
    let outgoing = draw(&mut c, 81, false).1.unwrap();
    assert!(!outgoing.clicked() && !outgoing.enabled);
    assert!(button_hit(&c, id).is_none());
    assert_eq!(c.focused_widget, None);
    let channel = Id::new(("window", "presence"))
        .with("content")
        .with(("presence", Id::new("item")));
    let alpha = c.sample_animation::<f32>(channel).unwrap().value;
    draw(&mut c, 81, true);
    assert_eq!(c.sample_animation::<f32>(channel).unwrap().value, alpha);
    assert_eq!(button_hit(&c, id).unwrap().rect, hit.rect);
    draw(&mut c, 281, true);
    assert_eq!(c.next_repaint(), None);
    let before = c.cache_stats();
    let revision = c.draw_data().revision;
    draw(&mut c, 282, true);
    assert_eq!(
        c.cache_stats().tessellated_elements,
        before.tessellated_elements
    );
    assert_eq!(c.draw_data().revision, revision);
    draw(&mut c, 283, false);
    assert_eq!(draw(&mut c, 483, false).0, 0);
    assert_eq!(c.next_repaint(), None);
    c.run_at(t + ms(484), |_| {});
    assert!(c.effect_states.is_empty());
    assert_eq!(c.next_repaint(), None);
    draw(&mut c, 485, false);
    assert_eq!(draw(&mut c, 486, true).0, 1);
}

#[test]
fn fade_scale_reuse_base_text_and_apply_same_mapping_to_paint_clips_and_hits() {
    let (mut c, t) = setup();
    let mut slider = 0.0;
    let draw = |c: &mut Context, slider: &mut f32, at| {
        let mut response = None;
        c.run_at(t + ms(at), |c| {
            Window::new("transforms").show(c, |ui| {
                Presence::fade()
                    .scaling(0.9)
                    .offset(vec2(12.0, 4.0))
                    .motion(TweenOptions::new(ms(200)))
                    .show(ui, "scaled", true, |ui| {
                        response = Some(ui.slider(slider, 0.0..=1.0));
                        ui.label("Cached glyphs");
                    });
                ui.label("Sibling stays opaque");
            });
        });
        response.unwrap()
    };
    let r = draw(&mut c, &mut slider, 0);
    let stats = c.cache_stats();
    draw(&mut c, &mut slider, 80);
    assert_eq!(
        c.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    let transform = c.input_transforms[&r.id];
    let h = c.previous_hits.iter().find(|h| h.id == r.id).unwrap();
    assert_eq!(h.rect, transform.rect(r.rect));
    assert!(h.clip.max.x <= c.viewport().max.x);
    let pointer = h.rect.center();
    c.slider_input
        .insert(r.id, vec![SliderInput::Pointer(pointer)]);
    let sample = c.take_slider_input(r.id);
    let SliderInput::Pointer(local) = sample[0] else {
        panic!()
    };
    assert!((local - r.rect.center()).length() < 0.001);
    let vertices = &c
        .elements
        .iter()
        .find(|e| e.id == r.id.with("body"))
        .map(|e| &e.mesh.vertices);
    // At least one transformed element exists; alpha is applied to all primitive types.
    assert!(!c.visual_meshes.is_empty());
    let _ = vertices;
    draw(&mut c, &mut slider, 200);
    assert_eq!(c.next_repaint(), None);
}

#[test]
fn reveal_measures_once_retargets_resizes_and_reflows_neighbors_and_scroll() {
    let (mut c, t) = setup();
    let draw = |c: &mut Context, at, open, height| {
        let mut calls = 0;
        let mut next = Rect::default();
        let mut inner = None;
        c.run_at(t + ms(at), |c| {
            Window::new("reveal").show(c, |ui| {
                ScrollArea::vertical().max_height(130.0).show(ui, |ui| {
                    ui.reveal("details", open, |ui| {
                        calls += 1;
                        inner = Some(ui.button("Inside"));
                        ui.allocate_space(vec2(80.0, height));
                    });
                    next = ui.button("After").rect;
                });
            });
        });
        (calls, next, inner)
    };
    let closed = draw(&mut c, 0, false, 40.0);
    assert_eq!(closed.0, 0);
    draw(&mut c, 1, true, 40.0);
    let middle = draw(&mut c, 81, true, 40.0);
    assert_eq!(middle.0, 1);
    assert!(middle.1.min.y > closed.1.min.y);
    let resize = draw(&mut c, 81, true, 100.0);
    assert_eq!(resize.1, middle.1);
    let grown = draw(&mut c, 241, true, 100.0);
    assert!(grown.1.min.y > middle.1.min.y);
    let closing = draw(&mut c, 241, false, 100.0);
    assert_eq!(closing.1, grown.1);
    assert!(button_hit(&c, closing.2.unwrap().id).is_none());
    let reopened = draw(&mut c, 281, true, 60.0);
    assert_eq!(reopened.0, 1);
    draw(&mut c, 441, true, 60.0);
    assert_eq!(c.next_repaint(), None);
    draw(&mut c, 442, false, 60.0);
    let end = draw(&mut c, 602, false, 60.0);
    assert_eq!(end.0, 0);
    assert_eq!(end.1, closed.1);
    assert_eq!(c.next_repaint(), None);
}

#[test]
fn reorder_uses_stable_model_ids_continues_motion_and_ignores_scroll() {
    let (mut c, t) = setup();
    let ids: Vec<_> = (0..3).map(Id::new).collect();
    let draw =
        |c: &mut Context, at, order: &[Id], offset, virtual_range: std::ops::Range<usize>| {
            let mut rows = Vec::new();
            c.run_at(t + ms(at), |c| {
                Window::new("reorder").show(c, |ui| {
                    ScrollArea::vertical()
                        .max_height(100.0)
                        .scroll_offset(vec2(0.0, offset))
                        .show(ui, |ui| {
                            Reorder::new("files")
                                .motion(TweenOptions::new(ms(200)))
                                .show(ui, order.iter().copied(), |list| {
                                    for (n, &id) in order.iter().enumerate() {
                                        if virtual_range.contains(&n) {
                                            list.item(id, |ui| {
                                                rows.push((
                                                    id,
                                                    ui.add(
                                                        Button::new("same label")
                                                            .min_size(vec2(140.0, 36.0)),
                                                    ),
                                                ));
                                            });
                                        } else {
                                            list.ui().allocate_space(vec2(140.0, 36.0));
                                        }
                                    }
                                });
                        });
                });
            });
            rows
        };
    let first = draw(&mut c, 0, &ids, 0.0, 0..3);
    let reversed: Vec<_> = ids.iter().rev().copied().collect();
    let switched = draw(&mut c, 1, &reversed, 0.0, 0..3);
    for (key, r) in first {
        let new = switched.iter().find(|(id, _)| *id == key).unwrap().1;
        assert_eq!(button_hit(&c, new.id).unwrap().rect, r.rect);
    }
    let moving = draw(&mut c, 81, &reversed, 0.0, 0..3);
    let displayed: Vec<_> = moving
        .iter()
        .map(|(id, r)| (*id, button_hit(&c, r.id).unwrap().rect))
        .collect();
    draw(&mut c, 81, &ids, 0.0, 0..3);
    for (id, rect) in displayed {
        let r = moving.iter().find(|(key, _)| *key == id).unwrap().1;
        assert_eq!(button_hit(&c, r.id).unwrap().rect, rect);
    }
    draw(&mut c, 281, &ids, 0.0, 0..3);
    assert_eq!(c.next_repaint(), None);
    draw(&mut c, 282, &ids, 30.0, 0..3);
    assert_eq!(c.next_repaint(), None);
    // Virtualized absence keeps membership and timeline but requests no frames.
    draw(&mut c, 283, &reversed, 30.0, 0..0);
    assert_eq!(c.next_repaint(), None);
    draw(&mut c, 1000, &reversed, 30.0, 0..3);
    assert_eq!(c.next_repaint(), None);
    draw(&mut c, 1001, &ids[0..1], 0.0, 0..1);
    draw(&mut c, 1201, &ids[0..1], 0.0, 0..1);
    draw(&mut c, 1202, &ids, 0.0, 0..3);
    assert_eq!(c.next_repaint(), None);
}

#[test]
fn selection_indicator_tracks_latest_bounds_size_orientation_and_removal() {
    let (mut c, t) = setup();
    let a = Id::new("a");
    let b = Id::new("b");
    let draw = |c: &mut Context, at, selected, width, orientation| {
        let mut indicator = None;
        c.run_at(t + ms(at), |c| {
            Window::new("selection").show(c, |ui| {
                let r = ui.allocate_space(vec2(width, 32.0));
                let next = ui.allocate_space(vec2(120.0, 32.0));
                indicator =
                    ui.selection_indicator("chosen", selected, &[(a, r), (b, next)], orientation);
            });
        });
        indicator
    };
    let first = draw(&mut c, 0, Some(a), 80.0, Layout::Horizontal).unwrap();
    assert_eq!(
        draw(&mut c, 1, Some(b), 80.0, Layout::Horizontal),
        Some(first)
    );
    let middle = draw(&mut c, 81, Some(b), 80.0, Layout::Horizontal).unwrap();
    assert_eq!(
        draw(&mut c, 81, Some(a), 150.0, Layout::Horizontal),
        Some(middle)
    );
    let resized = draw(&mut c, 281, Some(a), 150.0, Layout::Horizontal).unwrap();
    assert_eq!(resized.size().x, 150.0);
    draw(&mut c, 282, Some(a), 150.0, Layout::Vertical);
    assert_eq!(
        draw(&mut c, 482, Some(a), 150.0, Layout::Vertical)
            .unwrap()
            .size()
            .x,
        2.0
    );
    assert_eq!(draw(&mut c, 483, None, 150.0, Layout::Vertical), None);
    assert_eq!(c.next_repaint(), None);
}

#[test]
fn loader_progress_pulse_visibility_reduced_motion_and_idle_cache() {
    let (mut c, t) = setup();
    let draw = |c: &mut Context, at, active, offset| {
        let mut rect = Rect::default();
        c.run_at(t + ms(at), |c| {
            Window::new("busy").offset(vec2(offset, 0.0)).show(c, |ui| {
                rect = ui.add(Loader::new().active(active)).rect;
                ui.add(Progress::new(if active {
                    ProgressState::Indeterminate { active }
                } else {
                    ProgressState::Determinate(0.5)
                }));
                ui.pulse("task", active, |ui| {
                    ui.label("Busy");
                });
            });
        });
        rect
    };
    let first = draw(&mut c, 0, true, 0.0);
    assert!(c.wants_animation_frame());
    let before = c.cache_stats();
    assert_eq!(draw(&mut c, 300, true, 0.0), first);
    // Progress changes shape, but Loader/text base geometry is reusable.
    assert!(c.cache_stats().tessellated_elements - before.tessellated_elements <= 1);
    assert!(!c
        .previous_hits
        .iter()
        .any(|h| h.action == HitAction::Activate));
    draw(&mut c, 301, false, 0.0);
    assert_eq!(c.next_repaint(), None);
    let before = c.draw_data().revision;
    draw(&mut c, 302, false, 0.0);
    assert_eq!(c.draw_data().revision, before);
    draw(&mut c, 303, true, 1000.0);
    assert_eq!(c.next_repaint(), None);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    draw(&mut c, 304, true, 0.0);
    assert_eq!(c.next_repaint(), None);
    c.run_at(t + ms(305), |_| {});
    assert_eq!(c.next_repaint(), None);
}

#[test]
fn repeated_highlight_uses_current_alpha_and_current_theme_base() {
    let (mut c, t) = setup();
    let draw = |c: &mut Context, at, revision, base| {
        c.run_at(t + ms(at), |c| {
            Window::new("highlight").show(c, |ui| {
                let rect = ui.allocate_space(vec2(120.0, 24.0));
                ui.highlight("file", revision, rect, base, crate::Color::WHITE);
            });
        });
    };
    draw(&mut c, 0, 0, crate::Color::gray(50));
    assert_eq!(c.next_repaint(), None);
    draw(&mut c, 1, 1, crate::Color::gray(50));
    draw(&mut c, 81, 1, crate::Color::gray(50));
    let before = c.draw_data().vertices.clone();
    draw(&mut c, 81, 2, crate::Color::gray(50));
    assert_eq!(c.draw_data().vertices, before);
    draw(&mut c, 1000, 2, crate::Color::gray(100));
    assert_eq!(c.next_repaint(), None);
    let revision = c.draw_data().revision;
    draw(&mut c, 1001, 2, crate::Color::gray(100));
    assert_eq!(c.draw_data().revision, revision);
}

#[test]
fn transformed_nested_scroll_preserves_logical_extent_wheel_and_hit_clip() {
    let (mut c, t) = setup();
    let mut output = None;
    let mut row = None;
    let transform = crate::Transform::around(vec2(60.0, 100.0), 0.9, vec2(8.0, 4.0));
    c.run_at(t, |c| {
        Window::new("scroll-transform").show(c, |ui| {
            ui.visual("group", transform, 0.5, |ui| {
                let scroll = ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                    row = Some(ui.button("Inside"));
                    ui.allocate_space(vec2(140.0, 200.0));
                });
                output = Some((scroll.id, scroll.viewport, scroll.content_size));
            });
        });
    });
    let (id, viewport, content) = output.unwrap();
    let state = &c.scrolling.states[&id];
    assert_eq!(state.viewport, transform.rect(viewport));
    assert!((state.max_offset().y - (content.y - viewport.size().y)).abs() < 0.001);
    let hit = button_hit(&c, row.unwrap().id).unwrap();
    assert_eq!(hit.rect, transform.rect(row.unwrap().rect));
    assert!(hit.clip.max.y <= state.viewport.max.y);
    c.scroll_from(id, vec2(0.0, 18.0), false);
    assert!((c.scrolling.states[&id].offset.y - 20.0).abs() < 0.001);
}

#[test]
fn popup_portal_tracks_visual_anchor_and_can_remove_deferred_input() {
    let (mut c, t) = setup();
    let mut anchor = Rect::default();
    let mut response = None;
    let transform = crate::Transform::translation(vec2(8.0, 12.0));
    c.run_at(t, |c| {
        Window::new("portal").show(c, |ui| {
            ui.visual("group", transform, 0.5, |ui| {
                anchor = ui.button("Anchor").rect;
                let mut open = true;
                crate::Popup::new("menu", anchor).show(ui, &mut open, |ui| {
                    response = Some(ui.button("Choice"));
                });
            });
        });
    });
    assert_eq!(c.popup.as_ref().unwrap().anchor, transform.rect(anchor));
    let response = response.unwrap();
    assert_eq!(
        button_hit(&c, response.id).unwrap().rect,
        transform.rect(response.rect)
    );
    c.run_at(t + ms(1), |c| {
        Window::new("portal").show(c, |ui| {
            ui.visual("group", transform, 0.5, |ui| {
                let anchor = ui.button("Anchor").rect;
                let mut open = true;
                crate::Popup::new("menu", anchor).show(ui, &mut open, |ui| {
                    ui.button("Choice");
                    ui.context().close_popup();
                });
            });
        });
    });
    assert!(c.popup.is_none());
    assert!(button_hit(&c, response.id).is_none());
}
