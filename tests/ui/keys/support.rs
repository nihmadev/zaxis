//! A context, keys sent the way a window sends them, and external widgets built from the
//! public API only.

use crate::prelude::*;
use winit::{
    dpi::PhysicalSize,
    event::ElementState,
    keyboard::{Key, ModifiersState, NamedKey, NativeKey, PhysicalKey},
};
use zaxis::{KeyEvent, KeyInterest, Response, Root, Sense, Ui};

pub fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}

/// A region that takes focus and claims `interest`; the events it got go to `log`.
pub fn pad(
    ui: &mut Ui<'_>,
    name: &str,
    interest: KeyInterest<'_>,
    log: &mut Vec<KeyEvent>,
) -> Response {
    let rect = ui.allocate_space(vec2(90.0, 30.0));
    let response = ui.interact(rect, name, Sense::CLICK | Sense::FOCUS);
    log.extend(ui.keys(&response, interest));
    response
}

/// One pass of a root with the given content.
pub fn pass(c: &mut Context, build: impl FnOnce(&mut Ui<'_>)) {
    c.run(|c| {
        Root::new().show(c, build);
    });
}

fn logical(code: KeyCode) -> Key {
    let name = format!("{code:?}");
    match code {
        KeyCode::ArrowLeft => Key::Named(NamedKey::ArrowLeft),
        KeyCode::ArrowRight => Key::Named(NamedKey::ArrowRight),
        KeyCode::ArrowUp => Key::Named(NamedKey::ArrowUp),
        KeyCode::ArrowDown => Key::Named(NamedKey::ArrowDown),
        KeyCode::Home => Key::Named(NamedKey::Home),
        KeyCode::End => Key::Named(NamedKey::End),
        KeyCode::Enter => Key::Named(NamedKey::Enter),
        KeyCode::Space => Key::Named(NamedKey::Space),
        KeyCode::Tab => Key::Named(NamedKey::Tab),
        KeyCode::Escape => Key::Named(NamedKey::Escape),
        _ => match name.strip_prefix("Key").filter(|rest| rest.len() == 1) {
            Some(letter) => Key::Character(letter.to_lowercase().into()),
            None => Key::Unidentified(NativeKey::Unidentified),
        },
    }
}

/// An event as a window delivers it, with the logical key of a Latin layout.
pub fn key_input(code: KeyCode, state: ElementState, repeat: bool) -> InputEvent {
    key_as(code, logical(code), state, repeat, None)
}

pub fn key_as(
    code: KeyCode,
    logical: Key,
    state: ElementState,
    repeat: bool,
    text: Option<&str>,
) -> InputEvent {
    InputEvent::Key(KeyInput {
        physical: PhysicalKey::Code(code),
        logical,
        state,
        repeat,
        text: text.map(Into::into),
    })
}

/// Press `code`; returns whether the context consumed it.
pub fn down(c: &mut Context, code: KeyCode) -> bool {
    c.on_input(key_input(code, ElementState::Pressed, false))
        .consumed
}
pub fn again(c: &mut Context, code: KeyCode) -> bool {
    c.on_input(key_input(code, ElementState::Pressed, true))
        .consumed
}
pub fn up(c: &mut Context, code: KeyCode) -> bool {
    c.on_input(key_input(code, ElementState::Released, false))
        .consumed
}

/// Press and release; returns whether the press was consumed.
pub fn tap(c: &mut Context, code: KeyCode) -> bool {
    let consumed = down(c, code);
    up(c, code);
    consumed
}

pub fn mods(c: &mut Context, state: ModifiersState) {
    c.on_input(InputEvent::Modifiers(state));
}

/// Click the middle of `rect` with the pointer.
pub fn click(c: &mut Context, at: Vec2) {
    c.on_input(InputEvent::PointerMoved {
        x: f64::from(at.x),
        y: f64::from(at.y),
    });
    for state in [ElementState::Pressed, ElementState::Released] {
        c.on_input(InputEvent::Button {
            button: winit::event::MouseButton::Left,
            state,
        });
    }
}

pub fn codes(events: &[KeyEvent]) -> Vec<KeyCode> {
    events.iter().map(|event| event.code).collect()
}
