use super::support::*;
use crate::prelude::*;
use winit::keyboard::ModifiersState;

const WATCHED: [Cmd; 3] = [Cmd::Sidebar, Cmd::Save, Cmd::Delete];

fn start(c: &mut Context) {
    idle(c);
    assert!(press(c, CTRL, KeyCode::KeyK), "the first stroke is taken");
}

#[test]
fn two_strokes_run_the_action_and_nothing_before_the_second() {
    let mut c = setup();
    start(&mut c);
    assert_eq!(taken(&mut c, &WATCHED), []);
    assert!(press(&mut c, CTRL, KeyCode::KeyB));
    assert_eq!(taken(&mut c, &WATCHED), [Cmd::Sidebar]);
    assert!(c.actions().pending_chord().is_none());
}

#[test]
fn a_waiting_chord_is_visible_for_a_status_bar() {
    let mut c = setup();
    start(&mut c);
    let pending = c.actions().pending_chord().expect("waiting");
    assert_eq!(pending.text, "Ctrl+K …");
    assert_eq!(pending.strokes.len(), 1);
    idle(&mut c);
    assert!(c.actions().pending_chord().is_some(), "it survives passes");
}

#[test]
fn a_chord_gives_up_after_its_timeout() {
    let mut c = setup();
    let t0 = Instant::now();
    pass_at(&mut c, t0, |_| {});
    press(&mut c, CTRL, KeyCode::KeyK);
    let timeout = c.actions().keymap().chord_timeout();
    pass_at(&mut c, t0 + timeout / 2, |_| {});
    assert!(c.actions().pending_chord().is_some());
    pass_at(
        &mut c,
        t0 + timeout + later(t0, 1).duration_since(t0),
        |_| {},
    );
    assert!(c.actions().pending_chord().is_none(), "expired");
    // The second stroke alone is nothing now.
    assert!(!press(&mut c, CTRL, KeyCode::KeyB));
    assert_eq!(taken(&mut c, &WATCHED), []);
}

#[test]
fn a_chord_asks_for_a_repaint_at_its_deadline() {
    let mut c = setup();
    idle(&mut c);
    press(&mut c, CTRL, KeyCode::KeyK);
    let timeout = c.actions().keymap().chord_timeout();
    let after = Instant::now() + timeout + later(Instant::now(), 50).duration_since(Instant::now());
    assert!(c.needs_repaint_at(after));
}

#[test]
fn escape_cancels_the_chord_and_is_used_up() {
    let mut c = setup();
    start(&mut c);
    assert!(press(&mut c, ModifiersState::empty(), KeyCode::Escape));
    assert!(c.actions().pending_chord().is_none());
    assert!(!press(&mut c, CTRL, KeyCode::KeyB), "the chord is over");
    assert_eq!(taken(&mut c, &WATCHED), []);
}

#[test]
fn another_key_resets_the_chord_and_is_not_used_again() {
    let mut c = setup();
    start(&mut c);
    // Ctrl+S would be Save on its own; as a wrong second stroke it only ends the chord.
    assert!(press(&mut c, CTRL, KeyCode::KeyS));
    assert!(c.actions().pending_chord().is_none());
    assert_eq!(taken(&mut c, &WATCHED), []);
    assert!(
        press(&mut c, CTRL, KeyCode::KeyS),
        "and works again afterwards"
    );
    assert_eq!(taken(&mut c, &WATCHED), [Cmd::Save]);
}

#[test]
fn losing_the_window_focus_drops_the_chord() {
    let mut c = setup();
    start(&mut c);
    c.on_input(zaxis::InputEvent::Focus(false));
    c.on_input(zaxis::InputEvent::Focus(true));
    assert!(c.actions().pending_chord().is_none());
    assert!(!press(&mut c, CTRL, KeyCode::KeyB));
    assert_eq!(taken(&mut c, &WATCHED), []);
}

#[test]
fn a_modifier_press_inside_a_chord_does_not_break_it() {
    let mut c = setup();
    start(&mut c);
    c.on_input(zaxis::InputEvent::Modifiers(CTRL));
    c.on_key_event(KeyCode::ControlLeft, ElementState::Pressed, false);
    c.on_key_event(KeyCode::ControlLeft, ElementState::Released, false);
    assert!(c.actions().pending_chord().is_some());
}

#[test]
fn the_release_of_a_used_key_is_used_too() {
    let mut c = setup();
    idle(&mut c);
    c.on_input(zaxis::InputEvent::Modifiers(CTRL));
    let key = winit::keyboard::Key::Character("s".into());
    c.on_input(key_event(KeyCode::KeyS, key.clone(), ElementState::Pressed));
    assert!(
        c.on_input(key_event(KeyCode::KeyS, key, ElementState::Released))
            .consumed
    );
    assert!(!c.input().keys_down.contains(&KeyCode::KeyS));
}
