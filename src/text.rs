//! Font metrics, glyph rasterization, and a paged atlas. No GPU dependencies.
//!
//! A string reaches the screen in four steps: `Text` asks for a layout keyed by
//! (string, size, wrap width, font), cosmic-text shapes it with the face the
//! family resolves for the weight, painting turns each positioned glyph into a
//! cache key that also names face and weight, and the atlas rasterizes it once.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ab_glyph::{point, Font, FontVec, Glyph};
use cosmic_text::{fontdb, CacheKey, LayoutGlyph, SwashCache, SwashContent};

use crate::{protocol::TextureImage, shapes::Mesh, Color, Id, Rect, TextureId, Vec2};

mod atlas;
#[doc(hidden)]
pub mod family;
#[doc(hidden)]
pub mod fonts;
#[doc(hidden)]
pub mod layout;
mod lines;
mod rich;
mod shape;
#[doc(hidden)]
pub mod variant;
#[doc(hidden)]
pub mod weight;

pub use family::FontFamily;
pub use rich::StyleRun;
pub(crate) use rich::same_shape;
pub use variant::{MonospaceMetrics, TextFamily};
pub use weight::FontWeight;

use atlas::AtlasPage;
use fonts::{font_system, Registered};
pub use layout::DEFAULT_TAB;
use layout::{CachedLayout, FontKey};
pub(crate) use lines::VisualLine;
pub use variant::TextFont;

const ATLAS_SIZE: u32 = 1024;

#[derive(Clone, Copy, Debug)]
struct CachedGlyph {
    texture: TextureId,
    uv: Rect,
    offset: Vec2,
    size: Vec2,
    colored: bool,
}

pub struct TextLayout {
    pub glyphs: Vec<(LayoutGlyph, f32)>,
    pub baseline: f32,
    pub carets: Vec<(usize, f32)>,
    /// Visual lines top to bottom; never empty for a shaped paragraph.
    pub lines: Vec<VisualLine>,
    pub size: Vec2,
    pub width_independent: bool,
}

/// Glyph rasterization state shared by every `TextSystem` of one set of shared resources:
/// the glyph cache, atlas pages, rasterizer and per-size alignment bands. Atlas pages are
/// append-only, so a window that built geometry earlier still finds its glyphs.
pub(crate) struct GlyphStore {
    /// Cap-height top and descender bottom per font size and family, measured on the
    /// regular face. One band for all weights keeps optical alignment from shifting with weight.
    bands: HashMap<(u32, bool), (f32, f32)>,
    /// The regular face outlines used for `bands` (proportional, monospace), parsed on first use.
    outlines: [Option<Option<FontVec>>; 2],
    swash: SwashCache,
    glyphs: HashMap<CacheKey, Option<CachedGlyph>>,
    pages: Vec<AtlasPage>,
    ids: crate::images::TextureIds,
}

impl GlyphStore {
    pub(crate) fn new(ids: crate::images::TextureIds) -> Self {
        Self {
            bands: HashMap::new(),
            outlines: [None, None],
            swash: SwashCache::new(),
            glyphs: HashMap::new(),
            pages: Vec::new(),
            ids,
        }
    }
}

pub struct TextSystem {
    family: Registered,
    /// The registered monospace family; `None` shapes with the system's generic monospace font.
    mono: Option<Registered>,
    store: Arc<Mutex<GlyphStore>>,
    pub layouts: HashMap<Id, CachedLayout>,
    layout_keys: HashMap<Id, Id>,
    rich: HashMap<Id, rich::RichCached>,
    frame: u64,
    /// Layouts shaped from scratch since creation; cache hits do not count.
    pub builds: u64,
}

impl TextSystem {
    pub fn new(family: FontFamily) -> Self {
        let store = GlyphStore::new(crate::images::TextureIds::default());
        Self::with_store(
            family,
            FontFamily::default_monospace(),
            Arc::new(Mutex::new(store)),
        )
    }

