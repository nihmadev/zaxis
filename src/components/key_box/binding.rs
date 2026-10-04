use winit::keyboard::KeyCode;

/// A key or mouse button bound to an action. `None` means unbound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyBinding {
    #[default]
    None,
    Key(KeyCode),
    Mouse(MouseBinding),
}

/// A mouse button a [`KeyBinding`] can hold. The primary button is only captured by a
/// `KeyBox` that allows it; the secondary and middle buttons are always captured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseBinding {
    Left,
    Right,
    Middle,
}

impl KeyBinding {
    /// Short caption in the style of game menus: `A`, `1`, `RShift`, `LCtrl`, `PgUp`,
    /// `Num5`, `LMB`, `RMB`, `MMB`; `None` for an unbound entry.
    pub fn label(self) -> String {
        match self {
            Self::None => "None".to_owned(),
            Self::Mouse(MouseBinding::Left) => "LMB".to_owned(),
            Self::Mouse(MouseBinding::Right) => "RMB".to_owned(),
            Self::Mouse(MouseBinding::Middle) => "MMB".to_owned(),
            Self::Key(code) => key_label(code),
        }
    }

    pub fn is_none(self) -> bool {
        self == Self::None
    }

    /// Whether the bound key is held or the bound mouse button is down right now.
    pub fn is_down(self, input: &crate::InputState) -> bool {
        match self {
            Self::None => false,
            Self::Key(code) => input.keys_down.contains(&code),
            Self::Mouse(MouseBinding::Left) => input.primary_down,
            Self::Mouse(MouseBinding::Right) => input.secondary_down,
            Self::Mouse(MouseBinding::Middle) => input.middle_down,
        }
    }

    /// Whether the bound key was pressed since the previous pass. Mouse buttons report
    /// the primary and secondary presses the input state records; the middle button has
    /// no press edge there, so it never reports one.
    pub fn is_pressed(self, input: &crate::InputState) -> bool {
        match self {
            Self::None => false,
            Self::Key(code) => input.keys_pressed.contains(&code),
            Self::Mouse(MouseBinding::Left) => input.primary_pressed,
            Self::Mouse(MouseBinding::Right) => input.secondary_pressed,
            Self::Mouse(MouseBinding::Middle) => false,
        }
    }
}

impl From<KeyCode> for KeyBinding {
    fn from(code: KeyCode) -> Self {
        Self::Key(code)
    }
}

impl std::fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}

fn key_label(code: KeyCode) -> String {
    let name = format!("{code:?}");
    let side = |name: &str, left: &str, right: &str| {
        name.strip_suffix("Left")
            .map(|base| format!("{left}{base}"))
            .or_else(|| {
                name.strip_suffix("Right")
                    .map(|base| format!("{right}{base}"))
            })
    };
    let named = match name.as_str() {
        "Escape" => Some("Esc"),
        "Backspace" => Some("Backspace"),
        "Enter" => Some("Enter"),
        "CapsLock" => Some("Caps"),
        "Insert" => Some("Ins"),
        "Delete" => Some("Del"),
        "PageUp" => Some("PgUp"),
        "PageDown" => Some("PgDn"),
        "ArrowUp" => Some("Up"),
        "ArrowDown" => Some("Down"),
        "ArrowLeft" => Some("Left"),
        "ArrowRight" => Some("Right"),
        "ControlLeft" => Some("LCtrl"),
        "ControlRight" => Some("RCtrl"),
        "ShiftLeft" => Some("LShift"),
        "ShiftRight" => Some("RShift"),
        "AltLeft" => Some("LAlt"),
        "AltRight" => Some("RAlt"),
        _ => None,
    };
    if let Some(label) = named {
        return label.to_owned();
    }
    if let Some(rest) = name.strip_prefix("Key").filter(|rest| rest.len() == 1) {
        return rest.to_owned();
    }
    if let Some(rest) = name.strip_prefix("Digit") {
        return rest.to_owned();
    }
    if let Some(rest) = name.strip_prefix("Numpad") {
        return format!("Num{rest}");
    }
    side(&name, "L", "R").unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_follow_menu_conventions() {
        for (binding, label) in [
            (KeyBinding::None, "None"),
            (KeyBinding::Key(KeyCode::KeyE), "E"),
            (KeyBinding::Key(KeyCode::Digit3), "3"),
            (KeyBinding::Key(KeyCode::ShiftRight), "RShift"),
            (KeyBinding::Key(KeyCode::ControlLeft), "LCtrl"),
            (KeyBinding::Key(KeyCode::PageDown), "PgDn"),
            (KeyBinding::Key(KeyCode::Numpad5), "Num5"),
            (KeyBinding::Key(KeyCode::F11), "F11"),
            (KeyBinding::Key(KeyCode::Space), "Space"),
            (KeyBinding::Mouse(MouseBinding::Right), "RMB"),
        ] {
            assert_eq!(binding.label(), label);
        }
    }

    #[test]
    fn held_and_pressed_read_the_input_state() {
        let mut input = crate::InputState::default();
        let key = KeyBinding::Key(KeyCode::KeyF);
        assert!(!key.is_down(&input) && !key.is_pressed(&input));
        input.keys_down.insert(KeyCode::KeyF);
        input.keys_pressed.insert(KeyCode::KeyF);
        assert!(key.is_down(&input) && key.is_pressed(&input));
        input.secondary_down = true;
        assert!(KeyBinding::Mouse(MouseBinding::Right).is_down(&input));
        assert!(!KeyBinding::None.is_down(&input));
    }
}
