//! Font weight and the nearest-face rule shared by registration and layout.

use cosmic_text::fontdb;

/// A font weight on the CSS scale: 100 (thin) to 900 (black), 400 is regular.
///
/// Values outside the range are clamped. The weight selects a font file of the
/// family; it never thickens outlines. See [`FontFamily::resolve`](crate::FontFamily::resolve)
/// for the match used when the family has no file at exactly this weight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontWeight(u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const REGULAR: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMIBOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);

    /// A numeric weight, clamped to 100–900.
    pub const fn new(value: u16) -> Self {
        Self(if value < 100 {
            100
        } else if value > 900 {
            900
        } else {
            value
        })
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    pub(crate) fn to_fontdb(self) -> fontdb::Weight {
        fontdb::Weight(self.0)
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::REGULAR
    }
}

impl From<u16> for FontWeight {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

/// CSS font matching restricted to weight: an exact face wins. Below 400 the
/// next lighter face is preferred, then heavier ones. Above 500 the next heavier
/// face is preferred, then lighter ones. For 400–500 the search first goes up to
/// 500, then down, then above 500. `None` only when `available` is empty.
pub(super) fn nearest(
    available: impl Iterator<Item = FontWeight> + Clone,
    wanted: FontWeight,
) -> Option<FontWeight> {
    let up = |from: u16, to: u16| {
        available
            .clone()
            .filter(|w| (from..=to).contains(&w.0))
            .min()
    };
    let down = |below: u16| available.clone().filter(|w| w.0 < below).max();
    let w = wanted.0;
    if available.clone().any(|w| w == wanted) {
        return Some(wanted);
    }
    if (400..=500).contains(&w) {
        up(w + 1, 500).or_else(|| down(w)).or_else(|| up(501, 900))
    } else if w < 400 {
        down(w).or_else(|| up(w + 1, 900))
    } else {
        up(w + 1, 900).or_else(|| down(w))
    }
}
