//! Keyboard and wheel input.
use super::Rig;
use crate::prelude::*;
use winit::event::ElementState;

#[test]
fn the_carousel_is_one_tab_stop_and_arrows_turn_pages() {
    let mut rig = Rig::new(5);
    rig.pass();
    rig.c
        .on_key_event(KeyCode::Tab, ElementState::Pressed, false);
    rig.c
        .on_key_event(KeyCode::Tab, ElementState::Released, false);
    let out = rig.tick(16);
    assert!(
        out.response.has_focus && out.response.focus_visible,
        "Tab focuses it with a ring"
    );
    assert!(rig.key(KeyCode::ArrowRight).changed);
    assert_eq!(rig.page, 1);
    // Arrows across the axis do nothing.
    assert!(!rig.key(KeyCode::ArrowDown).changed);
    assert!(rig.key(KeyCode::ArrowLeft).changed);
    assert_eq!(rig.page, 0);
    // Only one change per key: the next pass reports nothing new.
    assert!(!rig.tick(16).changed);
}

#[test]
fn home_end_and_page_keys_jump_and_step() {
    let mut rig = Rig::new(6);
    rig.focus();
    assert!(rig.key(KeyCode::End).changed && rig.page == 5);
    assert!(rig.key(KeyCode::PageUp).changed && rig.page == 4);
    assert!(rig.key(KeyCode::PageDown).changed && rig.page == 5);
    assert!(
        !rig.key(KeyCode::ArrowRight).changed,
        "the end of a non-looping carousel"
    );
    assert!(rig.key(KeyCode::Home).changed && rig.page == 0);
    rig.settle();
    assert_eq!(rig.changes, 4);
}

#[test]
fn enter_and_space_activate_the_active_page() {
    let mut rig = Rig::new(3);
    rig.focus();
    rig.key(KeyCode::ArrowRight);
    for code in [KeyCode::Enter, KeyCode::Space] {
        rig.c.on_key_event(code, ElementState::Pressed, false);
        rig.c.on_key_event(code, ElementState::Released, false);
        let out = rig.tick(16);
        assert_eq!(out.activated, Some(1), "{code:?}");
        assert!(!out.changed);
    }
}

#[test]
fn a_horizontal_wheel_turns_pages_and_a_vertical_one_is_left_alone() {
    let mut rig = Rig::new(4);
    rig.settle();
    rig.c.move_pointer(rig.empty());
    rig.tick(16);
    assert!(
        !rig.c.scroll_wheel(Vec2::new(0.0, 28.0)),
        "vertical wheel goes to the area around"
    );
    assert!(!rig.tick(16).changed);
    assert!(rig.c.scroll_wheel(Vec2::new(14.0, 0.0)));
    let out = rig.tick(16);
    assert!(out.changed && out.page == 1, "one notch, one page");
    // Trackpad inertia right after a turn does not run through the pages.
    for _ in 0..5 {
        rig.c.scroll_wheel(Vec2::new(12.0, 0.0));
        assert!(!rig.tick(16).changed);
    }
    rig.tick(300);
    rig.c.scroll_wheel(Vec2::new(-14.0, 0.0));
    assert!(rig.tick(16).changed);
    assert_eq!(rig.page, 0);
}

#[test]
fn shift_and_the_wheel_turn_pages_like_a_horizontal_wheel() {
    let mut rig = Rig::new(4);
    rig.settle();
    rig.c.move_pointer(rig.empty());
    rig.tick(16);
    rig.c.set_modifiers(winit::keyboard::ModifiersState::SHIFT);
    rig.c
        .on_window_event(&winit::event::WindowEvent::MouseWheel {
            device_id: winit::event::DeviceId::dummy(),
            delta: winit::event::MouseScrollDelta::LineDelta(0.0, -1.0),
            phase: winit::event::TouchPhase::Moved,
        });
    let out = rig.tick(16);
    assert!(out.changed && out.page == 1);
}
