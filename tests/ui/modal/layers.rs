use super::*;
use std::time::Duration;

fn animated() -> Context {
    let mut c = setup();
    let mut style = c.style().clone();
    style.motion.reduced_motion = false;
    c.set_style(style);
    c
}

#[test]
fn popup_inside_the_modal_draws_and_takes_input_above_it() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    let modal = c.top_modal_id().unwrap();
    let p = center(s.combo_rect);
    click(&mut c, &mut s, p);
    let popup = c.probe().popup.as_ref().expect("combo popup is open").id;
    assert!(c.layer_rank(popup) > c.layer_rank(modal));
    let row = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.window == popup && h.action == HitAction::Activate)
        .map(|h| center(h.rect.intersect(h.clip)))
        .expect("an option row");
    assert_eq!(c.hit_test(row).map(|h| h.window), Some(popup));
    click(&mut c, &mut s, row);
    assert!(s.combo.is_some(), "the option under the popup was chosen");
    assert!(s.open && s.closed.is_empty());
}

#[test]
fn popup_open_under_the_modal_is_closed_when_it_opens() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    let p = center(s.under_combo_rect);
    click(&mut c, &mut s, p);
    assert!(c.probe().popup.is_some());
    s.open = true;
    draw(&mut c, &mut s);
    assert!(c.probe().popup.is_none());
    assert!(c.probe().modals.stack.len() == 1);
    // And it cannot be reopened through the overlay.
    click(&mut c, &mut s, p);
    assert!(c.probe().popup.is_none());
}

#[test]
fn local_theme_reaches_the_modal_like_a_popup() {
    let mut c = setup();
    let light = zaxis::Theme::light();
    let mut open = true;
    let mut seen = None;
    for _ in 0..3 {
        c.run(|c| {
            Root::new().show(c, |ui| {
                ui.with_theme(&light, |ui| {
                    Modal::new("themed").show(ui, &mut open, |ui| {
                        seen = Some(ui.style().text_color);
                    });
                });
            });
        });
    }
    assert_eq!(seen, Some(light.resolve().text_color));
    assert_ne!(seen, Some(c.style().text_color));
}

#[test]
fn surface_stays_centered_and_visible_through_resize_and_dpi() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    for (w, h, scale) in [
        (800, 600, 1.0),
        (360, 260, 1.0),
        (180, 140, 1.0),
        (1600, 1200, 2.0),
    ] {
        c.on_window_event(&winit::event::WindowEvent::Resized(PhysicalSize::new(w, h)));
        c.set_viewport(PhysicalSize::new(w, h), scale);
        settle(&mut c, &mut s);
        let viewport = c.viewport();
        assert!(s.surface.min.x >= viewport.min.x - 0.5 && s.surface.max.x <= viewport.max.x + 0.5);
        assert!(
            s.surface.max.y <= viewport.max.y + 0.5,
            "bottom stays on screen"
        );
        if viewport.size().x > 300.0 {
            let offset = (s.surface.center() - viewport.center()).abs();
            assert!(offset.x < 1.0 && offset.y < 1.0, "centered: {offset:?}");
        }
        assert_eq!(c.top_window(viewport.center()), c.top_modal_id());
    }
}

#[test]
fn long_body_scrolls_inside_the_surface() {
    let mut c = setup();
    let mut s = Scene {
        long_body: true,
        ..Scene::default()
    };
    open(&mut c, &mut s);
    let viewport = c.viewport();
    assert!(s.surface.size().y <= viewport.size().y - 32.0 + 0.5);
    let before = s.surface;
    c.move_pointer(center(s.surface));
    assert!(c.scroll_wheel(vec2(0.0, 120.0)));
    draw(&mut c, &mut s);
    let body = c
        .probe()
        .scrolling
        .states
        .iter()
        .find(|(_, state)| state.window == c.top_modal_id().unwrap())
        .map(|(_, state)| state.offset.y)
        .unwrap();
    assert!(body > 0.0, "the body scrolled");
    assert_eq!(s.surface, before, "the surface itself does not move");
    assert_eq!(s.scroll_offset, 0.0, "nothing below scrolled");
}

#[test]
fn reduced_motion_opens_and_closes_without_animation_frames() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    assert!(!c.wants_animation_frame());
    assert!(!c.needs_repaint(), "an open, still modal needs no frames");
    assert!(c.next_repaint().is_none());
}

#[test]
fn animation_keeps_content_mounted_then_unblocks_and_cleans_up() {
    let mut c = animated();
    let mut s = Scene::default();
    let t0 = Instant::now();
    draw_at(&mut c, &mut s, t0);
    s.open = true;
    for i in 1..=4 {
        draw_at(&mut c, &mut s, t0 + Duration::from_millis(i * 10));
    }
    assert!(c.wants_animation_frame() || c.next_repaint().is_some());
    let id = c.top_modal_id().unwrap();
    let settled = t0 + Duration::from_millis(1000);
    draw_at(&mut c, &mut s, settled);
    draw_at(&mut c, &mut s, settled + Duration::from_millis(16));
    assert!(!c.wants_animation_frame());
    assert!(
        c.animation_status(id.with("presence"))
            .is_some_and(|st| format!("{st:?}").contains("Completed"))
            || !c.wants_animation_frame()
    );
    // Close: content stays mounted while it fades, input is released at once.
    let under = center(s.under);
    c.move_pointer(under);
    s.open = false;
    let closing = settled + Duration::from_millis(100);
    draw_at(&mut c, &mut s, closing);
    assert!(c.probe().modals.stack.is_empty());
    assert!(
        c.probe().windows.contains_key(&id),
        "still mounted for the exit"
    );
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .all(|h| h.window != id || !h.action.focusable()));
    draw_at(&mut c, &mut s, closing + Duration::from_millis(20));
    assert!(
        s.under_hovered,
        "the layer below is hoverable during the exit"
    );
    let end = closing + Duration::from_millis(1500);
    draw_at(&mut c, &mut s, end);
    draw_at(&mut c, &mut s, end + Duration::from_millis(16));
    assert!(!c.probe().windows.contains_key(&id));
    assert!(c.probe().popup_layers.is_empty());
    assert!(c.probe().modals.measures.is_empty());
    assert!(c.animation_status(id.with("presence")).is_none());
    assert!(!c.wants_animation_frame());
}

#[test]
fn combo_popup_opens_inside_an_animated_modal() {
    let mut c = animated();
    let mut s = Scene::default();
    let t0 = Instant::now();
    s.open = true;
    let mut t = t0;
    for _ in 0..40 {
        t += Duration::from_millis(16);
        draw_at(&mut c, &mut s, t);
    }
    let p = center(s.combo_rect);
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    t += Duration::from_millis(16);
    draw_at(&mut c, &mut s, t);
    c.primary_button(ElementState::Released);
    for i in 0..30 {
        t += Duration::from_millis(16);
        draw_at(&mut c, &mut s, t);
        if i < 4 {
            // A popup animating inside the modal keeps asking for frames.
            assert!(
                c.wants_animation_frame() || c.next_repaint().is_some(),
                "frame {i}"
            );
        }
    }
    assert!(c.probe().popup.is_some(), "popup stays open");
    let first = c.probe().popup.as_ref().unwrap().rect.size().y;
    for _ in 0..60 {
        t += Duration::from_millis(16);
        draw_at(&mut c, &mut s, t);
    }
    let last = c.probe().popup.as_ref().unwrap().rect.size().y;
    assert!(
        first > 0.0 && last > 60.0,
        "the popup animation completes inside the modal"
    );
}
