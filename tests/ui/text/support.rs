//! Shared helpers: a context with reduced motion, simulated pointer and keys, and one UI pass.
pub use crate::prelude::*;
pub use winit::event::ElementState;
use winit::{dpi::PhysicalSize, keyboard::ModifiersState};
use zaxis::Root;

pub fn setup_at(width: u32, height: u32, scale: f64) -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(width, height), scale);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
pub fn setup() -> Context {
    setup_at(640, 480, 1.0)
}

/// One pass in a root with the default padding.
pub fn frame<R>(c: &mut Context, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
    let mut out = None;
    c.run(|c| {
        Root::new().show(c, |ui| out = Some(build(ui)));
    });
    out.unwrap()
}

pub fn down(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
}
pub fn up(c: &mut Context) {
    c.primary_button(ElementState::Released);
}
pub fn click(c: &mut Context, p: Vec2) {
    down(c, p);
    up(c);
}
pub fn right_click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.secondary_button(ElementState::Pressed);
    c.secondary_button(ElementState::Released);
}
pub fn middle_click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.middle_button(ElementState::Pressed);
    c.middle_button(ElementState::Released);
}
pub fn key(c: &mut Context, code: KeyCode) {
    c.on_key_event(code, ElementState::Pressed, false);
    c.on_key_event(code, ElementState::Released, false);
}
pub fn chord(c: &mut Context, mods: ModifiersState, code: KeyCode) {
    c.set_modifiers(mods);
    key(c, code);
    c.set_modifiers(ModifiersState::empty());
}
pub fn ctrl(c: &mut Context, code: KeyCode) {
    chord(c, ModifiersState::CONTROL, code);
}

/// One pass at a chosen time, for tooltip delays and animation deadlines.
pub fn frame_at<R>(c: &mut Context, now: Instant, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
    let mut out = None;
    c.run_at(now, |c| {
        Root::new().show(c, |ui| out = Some(build(ui)));
    });
    out.unwrap()
}
