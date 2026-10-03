use super::*;
use crate::{animation::*, vec2, Button, Color, ColorPicker, Response, Shape, Window};
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn setup() -> Context {
    let mut ctx = Context::new();
    ctx.set_viewport(PhysicalSize::new(800, 600), 1.0);
    ctx
}

#[test]
fn hover_changes_only_affected_geometry_and_settles_into_cached_idle() {
    let mut ctx = setup();
    let start = ctx.frame_time() + ms(1000);
    let draw = |ctx: &mut Context, at| {
        let mut response = None;
        ctx.run_at(start + ms(at), |ctx| {
            Window::new("Test").show(ctx, |ui| {
                response = Some(ui.button("Hover"));
                ui.label("Unchanged");
            });
        });
        response.unwrap()
    };
    let idle = draw(&mut ctx, 0);
    ctx.move_pointer(idle.rect.center());
    assert_eq!(draw(&mut ctx, 1).rect, idle.rect);
    assert_eq!(ctx.next_repaint(), Some(start + ms(17)));
    let before = ctx.cache_stats();
    draw(&mut ctx, 81);
    assert_eq!(
        ctx.cache_stats().tessellated_elements,
        before.tessellated_elements + 1
    );
    draw(&mut ctx, 161);
    assert_eq!(ctx.next_repaint(), None);
    let before = ctx.cache_stats();
    let revision = ctx.draw_data().revision;
    draw(&mut ctx, 10_000);
    assert_eq!(
        ctx.cache_stats().tessellated_elements,
        before.tessellated_elements
    );
    assert_eq!(ctx.draw_data().revision, revision);
    ctx.run_at(start + ms(11_000), |_| {});
    assert_eq!(ctx.next_repaint(), None);
}

#[test]
fn clipped_motion_and_hidden_pages_do_not_schedule_frames() {
    let mut ctx = setup();
    let start = ctx.frame_time() + ms(1000);
    let draw = |ctx: &mut Context, at, target| {
        ctx.run_at(start + ms(at), |ctx| {
            Window::new("Offscreen")
                .default_position(vec2(1000.0, 1000.0))
                .show(ctx, |ui| {
                    ui.transition("invisible", target, TweenOptions::new(ms(200)));
                    ui.button("Outside");
                });
        });
    };
    draw(&mut ctx, 0, 0.0_f32);
    draw(&mut ctx, 1, 1.0);
    assert_eq!(ctx.next_repaint(), None);
    assert!(!ctx.needs_repaint_at(start + ms(50)));
}

#[test]
fn page_slides_use_translated_hits_fixed_clip_and_only_latest_pending_page() {
    let mut ctx = setup();
    let start = ctx.frame_time() + ms(1000);
    let draw = |ctx: &mut Context, at, selected| {
        let mut pages: Vec<(usize, Response)> = Vec::new();
        ctx.run_at(start + ms(at), |ctx| {
            Window::new("Pages")
                .default_size(vec2(600.0, 400.0))
                .show(ctx, |ui| {
                    ui.tab_pages("content", selected, vec2(480.0, 220.0), |ui, index| {
                        pages.push((index, ui.add(Button::new("Page button"))));
                        let rect = ui.allocate_space(vec2(80.0, 20.0));
                        ui.paint(Shape::rect(rect, Color::WHITE));
                    });
                });
        });
        pages
    };
    let initial = draw(&mut ctx, 0, 0);
    let origin = initial[0].1.rect.min;
    draw(&mut ctx, 1, 1);
    let middle = draw(&mut ctx, 41, 2);
    assert!(middle.iter().any(|(page, _)| *page == 0));
    assert!(middle.iter().any(|(page, _)| *page == 1));
    assert!(middle.iter().all(|(_, r)| !r.enabled));
    let incoming = middle.iter().find(|(page, _)| *page == 1).unwrap().1;
    assert!(incoming.rect.min.x > origin.x);
    let hit = ctx
        .previous_hits
        .iter()
        .find(|h| h.id == incoming.id)
        .unwrap();
    assert_eq!(hit.rect, incoming.rect);
    assert!(hit.clip.max.x <= origin.x + 480.0);
    draw(&mut ctx, 80, 3); // replaces queued page 2
    let next = draw(&mut ctx, 281, 3);
    assert!(next.iter().all(|(page, _)| *page != 2));
    let done = draw(&mut ctx, 561, 3);
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].0, 3);
    assert_eq!(done[0].1.rect.min, origin);
    assert!(done[0].1.enabled);
    draw(&mut ctx, 1000, 3);
    assert_eq!(ctx.next_repaint(), None);
    let back = draw(&mut ctx, 1001, 0);
    assert!(back.iter().all(|(_, response)| !response.enabled));
    let middle = draw(&mut ctx, 1041, 0);
    let incoming = middle.iter().find(|(page, _)| *page == 0).unwrap().1;
    assert!(incoming.rect.min.x < origin.x);
    ctx.run_at(start + ms(1042), |_| {});
    assert_eq!(ctx.next_repaint(), None);
    assert!(ctx.tab_pages.is_empty());
}

