//! X11 keys: Linux key codes to physical keys, and keysyms to the meaning of a key.

use crate::win32::scan_code_to_key;
use zaxis::winit::keyboard::{Key, KeyCode, NamedKey, NativeKey};

/// The physical key of an X keycode (an evdev code plus 8). The first block of evdev codes is
/// the same numbering as the PC scan codes, which Windows reports too.
pub(super) fn physical_key(x_keycode: u32) -> Option<KeyCode> {
    let code = x_keycode.checked_sub(8)? as u16;
    if (1..=0x58).contains(&code) {
        return scan_code_to_key(code, false);
    }
    Some(match code {
        96 => KeyCode::NumpadEnter,
        97 => KeyCode::ControlRight,
        98 => KeyCode::NumpadDivide,
        99 => KeyCode::PrintScreen,
        100 => KeyCode::AltRight,
        102 => KeyCode::Home,
        103 => KeyCode::ArrowUp,
        104 => KeyCode::PageUp,
        105 => KeyCode::ArrowLeft,
        106 => KeyCode::ArrowRight,
        107 => KeyCode::End,
        108 => KeyCode::ArrowDown,
        109 => KeyCode::PageDown,
        110 => KeyCode::Insert,
        111 => KeyCode::Delete,
        119 => KeyCode::Pause,
        125 => KeyCode::SuperLeft,
        126 => KeyCode::SuperRight,
        127 => KeyCode::ContextMenu,
        _ => return None,
    })
}

/// The character a keysym types: Latin-1 and Latin keysyms (their codes are the characters),
/// Unicode keysyms (`0x01000000 + code point`), the Cyrillic block, and the keypad's digits
/// and operators. Other keysyms type nothing.
pub(super) fn keysym_char(keysym: u32) -> Option<char> {
    const CYRILLIC_LOWER: &str = "юабцдефгхийклмнопярстужвьызшэщчъ";
    const CYRILLIC_UPPER: &str = "ЮАБЦДЕФГХИЙКЛМНОПЯРСТУЖВЬЫЗШЭЩЧЪ";
    match keysym {
        0x20..=0x7e | 0xa0..=0xff => char::from_u32(keysym),
        0x0100_0000..=0x0110_ffff => {
            char::from_u32(keysym - 0x0100_0000).filter(|c| !c.is_control())
        }
        0x06a3 => Some('ё'),
        0x06b3 => Some('Ё'),
        0x06c0..=0x06df => CYRILLIC_LOWER.chars().nth((keysym - 0x06c0) as usize),
        0x06e0..=0x06ff => CYRILLIC_UPPER.chars().nth((keysym - 0x06e0) as usize),
        0xff80 => Some(' '),
        0xffaa..=0xffb9 => char::from_u32(keysym - 0xffaa + 0x2a)
            .filter(|c| matches!(c, '*' | '+' | ',' | '-' | '.' | '/' | '0'..='9')),
        _ => None,
    }
}

/// What a key means, for navigation: the named key of a keysym, with the keypad's
/// NumLock-off keys as the arrows and editing keys they then are.
pub(super) fn logical_key(keysym: u32, text: Option<&str>) -> Key {
    let named = match keysym {
        0xff08 => NamedKey::Backspace,
        0xff09 | 0xfe20 => NamedKey::Tab,
        0xff0d | 0xff8d => NamedKey::Enter,
        0xff1b => NamedKey::Escape,
        0xff50 | 0xff95 => NamedKey::Home,
        0xff51 | 0xff96 => NamedKey::ArrowLeft,
        0xff52 | 0xff97 => NamedKey::ArrowUp,
        0xff53 | 0xff98 => NamedKey::ArrowRight,
        0xff54 | 0xff99 => NamedKey::ArrowDown,
        0xff55 | 0xff9a => NamedKey::PageUp,
        0xff56 | 0xff9b => NamedKey::PageDown,
        0xff57 | 0xff9c => NamedKey::End,
        0xff63 | 0xff9e => NamedKey::Insert,
        0xffff | 0xff9f => NamedKey::Delete,
        0xffbe..=0xffd5 => return Key::Named(function_key((keysym - 0xffbe) as usize)),
        _ => {
            return match text {
                Some(text) => Key::Character(text.into()),
                None => Key::Unidentified(NativeKey::Xkb(keysym)),
            }
        }
    };
    Key::Named(named)
}

