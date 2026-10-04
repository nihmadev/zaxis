//! Lucide icons as static SVG data, bundled for offline desktop rendering.
//!
//! Every icon is a `static` [`Icon`] named after its Lucide file in `SCREAMING_SNAKE_CASE`
//! (`user` → [`USER`], `arrow-left` → [`ARROW_LEFT`], `grid-2x2` → [`GRID_2X2`]).
//! Unused icons are removed by the linker, so only the icons you reference reach the binary.
//!
//! Strokes are stored white instead of `currentColor`; color an icon by tinting the image
//! that displays it. This crate only exposes data and performs no parsing or rasterization.
//! Icons are distributed under the ISC license; a few derived from Feather are MIT
//! (see `LICENSE-LUCIDE`).

#![no_std]
#![forbid(unsafe_code)]

/// One bundled icon: its Lucide name and 24×24 SVG source (UTF-8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon {
    name: &'static str,
    svg: &'static [u8],
}

impl Icon {
    pub(crate) const fn new(name: &'static str, svg: &'static [u8]) -> Self {
        Self { name, svg }
    }

    /// Lucide name, e.g. `"arrow-left"`.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// SVG document bytes with white strokes.
    pub const fn svg(&self) -> &'static [u8] {
        self.svg
    }
}

#[rustfmt::skip]
mod generated;
pub use generated::*;
