//! Names of keys in the saved keymap format, and their captions in shortcut text.
//!
//! A saved name is lowercase and made of letters and digits only (`s`, `f5`, `pageup`,
//! `bracketleft`), so a chord never needs quoting: `ctrl+k ctrl+bracketleft`.

use super::Platform;
use winit::keyboard::KeyCode;

macro_rules! keys {
    ($($code:ident => $name:literal),* $(,)?) => {
        const KEYS: &[(KeyCode, &str)] = &[$((KeyCode::$code, $name)),*];
    };
}

keys! {
    KeyA => "a", KeyB => "b", KeyC => "c", KeyD => "d", KeyE => "e", KeyF => "f", KeyG => "g",
    KeyH => "h", KeyI => "i", KeyJ => "j", KeyK => "k", KeyL => "l", KeyM => "m", KeyN => "n",
    KeyO => "o", KeyP => "p", KeyQ => "q", KeyR => "r", KeyS => "s", KeyT => "t", KeyU => "u",
    KeyV => "v", KeyW => "w", KeyX => "x", KeyY => "y", KeyZ => "z",
    Digit0 => "0", Digit1 => "1", Digit2 => "2", Digit3 => "3", Digit4 => "4",
    Digit5 => "5", Digit6 => "6", Digit7 => "7", Digit8 => "8", Digit9 => "9",
    F1 => "f1", F2 => "f2", F3 => "f3", F4 => "f4", F5 => "f5", F6 => "f6", F7 => "f7",
    F8 => "f8", F9 => "f9", F10 => "f10", F11 => "f11", F12 => "f12", F13 => "f13",
    F14 => "f14", F15 => "f15", F16 => "f16", F17 => "f17", F18 => "f18", F19 => "f19",
    F20 => "f20", F21 => "f21", F22 => "f22", F23 => "f23", F24 => "f24",
    Numpad0 => "numpad0", Numpad1 => "numpad1", Numpad2 => "numpad2", Numpad3 => "numpad3",
    Numpad4 => "numpad4", Numpad5 => "numpad5", Numpad6 => "numpad6", Numpad7 => "numpad7",
    Numpad8 => "numpad8", Numpad9 => "numpad9", NumpadAdd => "numpadadd",
    NumpadSubtract => "numpadsubtract", NumpadMultiply => "numpadmultiply",
    NumpadDivide => "numpaddivide", NumpadDecimal => "numpaddecimal",
    NumpadEnter => "numpadenter",
    Escape => "escape", Enter => "enter", Space => "space", Tab => "tab",
    Backspace => "backspace", Delete => "delete", Insert => "insert", Home => "home",
    End => "end", PageUp => "pageup", PageDown => "pagedown", ArrowUp => "up",
    ArrowDown => "down", ArrowLeft => "left", ArrowRight => "right",
    Comma => "comma", Period => "period", Slash => "slash", Backslash => "backslash",
    Semicolon => "semicolon", Quote => "quote", BracketLeft => "bracketleft",
    BracketRight => "bracketright", Minus => "minus", Equal => "equal",
    Backquote => "backquote", ContextMenu => "contextmenu", PrintScreen => "printscreen",
}

/// The saved name of a key, or `None` for a key the format has no name for.
pub(crate) fn name(code: KeyCode) -> Option<&'static str> {
    KEYS.iter()
        .find(|(key, _)| *key == code)
        .map(|(_, name)| *name)
}

/// The key a saved name stands for. The `esc`, `del`, `ins`, `return`, `pgup` and `pgdn`
/// spellings are accepted as well; they are never written.
pub(crate) fn code(name: &str) -> Option<KeyCode> {
    let name = match name {
        "esc" => "escape",
        "del" => "delete",
        "ins" => "insert",
        "return" => "enter",
        "pgup" => "pageup",
        "pgdn" => "pagedown",
        other => other,
    };
    KEYS.iter()
        .find(|(_, key)| *key == name)
        .map(|(code, _)| *code)
}

/// Whether the key is a modifier itself; pressing it alone never starts a shortcut.
pub(crate) fn is_modifier(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
            | KeyCode::Meta
            | KeyCode::Hyper
            | KeyCode::Fn
            | KeyCode::CapsLock
            | KeyCode::NumLock
            | KeyCode::ScrollLock
    )
}

/// The caption of a key inside shortcut text: `S`, `F5`, `Enter`, or `↩` on macOS.
pub(crate) fn caption(code: KeyCode, platform: Platform) -> String {
    let mac = platform == Platform::Mac;
    let symbol = match (code, mac) {
        (KeyCode::ArrowUp, true) => "↑",
        (KeyCode::ArrowDown, true) => "↓",
        (KeyCode::ArrowLeft, true) => "←",
        (KeyCode::ArrowRight, true) => "→",
        (KeyCode::Enter | KeyCode::NumpadEnter, true) => "↩",
        (KeyCode::Escape, true) => "⎋",
        (KeyCode::Backspace, true) => "⌫",
        (KeyCode::Delete, true) => "⌦",
        (KeyCode::Tab, true) => "⇥",
        (KeyCode::PageUp, true) => "⇞",
        (KeyCode::PageDown, true) => "⇟",
        (KeyCode::Home, true) => "↖",
        (KeyCode::End, true) => "↘",
        (KeyCode::ArrowUp, false) => "Up",
        (KeyCode::ArrowDown, false) => "Down",
        (KeyCode::ArrowLeft, false) => "Left",
        (KeyCode::ArrowRight, false) => "Right",
        (KeyCode::Escape, false) => "Esc",
        (KeyCode::Delete, false) => "Del",
        (KeyCode::Insert, false) => "Ins",
        (KeyCode::PageUp, false) => "PgUp",
        (KeyCode::PageDown, false) => "PgDn",
        (KeyCode::Comma, _) => ",",
        (KeyCode::Period, _) => ".",
        (KeyCode::Slash, _) => "/",
        (KeyCode::Backslash, _) => "\\",
        (KeyCode::Semicolon, _) => ";",
        (KeyCode::Quote, _) => "'",
        (KeyCode::BracketLeft, _) => "[",
        (KeyCode::BracketRight, _) => "]",
        (KeyCode::Minus, _) => "-",
        (KeyCode::Equal, _) => "=",
        (KeyCode::Backquote, _) => "`",
        _ => "",
    };
    if !symbol.is_empty() {
        return symbol.to_owned();
    }
    let Some(name) = name(code) else {
        return format!("{code:?}");
    };
    if let Some(digit) = name.strip_prefix("numpad").filter(|d| d.len() == 1) {
        return format!("Num{digit}");
    }
    let mut chars = name.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