#[test]
fn color_picker_reverse_reveal_clips_hits_and_reduced_motion_keeps_final_state() {
    let mut ctx = setup();
    let start = ctx.frame_time() + ms(1000);
    let mut color = Color::WHITE;
    let draw = |ctx: &mut Context, color: &mut Color, at| {
        let mut response = None;
        ctx.run_at(start + ms(at), |ctx| {
            Window::new("Picker")
                .default_size(vec2(500.0, 450.0))
                .show(ctx, |ui| {
                    response = Some(ui.add(ColorPicker::new(color, "Color").id_source("stable")));
                });
        });
        response.unwrap()
    };
    let row = draw(&mut ctx, &mut color, 0);
    let click = |ctx: &mut Context| {
        ctx.move_pointer(row.rect.center());
        ctx.primary_button(ElementState::Pressed);
        ctx.primary_button(ElementState::Released);
    };
    click(&mut ctx);
    draw(&mut ctx, &mut color, 1);
    let half = draw(&mut ctx, &mut color, 81);
    assert!(half.rect.size().y > 24.0 && half.rect.size().y < 220.0);
    for hit in ctx
        .previous_hits
        .iter()
        .filter(|hit| hit.id != row.id && hit.window == Id::new(("window", "Picker")))
    {
        if hit.action == HitAction::Slider || hit.action == HitAction::TextEdit {
            assert!(hit.clip.max.y <= half.rect.max.y);
        }
    }
    click(&mut ctx);
    let reversing = draw(&mut ctx, &mut color, 81);
    assert_eq!(reversing.rect, half.rect);
    let angle = ctx
        .sample_animation::<f32>(row.id.with("chevron-angle"))
        .unwrap()
        .value;
    assert!(angle > 0.0 && angle < std::f32::consts::PI);
    draw(&mut ctx, &mut color, 241);
    assert_eq!(
        ctx.sample_animation::<f32>(row.id.with("chevron-angle"))
            .unwrap()
            .value,
        0.0
    );
    let mut style = ctx.style().clone();
    style.motion.reduced_motion = true;
    ctx.set_style(style);
    click(&mut ctx);
    let open = draw(&mut ctx, &mut color, 242);
    assert_eq!(open.rect.size().y, 220.0);
    assert_eq!(ctx.next_repaint(), None);
    draw(&mut ctx, &mut color, 243);
    assert!(!ctx.needs_repaint_at(start + ms(10_000)));
}

#[test]
fn unfocused_or_removed_text_edit_cancels_cursor_deadline() {
    let mut ctx = setup();
    let start = ctx.frame_time() + ms(1000);
    let mut text = String::new();
    let draw = |ctx: &mut Context, text: &mut String, at| {
        let mut response = None;
        ctx.run_at(start + ms(at), |ctx| {
            Window::new("Edit").show(ctx, |ui| {
                response = Some(ui.text_edit(text));
            });
        });
        response.unwrap()
    };
    let edit = draw(&mut ctx, &mut text, 0);
    ctx.move_pointer(edit.rect.center());
    ctx.primary_button(ElementState::Pressed);
    ctx.primary_button(ElementState::Released);
    draw(&mut ctx, &mut text, 1);
    draw(&mut ctx, &mut text, 161);
    assert_eq!(ctx.next_repaint(), Some(start + ms(501)));
    ctx.set_focus(None);
    ctx.move_pointer(vec2(0.0, 0.0));
    draw(&mut ctx, &mut text, 162);
    draw(&mut ctx, &mut text, 322);
    assert_eq!(ctx.next_repaint(), None);
    ctx.run_at(start + ms(323), |_| {});
    assert_eq!(ctx.next_repaint(), None);
}

#[test]
fn panel_offset_moves_paint_clip_and_hits_without_changing_retained_drag_position() {
    let mut ctx = setup();
    let start = ctx.frame_time() + ms(1000);
    let id = Id::new("offset-window");
    let draw = |ctx: &mut Context, at, offset| {
        let mut response = None;
        ctx.run_at(start + ms(at), |ctx| {
            Window::new("Offset")
                .id(id)
                .offset(vec2(0.0, offset))
                .blur(4.0)
                .draggable(false)
                .resizable(false)
                .show(ctx, |ui| {
                    response = Some(ui.button("Button"));
                });
        });
        response.unwrap()
    };
    let initial = draw(&mut ctx, 0, 0.0);
    let retained = ctx.windows[&id].rect;
    let moved = draw(&mut ctx, 1, 100.0);
    assert_eq!(moved.rect, initial.rect.translate(vec2(0.0, 100.0)));
    assert_eq!(ctx.windows[&id].rect, retained);
    assert_eq!(
        ctx.windows[&id].displayed_rect,
        retained.translate(vec2(0.0, 100.0))
    );
    ctx.move_pointer(moved.rect.center());
    assert!(draw(&mut ctx, 2, 100.0).hovered);
    ctx.primary_button(ElementState::Pressed);
    ctx.primary_button(ElementState::Released);
    assert!(draw(&mut ctx, 3, 100.0).clicked());
    let hidden = draw(&mut ctx, 4, 700.0);
    assert!(!hidden.hovered);
    assert!(!ctx.wants_animation_frame());
    assert_eq!(ctx.windows[&id].rect, retained);
    assert!(ctx
        .draw_data()
        .commands
        .iter()
        .all(|command| command.clip_rect.is_empty()));
}

#[test]
fn animating_blur_from_zero_does_not_jump_to_glass_opacity() {
    let style = crate::Style::default();
    assert_eq!(style.backdrop_fill(Color::WHITE, 0.0), Color::WHITE);
    assert!(style.backdrop_fill(Color::WHITE, 0.01).0[3] >= 254);
    assert_eq!(style.backdrop_fill(Color::WHITE, 1.0).0[3], 184);
}
