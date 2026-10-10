//! Framework-independent Lucide icons as static SVG data.
//!
//! Every icon is a `static` [`Icon`] named after its Lucide file in `SCREAMING_SNAKE_CASE`
//! (`user` → [`USER`], `arrow-left` → [`ARROW_LEFT`], `grid-2x2` → [`GRID_2X2`]).
//! No GUI toolkit, allocator, filesystem, parser or renderer is required.
//!
//! ```
//! let bytes: &'static [u8] = z_icons::USER.svg();
//! let text: &'static str = z_icons::USER.svg_str();
//! // Pass bytes/text to your toolkit's SVG loader, or export a colored document.
//! let svg = z_icons::USER.render().size(32, 32).color("#2563eb").to_string();
//! assert!(svg.contains("#2563eb"));
//! ```
//!
//! Raw SVGs have white strokes for image tinting. [`Icon::render`] defaults to
//! `currentColor` for inline SVG and can set an explicit color for SVG loaders.
//! [`Svg`] implements [`core::fmt::Display`], including for allocation-free writers.
//! Unused static icons can be discarded by the linker. Optional feature `catalog`
//! enables `all()` and `get()`, retaining the complete set when used.
//!
//! Rust code is MIT licensed; Lucide assets are ISC/MIT (see `LICENSE-LUCIDE`).

#![no_std]
#![forbid(unsafe_code)]

mod svg;
pub use svg::Svg;

#[cfg(feature = "catalog")]
mod catalog;
#[cfg(feature = "catalog")]
pub use catalog::{all, get};

/// A named, static UTF-8 SVG document. Bundled Lucide icons use a 24×24 viewBox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon {
    name: &'static str,
    svg: &'static str,
    bundled: bool,
}

impl Icon {
    /// Embed a custom static SVG, for example with `include_str!("icon.svg")`.
    ///
    /// The document is trusted application data, not sanitized. To use [`Self::render`],
    /// it must start with an `<svg ...>` root (leading whitespace is allowed), without
    /// an XML declaration or doctype. Root options follow normal SVG inheritance;
    /// explicit child strokes/styles take precedence.
    pub const fn new(name: &'static str, svg: &'static str) -> Self {
        Self {
            name,
            svg,
            bundled: false,
        }
    }

    pub(crate) const fn bundled(name: &'static str, svg: &'static str) -> Self {
        Self {
            name,
            svg,
            bundled: true,
        }
    }

    /// Icon name, e.g. `"arrow-left"`.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Original SVG bytes, without parsing or allocation. Bundled strokes are white.
    pub const fn svg(&self) -> &'static [u8] {
        self.svg.as_bytes()
    }

    /// Original SVG text, without parsing or allocation.
    pub const fn svg_str(&self) -> &'static str {
        self.svg
    }

    /// A configurable SVG document, using `currentColor` by default.
    ///
    /// Use `.color("#fff")` for tintable images, or an explicit color for rasterization.
    pub const fn render(&self) -> Svg<'_> {
        Svg::new(self)
    }
}

#[rustfmt::skip]
mod generated;
pub use generated::*;
