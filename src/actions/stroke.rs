use super::{names, Chord, Mods, Platform};
use crate::KeyBinding;
use winit::keyboard::KeyCode;

/// One key press with the modifiers held: `Ctrl+S`, `Shift+F5`, `Escape`.
///
/// The key is the existing [`KeyBinding`]; only [`KeyBinding::Key`] can be pressed on a
/// keyboard, so a stroke with `None` or a mouse button is invalid and never matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stroke {
    pub mods: Mods,
    pub key: KeyBinding,
}

impl Stroke {
    pub fn new(mods: Mods, key: impl Into<KeyBinding>) -> Self {
        Self {
            mods,
            key: key.into(),
        }
    }

    /// The key code, if the stroke is a keyboard key.
    pub fn code(&self) -> Option<KeyCode> {
        match self.key {
            KeyBinding::Key(code) => Some(code),
            _ => None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.code().is_some()
    }

    /// The stroke with [`Mods::PRIMARY`] resolved for `platform`.
    pub fn concrete(self, platform: Platform) -> Self {
        Self {
            mods: self.mods.concrete(platform),
            key: self.key,
        }
    }

    /// A chord of this stroke followed by `next`: `Ctrl+K` then `Ctrl+C`.
    pub fn then(self, next: Stroke) -> Chord {
        Chord::new([self, next])
    }
}

impl Mods {
    /// The stroke of `key` with these modifiers held:
    /// `Mods::PRIMARY.key(KeyCode::KeyS)` is the platform's Save shortcut.
    pub fn key(self, key: impl Into<KeyBinding>) -> Stroke {
        Stroke::new(self, key)
    }
}

impl From<KeyCode> for Stroke {
    fn from(code: KeyCode) -> Self {
        Self::new(Mods::NONE, code)
    }
}

impl From<KeyBinding> for Stroke {
    fn from(key: KeyBinding) -> Self {
        Self::new(Mods::NONE, key)
    }
}

impl std::fmt::Display for Stroke {
    /// The saved form: `mod+shift+s`, `f5`, `ctrl+bracketleft`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for name in self.mods.names() {
            write!(f, "{name}+")?;
        }
        match self.key {
            KeyBinding::Key(code) => match names::name(code) {
                Some(name) => f.write_str(name),
                None => write!(f, "{}", format!("{code:?}").to_ascii_lowercase()),
            },
            _ => f.write_str("none"),
        }
    }
}
