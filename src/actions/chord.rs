use super::{names, Mods, Stroke};
use crate::KeyBinding;
use std::{fmt, str::FromStr};
use winit::keyboard::KeyCode;

/// The most strokes one chord holds.
pub const MAX_STROKES: usize = 4;

/// A shortcut: one stroke, or a sequence such as `Ctrl+K` then `Ctrl+C`.
///
/// Written and read as text with `Display` and `FromStr`: strokes separated by a space,
/// modifiers and key by `+`, names in lowercase (`ctrl+k ctrl+c`, `mod+shift+s`, `f5`).
/// The text round-trips: `chord.to_string().parse() == Ok(chord)`.
///
/// ```
/// use zaxis::{Chord, Mods};
/// use zaxis::winit::keyboard::KeyCode;
/// let save: Chord = Mods::PRIMARY.key(KeyCode::KeyS).into();
/// assert_eq!(save.to_string(), "mod+s");
/// assert_eq!("ctrl+k ctrl+c".parse::<Chord>().unwrap().len(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Chord(Vec<Stroke>);

impl Chord {
    /// A chord of these strokes. Invalid strokes, and strokes beyond [`MAX_STROKES`], are
    /// dropped; an empty result is an empty chord that matches nothing.
    pub fn new(strokes: impl IntoIterator<Item = Stroke>) -> Self {
        Self(
            strokes
                .into_iter()
                .filter(Stroke::is_valid)
                .take(MAX_STROKES)
                .collect(),
        )
    }

    pub fn strokes(&self) -> &[Stroke] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// This chord followed by `stroke`.
    pub fn then(mut self, stroke: Stroke) -> Self {
        if stroke.is_valid() && self.0.len() < MAX_STROKES {
            self.0.push(stroke);
        }
        self
    }
}

impl From<Stroke> for Chord {
    fn from(stroke: Stroke) -> Self {
        Self::new([stroke])
    }
}

impl From<KeyCode> for Chord {
    fn from(code: KeyCode) -> Self {
        Stroke::from(code).into()
    }
}

impl From<KeyBinding> for Chord {
    fn from(key: KeyBinding) -> Self {
        Stroke::from(key).into()
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, stroke) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(" ")?;
            }
            write!(f, "{stroke}")?;
        }
        Ok(())
    }
}

/// Why a chord text could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChordError {
    Empty,
    /// A word before the last `+` that is not a modifier.
    UnknownModifier(String),
    /// The last word is not a key name.
    UnknownKey(String),
    /// More than [`MAX_STROKES`] strokes.
    TooLong,
}

impl fmt::Display for ChordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty shortcut"),
            Self::UnknownModifier(word) => write!(f, "unknown modifier `{word}`"),
            Self::UnknownKey(word) => write!(f, "unknown key `{word}`"),
            Self::TooLong => write!(f, "more than {MAX_STROKES} strokes"),
        }
    }
}

impl std::error::Error for ChordError {}

impl FromStr for Stroke {
    type Err = ChordError;

    fn from_str(text: &str) -> Result<Self, ChordError> {
        let lower = text.trim().to_ascii_lowercase();
        let mut words: Vec<&str> = lower.split('+').map(str::trim).collect();
        let key = words.pop().filter(|word| !word.is_empty());
        let key = key.ok_or(ChordError::Empty)?;
        let mut mods = Mods::NONE;
        for word in words {
            let flag =
                Mods::from_name(word).ok_or_else(|| ChordError::UnknownModifier(word.into()))?;
            mods = mods | flag;
        }
        let code = names::code(key).ok_or_else(|| ChordError::UnknownKey(key.into()))?;
        Ok(Stroke::new(mods, code))
    }
}

impl FromStr for Chord {
    type Err = ChordError;

    fn from_str(text: &str) -> Result<Self, ChordError> {
        let strokes = text
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<Vec<Stroke>, _>>()?;
        match strokes.len() {
            0 => Err(ChordError::Empty),
            n if n > MAX_STROKES => Err(ChordError::TooLong),
            _ => Ok(Self(strokes)),
        }
    }
}
