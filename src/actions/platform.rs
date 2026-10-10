use super::Mods;

/// The desktop family a shortcut is shown and resolved for. It decides what
/// [`Mods::PRIMARY`] stands for and how [`Kbd`](super::Kbd) writes a shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    Mac,
    Windows,
    Linux,
}

impl Platform {
    /// The platform this build runs on. A page in a browser counts as Linux: the shortcut
    /// text follows the PC convention there.
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Linux
        }
    }

    /// The modifier the platform's command shortcuts use: Command on macOS, Control elsewhere.
    pub const fn primary(self) -> Mods {
        match self {
            Self::Mac => Mods::META,
            Self::Windows | Self::Linux => Mods::CTRL,
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::current()
    }
}
