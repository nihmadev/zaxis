//! Virtual keys, modifiers and cursors.

use zaxis::winit::{
    event::MouseButton,
    keyboard::{Key, KeyCode, ModifiersState, NamedKey, NativeKey},
    window::CursorIcon,
};

/// The logical key a virtual-key code (`VK_*`) stands for. Letters and digits are lower-case
/// characters; text itself arrives through `WM_CHAR`. Codes without a name stay
/// `Unidentified` with the native code.
pub fn logical_key(vk: u16) -> Key {
    let named = match vk {
        0x08 => NamedKey::Backspace,
        0x09 => NamedKey::Tab,
        0x0D => NamedKey::Enter,
        0x10 => NamedKey::Shift,
        0x11 => NamedKey::Control,
        0x12 => NamedKey::Alt,
        0x13 => NamedKey::Pause,
        0x14 => NamedKey::CapsLock,
        0x1B => NamedKey::Escape,
        0x20 => NamedKey::Space,
        0x21 => NamedKey::PageUp,
        0x22 => NamedKey::PageDown,
        0x23 => NamedKey::End,
        0x24 => NamedKey::Home,
        0x25 => NamedKey::ArrowLeft,
        0x26 => NamedKey::ArrowUp,
        0x27 => NamedKey::ArrowRight,
        0x28 => NamedKey::ArrowDown,
        0x2C => NamedKey::PrintScreen,
        0x2D => NamedKey::Insert,
        0x2E => NamedKey::Delete,
        0x5B | 0x5C => NamedKey::Super,
        0x5D => NamedKey::ContextMenu,
        0x90 => NamedKey::NumLock,
        0x91 => NamedKey::ScrollLock,
        0x70..=0x87 => return Key::Named(function_key(vk - 0x70)),
        0x30..=0x39 => {
            return Key::Character(char::from(b'0' + (vk - 0x30) as u8).to_string().into())
        }
        0x41..=0x5A => {
            return Key::Character(char::from(b'a' + (vk - 0x41) as u8).to_string().into())
        }
        0x60..=0x69 => {
            return Key::Character(char::from(b'0' + (vk - 0x60) as u8).to_string().into())
        }
        _ => return Key::Unidentified(NativeKey::Windows(vk)),
    };
    Key::Named(named)
}

fn function_key(index: u16) -> NamedKey {
    use NamedKey::*;
    const KEYS: [NamedKey; 24] = [
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
        F21, F22, F23, F24,
    ];
    KEYS[usize::from(index)]
}

/// The modifier a key is, if it is one.
pub fn modifier_of(key: KeyCode) -> Option<ModifiersState> {
    Some(match key {
        KeyCode::ShiftLeft | KeyCode::ShiftRight => ModifiersState::SHIFT,
        KeyCode::ControlLeft | KeyCode::ControlRight => ModifiersState::CONTROL,
        KeyCode::AltLeft | KeyCode::AltRight => ModifiersState::ALT,
        KeyCode::SuperLeft | KeyCode::SuperRight => ModifiersState::SUPER,
        _ => return None,
    })
}

/// The `IDC_*` resource of the stock cursor that looks like `icon`.
pub fn cursor_resource(icon: CursorIcon) -> u32 {
    use CursorIcon::*;
    match icon {
        Text | VerticalText => 32513,
        Wait => 32514,
        Crosshair | Cell => 32515,
        NwseResize | NwResize | SeResize => 32642,
        NeswResize | NeResize | SwResize => 32643,
        EwResize | ColResize | EResize | WResize => 32644,
        NsResize | RowResize | NResize | SResize => 32645,
        Move | AllScroll | Grab | Grabbing => 32646,
        NotAllowed | NoDrop => 32648,
        Pointer | Alias => 32649,
        Progress => 32650,
        Help => 32651,
        _ => 32512,
    }
}

/// The mouse button an `XBUTTON` message names in the high word of its `wParam`.
pub(super) fn x_button(wparam: usize) -> MouseButton {
    match (wparam >> 16) & 0xFFFF {
        1 => MouseButton::Back,
        2 => MouseButton::Forward,
        other => MouseButton::Other(other as u16),
    }
}
