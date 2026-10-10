//! Which key a press counts as, for shortcuts.
//!
//! Letters are matched by the Latin letter the key produces, so a shortcut follows the
//! printed letter on Dvorak and AZERTY. A layout that produces no Latin letter (Russian,
//! Greek, Hebrew) falls back to the physical position, named by the US key: `Ctrl+S` is the
//! key that carries `ы` on a Russian layout. Everything else (digits, arrows, function and
//! punctuation keys) is matched by physical key.

use winit::keyboard::{Key, KeyCode};

const LETTERS: [KeyCode; 26] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
];

/// The letter key a logical key produces, if it is one Latin letter.
pub(crate) fn latin_letter(logical: &Key) -> Option<KeyCode> {
    let Key::Character(text) = logical else {
        return None;
    };
    let mut chars = text.chars();
    let (Some(letter), None) = (chars.next(), chars.next()) else {
        return None;
    };
    letter
        .is_ascii_alphabetic()
        .then(|| LETTERS[usize::from(letter.to_ascii_lowercase() as u8 - b'a')])
}
