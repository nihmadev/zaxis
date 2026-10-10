//! A context with a registry, and keys sent the way a window sends them.

use crate::prelude::*;
use std::time::Duration;
pub use winit::event::ElementState;
use winit::{
    dpi::PhysicalSize,
    keyboard::{Key, ModifiersState, PhysicalKey},
};
use zaxis::{Action, Actions, Mods, Platform, Root};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    Save,
    Open,
    Find,
    FindNext,
    Sidebar,
    Delete,
    Reload,
}

pub const CTRL: ModifiersState = ModifiersState::CONTROL;

pub fn registry() -> Actions {
    let mut actions = Actions::new()
        .register(
            Action::new(Cmd::Save, "Save")
                .shortcut(Mods::PRIMARY.key(KeyCode::KeyS))
                .group("File"),
        )
        .register(Action::new(Cmd::Open, "Open").shortcut(Mods::PRIMARY.key(KeyCode::KeyO)))
        .register(
            Action::new(Cmd::Sidebar, "Sidebar").shortcut(
                Mods::PRIMARY
                    .key(KeyCode::KeyK)
                    .then(Mods::PRIMARY.key(KeyCode::KeyB)),
            ),
        )
        .register(
            Action::new(Cmd::Find, "Find").shortcut_in("editor", Mods::PRIMARY.key(KeyCode::KeyF)),
        )
        .register(
            Action::new(Cmd::FindNext, "Find next")
                .shortcut_in("editor/find", Mods::PRIMARY.key(KeyCode::KeyF)),
        )
        .register(Action::new(Cmd::Delete, "Delete").shortcut(KeyCode::Delete))
        .register(Action::new(Cmd::Reload, "Reload").shortcut(KeyCode::F5));
    actions.keymap_mut().set_platform(Platform::Linux);
    actions
}

pub fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(640, 420), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c.set_actions(registry());
    c
}

/// One pass of an empty root.
pub fn idle(c: &mut Context) {
    pass(c, |_| {});
}

pub fn pass(c: &mut Context, build: impl FnOnce(&mut zaxis::Ui<'_>)) {
    c.run(|c| {
        Root::new().show(c, build);
    });
}

pub fn pass_at(c: &mut Context, at: Instant, build: impl FnOnce(&mut zaxis::Ui<'_>)) {
    c.run_at(at, |c| {
        Root::new().show(c, build);
    });
}

fn letter(code: KeyCode) -> Option<Key> {
    let name = format!("{code:?}");
    let letter = name.strip_prefix("Key").filter(|rest| rest.len() == 1)?;
    Some(Key::Character(letter.to_lowercase().into()))
}

/// A key press as a window delivers it, with its logical key (the letter on the keycap
/// in a Latin layout).
pub fn key_event(code: KeyCode, logical: Key, state: ElementState) -> zaxis::InputEvent {
    zaxis::InputEvent::Key(zaxis::KeyInput {
        physical: PhysicalKey::Code(code),
        logical,
        state,
        repeat: false,
        text: None,
    })
}

/// Press and release `code` with `mods` held; returns whether the press was consumed.
pub fn press(c: &mut Context, mods: ModifiersState, code: KeyCode) -> bool {
    let logical =
        letter(code).unwrap_or(Key::Unidentified(winit::keyboard::NativeKey::Unidentified));
    press_as(c, mods, code, logical)
}

/// Like [`press`] with an explicit logical key, for a layout other than the US one.
pub fn press_as(c: &mut Context, mods: ModifiersState, code: KeyCode, logical: Key) -> bool {
    c.on_input(zaxis::InputEvent::Modifiers(mods));
    let down = c.on_input(key_event(code, logical.clone(), ElementState::Pressed));
    c.on_input(key_event(code, logical, ElementState::Released));
    c.on_input(zaxis::InputEvent::Modifiers(ModifiersState::empty()));
    down.consumed
}

/// The events of one pass: how many times each action answered `triggered`.
pub fn taken(c: &mut Context, which: &[Cmd]) -> Vec<Cmd> {
    let mut seen = Vec::new();
    pass(c, |ui| {
        for cmd in which {
            if ui.actions().triggered(*cmd) {
                seen.push(*cmd);
            }
        }
    });
    seen
}

pub fn later(base: Instant, millis: u64) -> Instant {
    base + Duration::from_millis(millis)
}