fn function_key(index: usize) -> NamedKey {
    use NamedKey::*;
    const KEYS: [NamedKey; 24] = [
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
        F21, F22, F23, F24,
    ];
    KEYS[index.min(KEYS.len() - 1)]
}

/// The keysym a key gives in the current state, from the server's keyboard mapping:
/// `keysyms_per_keycode` columns per key, two per layout group (without and with Shift).
pub(super) struct KeyMap {
    pub first_keycode: u8,
    pub per_keycode: usize,
    pub keysyms: Vec<u32>,
}

impl KeyMap {
    pub fn keysym(&self, x_keycode: u32, group: u8, shift: bool, caps: bool, numlock: bool) -> u32 {
        let row =
            (x_keycode as usize).saturating_sub(usize::from(self.first_keycode)) * self.per_keycode;
        let at = |column: usize| {
            self.keysyms
                .get(row + column)
                .copied()
                .filter(|sym| *sym != 0)
        };
        let base = (usize::from(group) * 2).min(self.per_keycode.saturating_sub(2));
        let (lower, upper) = (
            at(base).unwrap_or(0),
            at(base + 1).or_else(|| at(base)).unwrap_or(0),
        );
        let keypad = (0xff80..=0xffbd).contains(&lower) || (0xff80..=0xffbd).contains(&upper);
        let letter = keysym_char(lower).is_some_and(|c| c.is_lowercase() && upper != lower);
        if keypad {
            // Keypad keys: the numeric keysym with NumLock on, the navigation one with it off.
            return if shift != numlock { upper } else { lower };
        }
        if letter && caps {
            if shift {
                lower
            } else {
                upper
            }
        } else if shift {
            upper
        } else {
            lower
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_keycodes_are_evdev_plus_eight() {
        assert_eq!(physical_key(38), Some(KeyCode::KeyA));
        assert_eq!(physical_key(9), Some(KeyCode::Escape));
        assert_eq!(physical_key(113), Some(KeyCode::ArrowLeft));
        assert_eq!(physical_key(104), Some(KeyCode::NumpadEnter));
        assert_eq!(physical_key(2), None);
    }

    #[test]
    fn keysyms_type_latin_unicode_and_cyrillic() {
        assert_eq!(keysym_char(0x61), Some('a'));
        assert_eq!(keysym_char(0xe9), Some('é'));
        assert_eq!(keysym_char(0x0100_20ac), Some('€'));
        assert_eq!(keysym_char(0x06c1), Some('а'));
        assert_eq!(keysym_char(0x06e1), Some('А'));
        assert_eq!(keysym_char(0x06a3), Some('ё'));
        assert_eq!(keysym_char(0xff0d), None);
        assert_eq!(keysym_char(0xffb5), Some('5'));
    }

    #[test]
    fn navigation_keysyms_have_names_and_the_keypad_follows_numlock() {
        assert_eq!(logical_key(0xff51, None), Key::Named(NamedKey::ArrowLeft));
        assert_eq!(logical_key(0xff96, None), Key::Named(NamedKey::ArrowLeft));
        assert_eq!(logical_key(0xffbe, None), Key::Named(NamedKey::F1));
        assert_eq!(logical_key(0x61, Some("a")), Key::Character("a".into()));
    }

    #[test]
    fn shift_caps_and_groups_pick_the_column() {
        // keycode 24: group 1 q/Q, group 2 cyrillic й/Й.
        let map = KeyMap {
            first_keycode: 8,
            per_keycode: 4,
            keysyms: [vec![0; 16 * 4], vec![0x71, 0x51, 0x06ca, 0x06ea]].concat(),
        };
        assert_eq!(map.keysym(24, 0, false, false, false), 0x71);
        assert_eq!(map.keysym(24, 0, true, false, false), 0x51);
        assert_eq!(
            map.keysym(24, 0, false, true, false),
            0x51,
            "caps lock capitalizes letters"
        );
        assert_eq!(map.keysym(24, 0, true, true, false), 0x71);
        assert_eq!(map.keysym(24, 1, false, false, false), 0x06ca);
    }
}
