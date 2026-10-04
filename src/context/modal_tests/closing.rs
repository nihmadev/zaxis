use super::*;

fn corner(s: &Scene) -> Vec2 {
    // Centre of the close button in the surface's top-right corner.
    vec2(s.surface.max.x - 8.0 - 14.0, s.surface.min.y + 8.0 + 14.0)
}

fn quiet(c: &mut Context, s: &mut Scene) {
    for _ in 0..3 {
        draw(c, s);
    }
}

#[test]
fn escape_closes_once() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    assert!(press(&mut c, &mut s, KeyCode::Escape));
    quiet(&mut c, &mut s);
    assert!(!s.open);
    assert_eq!(s.closed, vec![CloseReason::Escape]);
}

#[test]
fn overlay_click_closes_once_and_does_not_fall_through() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    open(&mut c, &mut s);
    let p = center(s.under);
    click(&mut c, &mut s, p);
    quiet(&mut c, &mut s);
    assert!(!s.open);
    assert_eq!(s.closed, vec![CloseReason::Overlay]);
    assert_eq!(
        s.under_clicks, 0,
        "the closing click must not reach the button below"
    );
    // The next click does reach it.
    click(&mut c, &mut s, p);
    assert_eq!(s.under_clicks, 1);
}

#[test]
fn release_after_a_keyboard_close_does_not_click_below() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    open(&mut c, &mut s);
    let p = center(s.under);
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    draw(&mut c, &mut s);
    press(&mut c, &mut s, KeyCode::Escape);
    c.primary_button(ElementState::Released);
    quiet(&mut c, &mut s);
    assert_eq!(s.under_clicks, 0);
}

#[test]
fn close_button_closes_with_its_own_reason() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    let p = corner(&s);
    click(&mut c, &mut s, p);
    quiet(&mut c, &mut s);
    assert_eq!(s.closed, vec![CloseReason::CloseButton]);
}

#[test]
fn every_close_condition_is_configured_separately() {
    let mut c = setup();
    let mut s = Scene {
        escape: false,
        ..Scene::default()
    };
    open(&mut c, &mut s);
    assert!(
        press(&mut c, &mut s, KeyCode::Escape),
        "Escape is still consumed"
    );
    assert!(s.open && s.closed.is_empty());
    s.escape = true;
    s.overlay = false;
    let p = center(s.under);
    click(&mut c, &mut s, p);
    assert!(s.open && s.closed.is_empty());
    press(&mut c, &mut s, KeyCode::Escape);
    assert_eq!(s.closed, vec![CloseReason::Escape]);
}

#[test]
fn escape_closes_a_nested_popup_before_the_modal() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    let p = center(s.combo_rect);
    click(&mut c, &mut s, p);
    assert!(
        c.popup.is_some(),
        "the combo box popup opened inside the modal"
    );
    press(&mut c, &mut s, KeyCode::Escape);
    assert!(c.popup.is_none());
    assert!(s.open && s.closed.is_empty());
    press(&mut c, &mut s, KeyCode::Escape);
    assert!(!s.open);
    assert_eq!(s.closed, vec![CloseReason::Escape]);
}

#[test]
fn nested_modals_stack_input_escape_and_focus() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    open(&mut c, &mut s);
    let outer = c.top_modal_id().unwrap();
    let focus_outer = c.focused_widget;
    s.nested = true;
    settle(&mut c, &mut s);
    assert_eq!(c.modals.stack.len(), 2);
    let top = c.top_modal_id().unwrap();
    assert_ne!(top, outer);
    // Only the top modal takes input.
    let p = center(s.inner);
    click(&mut c, &mut s, p);
    assert_eq!(s.inner_clicks, 0);
    assert!(
        s.open,
        "a click outside the nested modal does not reach the outer overlay"
    );
    assert_eq!(s.nested_closed, vec![CloseReason::Overlay]);
    // Reopen and close by Escape: one level at a time.
    s.nested = true;
    s.nested_closed.clear();
    settle(&mut c, &mut s);
    press(&mut c, &mut s, KeyCode::Escape);
    assert_eq!(s.nested_closed, vec![CloseReason::Escape]);
    assert!(s.open && s.closed.is_empty());
    assert_eq!(c.top_modal_id(), Some(outer));
    assert_eq!(c.focused_widget, focus_outer);
    press(&mut c, &mut s, KeyCode::Escape);
    assert_eq!(s.closed, vec![CloseReason::Escape]);
    assert!(c.modals.stack.is_empty());
}

#[test]
fn closed_modal_leaves_no_state_hits_or_layers() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    let id = c.top_modal_id().unwrap();
    press(&mut c, &mut s, KeyCode::Escape);
    quiet(&mut c, &mut s);
    assert!(c.modals.stack.is_empty());
    assert!(c.modals.measures.is_empty());
    assert!(!c.windows.contains_key(&id));
    assert!(c.popup_layers.is_empty());
    assert!(c.previous_hits.iter().all(|h| h.window != id));
    assert!(c.elements.iter().all(|e| e.layer != id));
    assert!(c.animation_status(id.with("presence")).is_none());
    assert!(!c.input_blocked());
}

#[test]
fn reopening_works_after_closing() {
    let mut c = setup();
    let mut s = Scene::default();
    for round in 1..=3 {
        open(&mut c, &mut s);
        press(&mut c, &mut s, KeyCode::Escape);
        quiet(&mut c, &mut s);
        assert_eq!(s.closed.len(), round);
    }
}
