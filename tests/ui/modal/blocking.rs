use super::*;

#[test]
fn pointer_and_hover_stop_at_the_overlay() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    let under = center(s.under);
    c.move_pointer(under);
    draw(&mut c, &mut s);
    assert!(s.under_hovered);
    open(&mut c, &mut s);
    c.move_pointer(under);
    draw(&mut c, &mut s);
    assert!(
        !s.under_hovered,
        "hover must not reach controls under the overlay"
    );
    assert_eq!(c.top_window(under), c.top_modal_id());
    s.overlay = false;
    click(&mut c, &mut s, under);
    assert_eq!(s.under_clicks, 0);
    assert!(s.open, "overlay click with dismissal off keeps it open");
    let p = center(s.inner);
    click(&mut c, &mut s, p);
    assert_eq!(s.inner_clicks, 1);
}

#[test]
fn cursor_over_the_overlay_ignores_controls_below() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    let text = center(s.text_rect);
    c.move_pointer(text);
    assert_eq!(c.cursor_icon(), winit::window::CursorIcon::Text);
    open(&mut c, &mut s);
    c.move_pointer(text);
    assert_eq!(c.cursor_icon(), winit::window::CursorIcon::Default);
}

#[test]
fn wheel_is_consumed_and_never_scrolls_content_below() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    open(&mut c, &mut s);
    c.move_pointer(center(s.scroll_rect));
    let consumed = c.scroll_wheel(vec2(0.0, 80.0));
    draw(&mut c, &mut s);
    assert!(consumed);
    assert_eq!(s.scroll_offset, 0.0);
    // Without the modal the same wheel scrolls.
    s.open = false;
    settle(&mut c, &mut s);
    c.move_pointer(center(s.scroll_rect));
    c.scroll_wheel(vec2(0.0, 80.0));
    draw(&mut c, &mut s);
    assert!(s.scroll_offset > 0.0);
}

#[test]
fn shortcuts_text_and_keys_do_not_reach_the_application_below() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    press(&mut c, &mut s, KeyCode::F3);
    assert_eq!(s.shortcut, 1);
    open(&mut c, &mut s);
    press(&mut c, &mut s, KeyCode::F3);
    assert_eq!(
        s.shortcut, 1,
        "global shortcuts are idle while a modal is open"
    );
    assert!(c.input().keys_down.is_empty());
    assert!(c.input().pointer.is_none());
    // Typing goes only to the focused control inside the modal.
    let p = center(s.inner_text_rect);
    click(&mut c, &mut s, p);
    c.on_text_event("a");
    draw(&mut c, &mut s);
    assert_eq!(s.inner_text, "a");
    assert_eq!(s.under_text, "");
    s.open = false;
    settle(&mut c, &mut s);
    press(&mut c, &mut s, KeyCode::F3);
    assert_eq!(s.shortcut, 2);
}

#[test]
fn opening_cancels_capture_and_drag_without_changing_values() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    let p = vec2(s.slider_rect.min.x + 100.0, center(s.slider_rect).y);
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    draw(&mut c, &mut s);
    assert!(c.probe().capture.is_some());
    let value = s.slider;
    s.open = true;
    draw(&mut c, &mut s);
    assert!(
        c.probe().capture.is_none(),
        "capture ends when the modal opens"
    );
    c.move_pointer(p + vec2(40.0, 0.0));
    draw(&mut c, &mut s);
    assert_eq!(s.slider, value);
    c.primary_button(ElementState::Released);
    draw(&mut c, &mut s);
    assert_eq!(s.slider, value);
    assert_eq!(s.under_clicks, 0);
}

#[test]
fn underlying_ui_keeps_running_and_painting_every_pass() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    let before = c.cache_stats().ui_passes;
    draw(&mut c, &mut s);
    draw(&mut c, &mut s);
    assert_eq!(c.cache_stats().ui_passes, before + 2);
    assert!(s.under.size().x > 0.0, "controls below are still laid out");
    let modal = c.top_modal_id().unwrap();
    assert!(c.probe().elements.iter().any(|e| e.layer != modal));
}
