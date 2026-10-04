//! Font metrics, glyph rasterization, and a paged atlas. No GPU dependencies.
//!
//! A string reaches the screen in four steps: `Text` asks for a layout keyed by
//! (string, size, wrap width, font), cosmic-text shapes it with the face the
//! family resolves for the weight, painting turns each positioned glyph into a
//! cache key that also names face and weight, and the atlas rasterizes it once.

use std::{collections::HashMap, sync::Arc};

use ab_glyph::{point, Font, FontVec, Glyph};
use cosmic_text::{fontdb, CacheKey, LayoutGlyph, SwashCache, SwashContent};

use crate::{protocol::TextureImage, shapes::Mesh, Color, Id, Rect, TextureId, Vec2};

mod atlas;
mod family;
mod fonts;
mod layout;
mod lines;
mod weight;

pub use family::FontFamily;
pub use weight::FontWeight;

use atlas::AtlasPage;
use fonts::{font_system, Registered};
use layout::{CachedLayout, FontKey};
pub(crate) use layout::DEFAULT_TAB;
pub(crate) use lines::VisualLine;

const ATLAS_SIZE: u32 = 1024;

#[derive(Clone, Copy, Debug)]
struct CachedGlyph {
    texture: TextureId,
    uv: Rect,
    offset: Vec2,
    size: Vec2,
    colored: bool,
}

pub(crate) struct TextLayout {
    glyphs: Vec<(LayoutGlyph, f32)>,
    baseline: f32,
    carets: Vec<(usize, f32)>,
    /// Visual lines top to bottom; never empty for a shaped paragraph.
    pub(crate) lines: Vec<VisualLine>,
    pub(crate) size: Vec2,
    width_independent: bool,
}

pub(crate) struct TextSystem {
    family: Registered,
    /// Cap-height top and descender bottom per font size, measured on the regular
    /// face. One band for all weights keeps optical alignment from shifting with weight.
    bands: HashMap<u32, (f32, f32)>,
    /// The regular face outlines used for `bands`, parsed on first use.
    outlines: Option<Option<FontVec>>,
    swash: SwashCache,
    glyphs: HashMap<CacheKey, Option<CachedGlyph>>,
    pages: Vec<AtlasPage>,
    layouts: HashMap<Id, CachedLayout>,
    layout_keys: HashMap<Id, Id>,
    frame: u64,
    /// Layouts shaped from scratch since creation; cache hits do not count.
    pub(crate) builds: u64,
    ids: crate::images::TextureIds,
}

impl TextSystem {
    #[cfg(test)]
    pub fn new(family: FontFamily) -> Self {
        Self::with_allocator(family, crate::images::TextureIds::default())
    }

    pub(crate) fn with_allocator(family: FontFamily, ids: crate::images::TextureIds) -> Self {
        Self {
            family: fonts::register(family),
            bands: HashMap::new(),
            outlines: None,
            swash: SwashCache::new(),
            glyphs: HashMap::new(),
            pages: Vec::new(),
            layouts: HashMap::new(),
            layout_keys: HashMap::new(),
            frame: 0,
            builds: 0,
            ids,
        }
    }

    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    pub fn end_frame(&mut self) {
        // Keep layouts used by the live UI; changing strings or weights cannot
        // accumulate indefinitely. Measurement and painting share one immutable layout.
        self.layouts
            .retain(|_, cached| cached.last_frame == self.frame);
        self.layout_keys
            .retain(|_, key| self.layouts.contains_key(key));
    }

    /// The font a request is shaped with: the family file the weight resolves to.
    pub(crate) fn font_key(&self, weight: FontWeight) -> FontKey {
        FontKey::new(self.family.id, self.family.family.resolve(weight))
    }

    /// Optical alignment uses a stable cap/descender band, independent of line gap.
    /// The same baseline is retained as characters are typed or a placeholder changes.
    pub fn centered_line_offset(&mut self, text: &str, size: f32, weight: FontWeight) -> f32 {
        let layout = self.layout(text, size, weight, f32::INFINITY);
        let (top, bottom) = self.band(size);
        layout.size.y * 0.5 - layout.baseline - (top + bottom) * 0.5
    }

