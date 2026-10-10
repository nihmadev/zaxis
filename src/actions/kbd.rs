use super::{names, Chord, Mods, Platform, Stroke};
use std::fmt;

/// Shortcut text for people: `⌘⇧S` on macOS, `Ctrl+Shift+S` elsewhere.
///
/// One formatter for menus, tooltips and any component that shows a shortcut, so the same
/// chord reads the same everywhere. [`Mods::PRIMARY`] becomes Command or Control for the
/// chosen platform. Strokes of a chord are separated by a space: `Ctrl+K Ctrl+C`.
///
/// ```
/// use zaxis::{Kbd, Mods, Platform};
/// use zaxis::winit::keyboard::KeyCode;
/// let save = Mods::PRIMARY.key(KeyCode::KeyS).into();
/// assert_eq!(Kbd::new(&save).platform(Platform::Linux).to_string(), "Ctrl+S");
/// assert_eq!(Kbd::new(&save).platform(Platform::Mac).to_string(), "⌘S");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Kbd<'a> {
    chord: &'a Chord,
    platform: Platform,
}

impl<'a> Kbd<'a> {
    pub fn new(chord: &'a Chord) -> Self {
        Self {
            chord,
            platform: Platform::current(),
        }
    }

    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    /// One stroke as text.
    pub fn stroke(stroke: &Stroke, platform: Platform) -> String {
        let mods = stroke.mods.concrete(platform);
        let key = stroke
            .code()
            .map_or_else(|| "?".to_owned(), |code| names::caption(code, platform));
        let table: [(Mods, &str, &str); 4] = match platform {
            Platform::Mac => [
                (Mods::CTRL, "⌃", ""),
                (Mods::ALT, "⌥", ""),
                (Mods::SHIFT, "⇧", ""),
                (Mods::META, "⌘", ""),
            ],
            Platform::Windows => [
                (Mods::CTRL, "Ctrl", "+"),
                (Mods::ALT, "Alt", "+"),
                (Mods::SHIFT, "Shift", "+"),
                (Mods::META, "Win", "+"),
            ],
            Platform::Linux => [
                (Mods::CTRL, "Ctrl", "+"),
                (Mods::ALT, "Alt", "+"),
                (Mods::SHIFT, "Shift", "+"),
                (Mods::META, "Super", "+"),
            ],
        };
        let mut text = String::new();
        for (flag, name, glue) in table {
            if mods.contains(flag) {
                text.push_str(name);
                text.push_str(glue);
            }
        }
        text + &key
    }
}

impl fmt::Display for Kbd<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, stroke) in self.chord.strokes().iter().enumerate() {
            if index > 0 {
                f.write_str(" ")?;
            }
            f.write_str(&Self::stroke(stroke, self.platform))?;
        }
        Ok(())
    }
}
