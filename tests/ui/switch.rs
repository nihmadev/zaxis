use crate::prelude::*;
use std::time::Duration;
use winit::{
    dpi::PhysicalSize,
    event::ElementState,
    keyboard::{KeyCode, ModifiersState},
};
use zaxis::{Switch, Window};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}
fn draw(c: &mut Context, on: &mut bool, enabled: bool, at: u64) -> zaxis::Response {
    let start = c.frame_time();
    let mut response = None;
    c.run_at(start + Duration::from_millis(at), |c| {
        Window::new("Test").show(c, |ui| {
            ui.button("Before");
            response = Some(ui.add(Switch::new(on, "Wi-Fi").enabled(enabled)));
        });
    });
    response.unwrap()
}
fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}

#[test]
fn pointer_on_track_or_label_toggles_once_and_release_outside_cancels() {
    let mut c = setup();
    let mut on = false;
    let r = draw(&mut c, &mut on, true, 0);
    click(&mut c, r.rect.min + Vec2::new(4.0, 4.0));
    assert!(draw(&mut c, &mut on, true, 1).changed());
    assert!(on);
    assert!(!draw(&mut c, &mut on, true, 2).changed());
    click(&mut c, Vec2::new(r.rect.max.x - 2.0, r.rect.center().y));
    draw(&mut c, &mut on, true, 3);
    assert!(!on);
    c.move_pointer(r.rect.center());
    c.primary_button(ElementState::Pressed);
    c.move_pointer(Vec2::new(700.0, 500.0));
    c.primary_button(ElementState::Released);
    draw(&mut c, &mut on, true, 4);
    assert!(!on);
}

#[test]
fn keyboard_space_and_enter_toggle_focused_switch_and_disabled_ignores_input() {
    let mut c = setup();
    let mut on = false;
    draw(&mut c, &mut on, true, 0);
    c.set_modifiers(ModifiersState::empty());
    for _ in 0..2 {
        c.key(KeyCode::Tab, ElementState::Pressed, false);
        c.key(KeyCode::Tab, ElementState::Released, false);
    }
    assert!(draw(&mut c, &mut on, true, 1).has_focus);
    for key in [KeyCode::Space, KeyCode::Enter] {
        c.key(key, ElementState::Pressed, false);
        c.key(key, ElementState::Released, false);
        draw(&mut c, &mut on, true, 2);
    }
    assert!(!on, "two activations return to the original value");
    c.key(KeyCode::Space, ElementState::Pressed, false);
    c.key(KeyCode::Space, ElementState::Released, false);
    draw(&mut c, &mut on, true, 3);
    assert!(on);
    let r = draw(&mut c, &mut on, false, 4);
    click(&mut c, r.rect.center());
    assert!(!draw(&mut c, &mut on, false, 5).changed());
    assert!(on, "disabled preserves the value");
}

#[test]
fn thumb_slides_then_settles_and_reduced_motion_snaps_without_redraws() {
    let mut c = setup();
    let mut on = false;
    let r = draw(&mut c, &mut on, true, 0);
    let id = r.id.with("thumb-position");
    assert_eq!(c.sample_animation::<f32>(id).unwrap().value, 0.0);
    click(&mut c, r.rect.min + Vec2::new(4.0, 4.0));
    draw(&mut c, &mut on, true, 10);
    draw(&mut c, &mut on, true, 90);
    let mid = c.sample_animation::<f32>(id).unwrap().value;
    assert!(mid > 0.0 && mid < 1.0, "mid-transition value {mid}");
    draw(&mut c, &mut on, true, 1000);
    draw(&mut c, &mut on, true, 1001);
    assert_eq!(c.sample_animation::<f32>(id).unwrap().value, 1.0);
    assert!(!c.needs_repaint_at(c.frame_time() + Duration::from_secs(10)));
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    click(&mut c, r.rect.min + Vec2::new(4.0, 4.0));
    draw(&mut c, &mut on, true, 1100);
    draw(&mut c, &mut on, true, 1101);
    assert_eq!(c.sample_animation::<f32>(id).unwrap().value, 0.0);
    assert!(!c.needs_repaint_at(c.frame_time() + Duration::from_secs(10)));
}
