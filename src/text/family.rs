//! A font family described as one file per weight.

use std::{fmt, sync::Arc};

use super::weight::{nearest, FontWeight};
use crate::Id;

type FontData = Arc<dyn AsRef<[u8]> + Send + Sync>;

/// A font family: the font files of one typeface, each declared for a weight.
///
/// Every weight is a separate file, so bold is drawn from real bold outlines
/// and metrics. A weight without a file is served by the nearest declared one.
/// Pass the family to [`RunOptions::with_font_family`](crate::RunOptions::with_font_family)
/// or [`Context::with_fonts`](crate::Context::with_fonts).
///
/// ```
/// use zaxis::{FontFamily, FontWeight};
/// # let regular: &'static [u8] = &[];
/// # let bold: &'static [u8] = &[];
/// let family = FontFamily::new(regular).with_weight(FontWeight::BOLD, bold);
/// assert_eq!(family.resolve(FontWeight::SEMIBOLD), FontWeight::BOLD);
/// ```
#[derive(Clone)]
pub struct FontFamily {
    faces: Vec<(FontWeight, FontData)>,
}

impl FontFamily {
    /// A family whose only file is the regular (400) face. `include_bytes!`
    /// data stays in the executable image; `Vec<u8>` is kept as given.
    pub fn new(regular: impl AsRef<[u8]> + Send + Sync + 'static) -> Self {
        Self {
            faces: vec![(FontWeight::REGULAR, Arc::new(regular))],
        }
    }

    /// Declare the file for `weight`, replacing an earlier file of that weight.
    /// The declared weight is authoritative; the font's own weight metadata is ignored.
    pub fn with_weight(
        mut self,
        weight: FontWeight,
        data: impl AsRef<[u8]> + Send + Sync + 'static,
    ) -> Self {
        self.faces.retain(|(w, _)| *w != weight);
        self.faces.push((weight, Arc::new(data)));
        self
    }

    /// The bundled Inter family. Medium, SemiBold and Bold exist only with the
    /// `bundled-weights` feature; without it every weight resolves to Regular.
    pub fn inter() -> Self {
        let family = Self::new(include_bytes!("../../assets/Inter-Regular.ttf"));
        #[cfg(feature = "bundled-weights")]
        let family = family
            .with_weight(
                FontWeight::MEDIUM,
                include_bytes!("../../assets/Inter-Medium.ttf"),
            )
            .with_weight(
                FontWeight::SEMIBOLD,
                include_bytes!("../../assets/Inter-SemiBold.ttf"),
            )
            .with_weight(
                FontWeight::BOLD,
                include_bytes!("../../assets/Inter-Bold.ttf"),
            );
        family
    }

    /// Declared weights in ascending order.
    pub fn weights(&self) -> Vec<FontWeight> {
        let mut weights: Vec<_> = self.faces.iter().map(|(w, _)| *w).collect();
        weights.sort_unstable();
        weights
    }

    /// The weight whose file is used for a request. An exact file wins. Otherwise
    /// the CSS rule applies: below 400 lighter files are preferred, above 500
    /// heavier ones, and 400–500 look up to 500 first. Never synthesizes bold.
    pub fn resolve(&self, wanted: FontWeight) -> FontWeight {
        nearest(self.faces.iter().map(|(w, _)| *w), wanted).unwrap_or(FontWeight::REGULAR)
    }

    pub(super) fn data(&self, weight: FontWeight) -> Option<&[u8]> {
        self.faces
            .iter()
            .find(|(w, _)| *w == weight)
            .map(|(_, d)| (**d).as_ref())
    }

    pub(super) fn faces(&self) -> impl Iterator<Item = (FontWeight, &FontData)> {
        self.faces.iter().map(|(w, d)| (*w, d))
    }

    /// Identity of the font files, used to name the family and key caches.
    /// Samples the data instead of hashing megabytes on every `Context::new`.
    pub(super) fn identity(&self) -> Id {
        let mut parts: Vec<(u16, usize, Vec<u8>)> = self
            .faces
            .iter()
            .map(|(w, d)| {
                let bytes = (**d).as_ref();
                let step = (bytes.len() / 512).max(1);
                let sample = bytes.iter().step_by(step).copied().collect();
                (w.value(), bytes.len(), sample)
            })
            .collect();
        parts.sort();
        Id::new(parts)
    }
}

impl fmt::Debug for FontFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontFamily")
            .field("weights", &self.weights())
            .finish()
    }
}

impl Default for FontFamily {
    fn default() -> Self {
        Self::inter()
    }
}
