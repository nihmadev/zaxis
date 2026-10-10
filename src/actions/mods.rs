use super::Platform;
use std::ops::BitOr;
use winit::keyboard::ModifiersState;

/// A set of modifier keys.
///
/// [`Mods::PRIMARY`] is the platform's command modifier, written once for every system:
/// Command on macOS, Control elsewhere. It resolves against the keymap's [`Platform`] when a
/// key is matched and when a shortcut is shown, so the application never branches on the
/// operating system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mods(u8);

impl Mods {
    pub const NONE: Self = Self(0);
    pub const CTRL: Self = Self(1);
    pub const SHIFT: Self = Self(2);
    pub const ALT: Self = Self(4);
    /// Command on macOS, the Windows key on Windows, Super elsewhere.
    pub const META: Self = Self(8);
    /// Command on macOS, Control elsewhere.
    pub const PRIMARY: Self = Self(16);

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// The set with [`Mods::PRIMARY`] replaced by the key it stands for on `platform`.
    pub fn concrete(self, platform: Platform) -> Self {
        if self.contains(Self::PRIMARY) {
            Self(self.0 & !Self::PRIMARY.0) | platform.primary()
        } else {
            self
        }
    }

    /// The modifiers held, as winit reports them. Never contains `PRIMARY`.
    pub fn from_state(state: ModifiersState) -> Self {
        let mut mods = Self::NONE;
        for (held, flag) in [
            (state.control_key(), Self::CTRL),
            (state.shift_key(), Self::SHIFT),
            (state.alt_key(), Self::ALT),
            (state.super_key(), Self::META),
        ] {
            if held {
                mods = mods | flag;
            }
        }
        mods
    }

    /// Names in the saved format, in its fixed order.
    pub(crate) fn names(self) -> impl Iterator<Item = &'static str> {
        [
            (Self::PRIMARY, "mod"),
            (Self::CTRL, "ctrl"),
            (Self::ALT, "alt"),
            (Self::SHIFT, "shift"),
            (Self::META, "meta"),
        ]
        .into_iter()
        .filter(move |(flag, _)| self.contains(*flag))
        .map(|(_, name)| name)
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "mod" | "primary" => Self::PRIMARY,
            "ctrl" | "control" => Self::CTRL,
            "shift" => Self::SHIFT,
            "alt" | "option" | "opt" => Self::ALT,
            "meta" | "cmd" | "command" | "super" | "win" => Self::META,
            _ => return None,
        })
    }
}

impl BitOr for Mods {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
