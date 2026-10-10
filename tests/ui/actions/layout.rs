//! Letters follow the Latin letter a key produces, and fall back to the physical key on a
//! layout without Latin letters. Everything else is physical.

use super::support::*;
use crate::prelude::*;
use winit::keyboard::{Key, ModifiersState};

#[test]
fn a_latin_layout_matches_by_the_letter() {
    let mut c = setup();
    idle(&mut c);
    assert!(press_as(
        &mut c,
        CTRL,
        KeyCode::KeyS,
        Key::Character("s".into())
    ));
    assert_eq!(taken(&mut c, &[Cmd::Save]), [Cmd::Save]);
    // Shift (caps) gives an upper case letter for the same key.
    assert!(press_as(
        &mut c,
        CTRL,
        KeyCode::KeyS,
        Key::Character("S".into())
    ));
}

#[test]
fn a_dvorak_key_follows_its_letter_not_its_position() {
    let mut c = setup();
    idle(&mut c);
    // The key at the QWERTY `O` position prints `s` on Dvorak: Ctrl+S saves.
    assert!(press_as(
        &mut c,
        CTRL,
        KeyCode::KeyO,
        Key::Character("s".into())
    ));
    assert_eq!(taken(&mut c, &[Cmd::Save, Cmd::Open]), [Cmd::Save]);
}

#[test]
fn a_russian_layout_falls_back_to_the_physical_letter_key() {
    let mut c = setup();
    idle(&mut c);
    // `ы` sits where `S` does on QWERTY: Ctrl+ы is Ctrl+S.
    assert!(press_as(
        &mut c,
        CTRL,
        KeyCode::KeyS,
        Key::Character("ы".into())
    ));
    assert_eq!(taken(&mut c, &[Cmd::Save]), [Cmd::Save]);
    // `щ` is the `O` key.
    assert!(press_as(
        &mut c,
        CTRL,
        KeyCode::KeyO,
        Key::Character("щ".into())
    ));
    assert_eq!(taken(&mut c, &[Cmd::Open]), [Cmd::Open]);
    // And so is a capital letter.
    assert!(press_as(
        &mut c,
        CTRL,
        KeyCode::KeyS,
        Key::Character("Ы".into())
    ));
}

#[test]
fn chords_work_on_a_russian_layout() {
    let mut c = setup();
    idle(&mut c);
    assert!(press_as(
        &mut c,
        CTRL,
        KeyCode::KeyK,
        Key::Character("л".into())
    ));
    assert!(press_as(
        &mut c,
        CTRL,
        KeyCode::KeyB,
        Key::Character("и".into())
    ));
    assert_eq!(taken(&mut c, &[Cmd::Sidebar]), [Cmd::Sidebar]);
}

#[test]
fn function_keys_digits_and_arrows_are_physical() {
    let mut c = setup();
    idle(&mut c);
    let none = ModifiersState::empty();
    assert!(press_as(
        &mut c,
        none,
        KeyCode::F5,
        Key::Named(winit::keyboard::NamedKey::F5)
    ));
    assert!(press_as(
        &mut c,
        none,
        KeyCode::Delete,
        Key::Named(winit::keyboard::NamedKey::Delete)
    ));
    assert_eq!(
        taken(&mut c, &[Cmd::Reload, Cmd::Delete]),
        [Cmd::Reload, Cmd::Delete]
    );
}

#[test]
fn a_key_that_produces_no_single_letter_is_physical() {
    let mut c = setup();
    idle(&mut c);
    // A dead key or a multi-letter string is not a letter shortcut.
    assert!(press_as(&mut c, CTRL, KeyCode::KeyS, Key::Dead(Some('`'))));
    assert!(press_as(
        &mut c,
        CTRL,
        KeyCode::KeyS,
        Key::Character("ss".into())
    ));
}