    fn band(&mut self, size: f32) -> (f32, f32) {
        if let Some(band) = self.bands.get(&size.to_bits()) {
            return *band;
        }
        let family = &self.family.family;
        let font = self.outlines.get_or_insert_with(|| {
            let regular = family.data(family.resolve(FontWeight::REGULAR))?;
            FontVec::try_from_vec(regular.to_vec()).ok()
        });
        let band = font
            .as_ref()
            .map_or((0.0, 0.0), |font| alignment_band(font, size));
        self.bands.insert(size.to_bits(), band);
        band
    }

    /// Integer physical motion normally preserves raster phases. Check the actual
    /// keys as well: floating-point addition near a subpixel bin boundary can
    /// otherwise change a glyph even when the requested delta is an integer.
    pub(crate) fn translation_preserves_raster(
        &mut self,
        text: &str,
        position: Vec2,
        size: f32,
        weight: FontWeight,
        wrap_width: f32,
        tab: u16,
        delta: Vec2,
        scale: f32,
    ) -> bool {
        let layout = self.layout_with_tab(text, size, weight, wrap_width, tab);
        let moved = position + delta;
        layout.glyphs.iter().all(|(glyph, baseline)| {
            let old = glyph.physical((position.x * scale, (position.y + baseline) * scale), scale);
            let new = glyph.physical((moved.x * scale, (moved.y + baseline) * scale), scale);
            old.cache_key == new.cache_key
        })
    }

    pub fn paint(
        &mut self,
        mesh: &mut Mesh,
        text: &str,
        position: Vec2,
        size: f32,
        weight: FontWeight,
        wrap_width: f32,
        color: Color,
        scale: f32,
    ) {
        self.paint_with_tab(mesh, text, position, size, weight, wrap_width, DEFAULT_TAB, color, scale);
    }

    pub fn paint_with_tab(
        &mut self,
        mesh: &mut Mesh,
        text: &str,
        position: Vec2,
        size: f32,
        weight: FontWeight,
        wrap_width: f32,
        tab: u16,
        color: Color,
        scale: f32,
    ) {
        let layout = self.layout_with_tab(text, size, weight, wrap_width, tab);
        for (glyph, baseline) in &layout.glyphs {
            // Subpixel positioning is part of the raster cache key, including DPI.
            let physical =
                glyph.physical((position.x * scale, (position.y + baseline) * scale), scale);
            if let Some(cached) = self.glyph(physical.cache_key) {
                let min =
                    Vec2::new(physical.x as f32, physical.y as f32) / scale + cached.offset / scale;
                let tint = if cached.colored {
                    Color::rgba(255, 255, 255, color.0[3])
                } else {
                    color
                };
                mesh.quad(
                    Rect::from_min_size(min, cached.size / scale),
                    cached.uv,
                    tint.linear(),
                    cached.texture,
                );
            }
        }
    }
}

/// Pixel bounds of the outlines of `H` and `g` at `size`.
fn alignment_band(font: &FontVec, size: f32) -> (f32, f32) {
    let em_scale =
        size * font.height_unscaled() / font.units_per_em().unwrap_or(font.height_unscaled());
    let (mut top, mut bottom) = (0.0_f32, 0.0_f32);
    for ch in ['H', 'g'] {
        if let Some(outline) = font.outline_glyph(Glyph {
            id: font.glyph_id(ch),
            scale: em_scale.into(),
            position: point(0.0, 0.0),
        }) {
            let bounds = outline.px_bounds();
            top = top.min(bounds.min.y);
            bottom = bottom.max(bounds.max.y);
        }
    }
    (top, bottom)
}

#[cfg(test)]
#[path = "../tests/text/layout.rs"]
mod tests;
#[cfg(test)]
#[path = "../tests/text/weights.rs"]
mod weight_tests;