    pub(crate) fn with_store(
        family: FontFamily,
        mono: Option<FontFamily>,
        store: Arc<Mutex<GlyphStore>>,
    ) -> Self {
        Self {
            family: fonts::register(family),
            mono: mono.map(fonts::register),
            store,
            layouts: HashMap::new(),
            layout_keys: HashMap::new(),
            rich: HashMap::new(),
            frame: 0,
            builds: 0,
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
        self.rich
            .retain(|_, cached| cached.last_frame == self.frame);
    }

    /// The registered family a request is shaped with, if it is not the system monospace.
    fn registered(&self, font: TextFont) -> Option<&Registered> {
        if font.monospace() {
            self.mono.as_ref()
        } else {
            Some(&self.family)
        }
    }

    /// The font a request is shaped with: the family file the weight resolves to.
    pub fn font_key(&self, font: impl Into<TextFont>) -> FontKey {
        let font = font.into();
        let (id, weight) = self
            .registered(font)
            .map_or((Id::new("system-monospace"), font.weight), |r| {
                (r.id, r.family.resolve(font.weight))
            });
        FontKey::new(id, weight, font.monospace(), font.tabular)
    }

    /// Optical alignment uses a stable cap/descender band, independent of line gap.
    /// The same baseline is retained as characters are typed or a placeholder changes.
    pub fn centered_line_offset(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
    ) -> f32 {
        let font = font.into();
        let layout = self.layout(text, size, font, f32::INFINITY);
        let (top, bottom) = self.band(size, font.monospace());
        layout.size.y * 0.5 - layout.baseline - (top + bottom) * 0.5
    }

    /// The alignment band of the family. A system monospace font has no outlines to
    /// measure here and borrows the proportional band.
    fn band(&mut self, size: f32, monospace: bool) -> (f32, f32) {
        let monospace = monospace && self.mono.is_some();
        let mut store = self.store.lock().expect("glyph store mutex");
        if let Some(band) = store.bands.get(&(size.to_bits(), monospace)) {
            return *band;
        }
        let family = if monospace {
            self.mono.as_ref().map(|r| &r.family)
        } else {
            Some(&self.family.family)
        };
        let outlines = store.outlines[usize::from(monospace)].get_or_insert_with(|| {
            let family = family?;
            let regular = family.data(family.resolve(FontWeight::REGULAR))?;
            FontVec::try_from_vec(regular.to_vec()).ok()
        });
        let band = outlines
            .as_ref()
            .map_or((0.0, 0.0), |font| alignment_band(font, size));
        store.bands.insert((size.to_bits(), monospace), band);
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
        font: impl Into<TextFont>,
        wrap_width: f32,
        tab: u16,
        delta: Vec2,
        scale: f32,
    ) -> bool {
        let layout = self.layout_with_tab(text, size, font, wrap_width, tab);
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
        font: impl Into<TextFont>,
        wrap_width: f32,
        color: Color,
        scale: f32,
    ) {
        self.paint_with_tab(
            mesh,
            text,
            position,
            size,
            font,
            wrap_width,
            DEFAULT_TAB,
            color,
            scale,
        );
    }

    pub fn paint_with_tab(
        &mut self,
        mesh: &mut Mesh,
        text: &str,
        position: Vec2,
        size: f32,
        font: impl Into<TextFont>,
        wrap_width: f32,
        tab: u16,
        color: Color,
        scale: f32,
    ) {
        let layout = self.layout_with_tab(text, size, font, wrap_width, tab);
        let mut store = self.store.lock().expect("glyph store mutex");
        for (glyph, baseline) in &layout.glyphs {
            // Subpixel positioning is part of the raster cache key, including DPI.
            let physical =
                glyph.physical((position.x * scale, (position.y + baseline) * scale), scale);
            if let Some(cached) = store.glyph(physical.cache_key) {
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
