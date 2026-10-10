//! A shortcut editor built from `KeyBox::chord`, `Keymap::rebind` and `Keymap::conflicts`.

use super::support::*;
use crate::prelude::*;
use winit::keyboard::{Key, ModifiersState};
use zaxis::{Chord, KeyBox};

fn draw(c: &mut Context, chord: &mut Chord, at: Instant, sequence: bool) -> bool {
    let mut changed = false;
    pass_at(c, at, |ui| {
        changed = ui
            .add(KeyBox::chord(chord, "Save").sequence(sequence))
            .changed();
    });
    changed
}

fn listen(c: &mut Context, chord: &mut Chord, at: Instant, sequence: bool) {
    draw(c, chord, at, sequence);
    let spot = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.action != HitAction::Block)
        .unwrap()
        .rect
        .center();
    c.move_pointer(spot);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    draw(c, chord, at, sequence);
    assert!(c.key_capture_active());
}

#[test]
fn the_box_records_a_key_with_its_modifiers() {
    let mut c = setup();
    let mut chord = Chord::default();
    let t = Instant::now();
    listen(&mut c, &mut chord, t, false);
    let both = CTRL | ModifiersState::SHIFT;
    press(&mut c, both, KeyCode::KeyP);
    assert!(draw(&mut c, &mut chord, t, false), "changed");
    assert_eq!(chord.to_string(), "ctrl+shift+p");
    assert!(!c.key_capture_active());
    assert_eq!(
        taken(&mut c, &[Cmd::Save]),
        [],
        "recording did not run an action"
    );
}

#[test]
fn the_box_records_the_key_as_the_keyboard_will_match_it() {
    let mut c = setup();
    let mut chord = Chord::default();
    let t = Instant::now();
    listen(&mut c, &mut chord, t, false);
    // Ctrl+ы on a Russian layout is the S key.
    press_as(&mut c, CTRL, KeyCode::KeyS, Key::Character("ы".into()));
    draw(&mut c, &mut chord, t, false);
    assert_eq!(chord.to_string(), "ctrl+s");
}

#[test]
fn a_sequence_waits_for_a_second_stroke_then_for_the_timeout() {
    let mut c = setup();
    let mut chord = Chord::default();
    let t = Instant::now();
    listen(&mut c, &mut chord, t, true);
    press(&mut c, CTRL, KeyCode::KeyK);
    assert!(!draw(&mut c, &mut chord, t, true), "still listening");
    assert!(c.key_capture_active());
    press(&mut c, CTRL, KeyCode::KeyC);
    assert!(draw(&mut c, &mut chord, t, true));
    assert_eq!(chord.to_string(), "ctrl+k ctrl+c");

    // One stroke alone is taken once the chord timeout passes.
    let mut single = Chord::default();
    listen(&mut c, &mut single, t, true);
    press(&mut c, CTRL, KeyCode::KeyK);
    draw(&mut c, &mut single, t, true);
    let timeout = c.actions().keymap().chord_timeout();
    assert!(!draw(&mut c, &mut single, t + timeout / 2, true));
    assert!(draw(&mut c, &mut single, t + timeout * 2, true));
    assert_eq!(single.to_string(), "ctrl+k");
}

#[test]
fn escape_cancels_and_backspace_clears() {
    let mut c = setup();
    let mut chord: Chord = "ctrl+k".parse().unwrap();
    let t = Instant::now();
    listen(&mut c, &mut chord, t, false);
    press(&mut c, ModifiersState::empty(), KeyCode::Escape);
    assert!(!draw(&mut c, &mut chord, t, false));
    assert_eq!(chord.to_string(), "ctrl+k");
    assert!(!c.key_capture_active());

    listen(&mut c, &mut chord, t, false);
    press(&mut c, ModifiersState::empty(), KeyCode::Backspace);
    assert!(draw(&mut c, &mut chord, t, false));
    assert!(chord.is_empty());
}

#[test]
fn recording_then_rebinding_shows_the_conflict_and_the_new_key_works() {
    let mut c = setup();
    let t = Instant::now();
    let mut chord: Chord = "ctrl+s".parse().unwrap();
    listen(&mut c, &mut chord, t, false);
    press(&mut c, CTRL, KeyCode::KeyO);
    draw(&mut c, &mut chord, t, false);
    // The application applies what the box recorded and lists the clashes.
    let clashes = c
        .actions()
        .keymap_mut()
        .rebind(Cmd::Save, Some(chord.clone()));
    assert_eq!(clashes.len(), 1);
    assert_eq!(
        (clashes[0].first, clashes[0].second),
        (Id::new(Cmd::Save), Id::new(Cmd::Open))
    );
    // The action registered first keeps the key until the user moves the other one.
    c.actions()
        .keymap_mut()
        .rebind(Cmd::Open, Some("ctrl+shift+o".parse().unwrap()));
    assert!(c.actions().keymap().conflicts().is_empty());
    assert!(press(&mut c, CTRL, KeyCode::KeyO));
    assert_eq!(taken(&mut c, &[Cmd::Save, Cmd::Open]), [Cmd::Save]);
    assert!(press(&mut c, CTRL | ModifiersState::SHIFT, KeyCode::KeyO));
    assert_eq!(taken(&mut c, &[Cmd::Save, Cmd::Open]), [Cmd::Open]);
}
