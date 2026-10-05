//! How a run of text is set beyond size and weight: the family it is shaped with
//! and whether its digits are tabular.

use super::weight::FontWeight;

/// The family a piece of text is shaped with. The two families are chosen
/// independently of each other and of the weight.
///
/// `Proportional` is the application's main font ([`Context::with_fonts`](crate::Context::with_fonts)).
/// `Monospace` is the code font: bundled JetBrains Mono with the default
/// `bundled-monospace` feature, or the family given to
/// [`SharedResources::with_font_families`](crate::SharedResources::with_font_families).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextFamily {
    #[default]
    Proportional,
    Monospace,
}

/// Cell size of the monospace family at one size and weight, from the same layout
/// that paints and measures monospace text.
///
/// `cell_width` is the advance of every character of the primary font, so `n` of them are
/// `n * cell_width` wide and a tab stop is `tab_size * cell_width` apart. A glyph that the
/// primary font lacks (CJK, emoji, symbols) comes from a fallback font and keeps that font's
/// own advance, which is generally not a whole number of cells; the width of a string is
/// always its real shaped width. `line_height` is the height of one line, also the row
/// step of a code view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonospaceMetrics {
    pub cell_width: f32,
    pub line_height: f32,
}

/// Weight, family and figure style: everything about a font that a layout depends on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextFont {
    pub weight: FontWeight,
    pub family: TextFamily,
    /// OpenType `tnum`. Always false for the monospace family, whose digits are already
    /// one cell wide, so the cache never holds two identical layouts.
    pub tabular: bool,
}

impl TextFont {
    pub fn new(weight: FontWeight, family: TextFamily, tabular: bool) -> Self {
        Self {
            weight,
            family,
            tabular: tabular && family == TextFamily::Proportional,
        }
    }

    pub(crate) fn monospace(self) -> bool {
        self.family == TextFamily::Monospace
    }

    /// The default family without figure features: what plain text always uses.
    pub(crate) fn is_plain(self) -> bool {
        self.family == TextFamily::Proportional && !self.tabular
    }
}

impl From<FontWeight> for TextFont {
    fn from(weight: FontWeight) -> Self {
        Self::new(weight, TextFamily::Proportional, false)
    }
}
