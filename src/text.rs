//! Font metrics, glyph rasterization, and a paged atlas. No GPU dependencies.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use ab_glyph::{point, Font, FontArc, Glyph};
use cosmic_text::{
    fontdb, Attrs, Buffer, CacheKey, Fallback, Family, FontSystem, LayoutGlyph, Metrics,
    PlatformFallback, Shaping, SwashCache, SwashContent, Wrap,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{protocol::TextureImage, shapes::Mesh, Color, Id, Rect, TextureId, Vec2};

const ATLAS_SIZE: u32 = 1024;

#[derive(Clone, Copy, Debug)]
struct CachedGlyph {
    texture: TextureId,
    uv: Rect,
    offset: Vec2,
    size: Vec2,
    colored: bool,
}

struct TextLayout {
    glyphs: Vec<(LayoutGlyph, f32)>,
    baseline: f32,
    carets: Vec<(usize, f32)>,
    size: Vec2,
}

struct CachedLayout {
    text: String,
    size: u32,
    wrap: u32,
    layout: Arc<TextLayout>,
    last_frame: u64,
}

struct AtlasPage {
    image: TextureImage,
    x: u32,
    y: u32,
    row_height: u32,
}

impl AtlasPage {
    fn new(id: TextureId, side: u32) -> Self {
        Self {
            image: TextureImage {
                id,
                size: [side; 2],
                pixels: Arc::new([255, 255, 255, 0].repeat((side * side) as usize)),
                revision: 0,
            },
            x: 1,
            y: 1,
            row_height: 0,
        }
    }

    fn allocate(&mut self, width: u32, height: u32) -> Option<[u32; 2]> {
        let side = self.image.size[0];
        if width + 2 > side || height + 2 > side {
            return None;
        }
        if self.x + width + 1 > side {
            self.x = 1;
            self.y += self.row_height + 2;
            self.row_height = 0;
        }
        if self.y + height + 1 > side {
            return None;
        }
        let origin = [self.x, self.y];
        self.x += width + 2;
        self.row_height = self.row_height.max(height);
        Some(origin)
    }
}

struct EmojiFallback(Vec<&'static str>);

impl Fallback for EmojiFallback {
    fn common_fallback(&self) -> &[&'static str] {
        &self.0
    }
    fn forbidden_fallback(&self) -> &[&'static str] {
        PlatformFallback.forbidden_fallback()
    }
    fn script_fallback(&self, script: unicode_script::Script, locale: &str) -> &[&'static str] {
        PlatformFallback.script_fallback(script, locale)
    }
}

fn font_system() -> &'static Mutex<FontSystem> {
    static FONTS: OnceLock<Mutex<FontSystem>> = OnceLock::new();
    FONTS.get_or_init(|| {
        #[cfg(feature = "bundled-emoji")]
        let bundled = [fontdb::Source::Binary(Arc::new(
            zaxis_emoji::FONT_DATA.to_vec(),
        ))];
        #[cfg(not(feature = "bundled-emoji"))]
        let bundled = [];
        let system = FontSystem::new_with_fonts(bundled);
        let (locale, db) = system.into_locale_and_db();
        let common = std::iter::once("Noto Color Emoji")
            .chain(PlatformFallback.common_fallback().iter().copied())
            .collect();
        Mutex::new(FontSystem::new_with_locale_and_db_and_fallback(
            locale,
            db,
            EmojiFallback(common),
        ))
    })
}

pub(crate) struct TextSystem {
    font: FontArc,
    family: String,
    swash: SwashCache,
    glyphs: HashMap<CacheKey, Option<CachedGlyph>>,
    pages: Vec<AtlasPage>,
    layouts: HashMap<Id, CachedLayout>,
    frame: u64,
    ids: crate::images::TextureIds,
}

impl TextSystem {
    #[cfg(test)]
    pub fn new(font: FontArc) -> Self {
        Self::with_allocator(font, crate::images::TextureIds::default())
    }

    pub(crate) fn with_allocator(font: FontArc, ids: crate::images::TextureIds) -> Self {
        // Keep custom fonts distinct from installed fonts with the same family name.
        // The system font database and fallback caches are initialized once per process.
        let family = format!("zaxis-{:?}", Id::new(font.font_data()));
        let mut fonts = font_system().lock().unwrap();
        if !fonts
            .db()
            .faces()
            .any(|face| face.families.iter().any(|(name, _)| name == &family))
        {
            let mut db = fontdb::Database::new();
            db.load_font_data(font.font_data().to_vec());
            for mut face in db.faces().cloned() {
                face.families = vec![(family.clone(), face.families[0].1)];
                fonts.db_mut().push_face_info(face);
            }
        }
        Self {
            font,
            family,
            swash: SwashCache::new(),
            glyphs: HashMap::new(),
            pages: Vec::new(),
            layouts: HashMap::new(),
            frame: 0,
            ids,
        }
    }

    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    pub fn end_frame(&mut self) {
        // Keep layouts used by the live UI; changing strings cannot accumulate
        // indefinitely. Measurement and painting share the same immutable layout.
        self.layouts
            .retain(|_, cached| cached.last_frame == self.frame);
    }

    pub fn measure(&mut self, text: &str, size: f32, wrap_width: f32) -> Vec2 {
        self.layout(text, size, wrap_width).size
    }

    /// Insertion positions from the same kerning and fallback metrics used for painting.
    pub fn carets(&mut self, text: &str, size: f32) -> Vec<(usize, f32)> {
        self.layout(text, size, f32::INFINITY).carets.clone()
    }

    fn layout(&mut self, text: &str, size: f32, wrap_width: f32) -> Arc<TextLayout> {
        let key = Id::new((text, size.to_bits(), wrap_width.to_bits()));
        if let Some(cached) = self.layouts.get_mut(&key) {
            // Verify the key contents as well: a hash collision must never reuse
            // another string's glyph positions.
            if cached.text == text
                && cached.size == size.to_bits()
                && cached.wrap == wrap_width.to_bits()
            {
                cached.last_frame = self.frame;
                return Arc::clone(&cached.layout);
            }
        }
        let layout = Arc::new(self.build_layout(text, size, wrap_width));
        self.layouts.insert(
            key,
            CachedLayout {
                text: text.to_owned(),
                size: size.to_bits(),
                wrap: wrap_width.to_bits(),
                layout: Arc::clone(&layout),
                last_frame: self.frame,
            },
        );
        layout
    }

    fn build_layout(&self, text: &str, size: f32, wrap_width: f32) -> TextLayout {
        let line_height = size * 1.25;
        let mut fonts = font_system().lock().unwrap();
        let mut buffer = Buffer::new(&mut fonts, Metrics::new(size, line_height));
        buffer.set_wrap(
            &mut fonts,
            if wrap_width.is_finite() {
                Wrap::Glyph
            } else {
                Wrap::None
            },
        );
        buffer.set_size(
            &mut fonts,
            wrap_width.is_finite().then_some(wrap_width.max(0.0)),
            None,
        );
        buffer.set_text(
            &mut fonts,
            text,
            &Attrs::new().family(Family::Name(&self.family)),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut fonts, false);
        let mut glyphs = Vec::new();
        let mut carets = Vec::new();
        let mut width = 0.0_f32;
        let mut height = line_height;
        let mut baseline = 0.0;
        let mut line_start = 0;
        let mut previous_line = 0;
        for run in buffer.layout_runs() {
            if glyphs.is_empty() {
                baseline = run.line_y;
            }
            while previous_line < run.line_i {
                line_start += buffer.lines[previous_line].text().len()
                    + buffer.lines[previous_line].ending().as_str().len();
                previous_line += 1;
            }
            width = width.max(run.line_w);
            height = height.max(run.line_top + run.line_height);
            // A shaped cluster may contain a ligature or several glyphs. Give each
            // grapheme boundary a caret, but never an endpoint inside a grapheme.
            for (byte, _) in run
                .text
                .grapheme_indices(true)
                .chain([(run.text.len(), "")])
            {
                let matching: Vec<_> = run
                    .glyphs
                    .iter()
                    .filter(|g| {
                        g.start <= byte && (byte < g.end || byte == run.text.len() && byte == g.end)
                    })
                    .collect();
                let x = matching
                    .first()
                    .map_or(if run.rtl { 0.0 } else { run.line_w }, |glyph| {
                        let left = matching.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);
                        let right = matching
                            .iter()
                            .map(|g| g.x + g.w)
                            .fold(f32::NEG_INFINITY, f32::max);
                        let cluster = &run.text[glyph.start..glyph.end];
                        let count = cluster.graphemes(true).count().max(1);
                        let before = run.text[glyph.start..byte].graphemes(true).count();
                        let fraction = before as f32 / count as f32;
                        if glyph.level.is_rtl() {
                            right - (right - left) * fraction
                        } else {
                            left + (right - left) * fraction
                        }
                    });
                carets.push((line_start + byte, x));
            }
            glyphs.extend(run.glyphs.iter().cloned().map(|g| (g, run.line_y)));
        }
        carets.sort_by_key(|p| p.0);
        carets.dedup_by_key(|p| p.0);
        TextLayout {
            glyphs,
            baseline,
            carets,
            size: Vec2::new(width, height),
        }
    }

    /// Optical alignment uses a stable cap/descender band, independent of line gap.
    /// The same baseline is retained as characters are typed or a placeholder changes.
    pub fn centered_line_offset(&mut self, text: &str, size: f32) -> f32 {
        let layout = self.layout(text, size, f32::INFINITY);
        let em_scale = size * self.font.height_unscaled()
            / self
                .font
                .units_per_em()
                .unwrap_or(self.font.height_unscaled());
        let mut top = 0.0_f32;
        let mut bottom = 0.0_f32;
        for ch in ['H', 'g'] {
            if let Some(outline) = self.font.outline_glyph(Glyph {
                id: self.font.glyph_id(ch),
                scale: em_scale.into(),
                position: point(0.0, 0.0),
            }) {
                let bounds = outline.px_bounds();
                top = top.min(bounds.min.y);
                bottom = bottom.max(bounds.max.y);
            }
        }
        layout.size.y * 0.5 - layout.baseline - (top + bottom) * 0.5
    }

    /// Integer physical motion normally preserves raster phases. Check the actual
    /// keys as well: floating-point addition near a subpixel bin boundary can
    /// otherwise change a glyph even when the requested delta is an integer.
    pub(crate) fn translation_preserves_raster(
        &mut self,
        text: &str,
        position: Vec2,
        size: f32,
        wrap_width: f32,
        delta: Vec2,
        scale: f32,
    ) -> bool {
        let layout = self.layout(text, size, wrap_width);
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
        wrap_width: f32,
        color: Color,
        scale: f32,
    ) {
        let layout = self.layout(text, size, wrap_width);
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

    fn glyph(&mut self, key: CacheKey) -> Option<CachedGlyph> {
        if let Some(cached) = self.glyphs.get(&key) {
            return *cached;
        }
        let mut fonts = font_system().lock().unwrap();
        let Some(image) = self.swash.get_image_uncached(&mut fonts, key) else {
            self.glyphs.insert(key, None);
            return None;
        };
        drop(fonts);
        let width = image.placement.width;
        let height = image.placement.height;
        if width == 0 || height == 0 {
            self.glyphs.insert(key, None);
            return None;
        }
        let mut allocation = self
            .pages
            .iter_mut()
            .enumerate()
            .find_map(|(i, page)| page.allocate(width, height).map(|origin| (i, origin)));
        if allocation.is_none() {
            let side = ATLAS_SIZE.max((width.max(height) + 2).next_power_of_two());
            let mut page = AtlasPage::new(self.ids.next(), side);
            let origin = page
                .allocate(width, height)
                .expect("a new atlas page fits the glyph");
            allocation = Some((self.pages.len(), origin));
            self.pages.push(page);
        }
        let (page_index, [x, y]) = allocation.unwrap();
        let page = &mut self.pages[page_index];
        let side = page.image.size[0];
        let pixels = Arc::make_mut(&mut page.image.pixels);
        for gy in 0..height {
            for gx in 0..width {
                let source = (gy * width + gx) as usize;
                let rgba = match image.content {
                    SwashContent::Mask => [255, 255, 255, image.data[source]],
                    SwashContent::Color => {
                        let p = &image.data[source * 4..source * 4 + 4];
                        // Swash color images are premultiplied; atlas textures are straight RGBA.
                        let alpha = u32::from(p[3]);
                        let unpremultiply = |c: u8| {
                            (u32::from(c) * 255)
                                .checked_div(alpha)
                                .unwrap_or(0)
                                .min(255) as u8
                        };
                        [
                            unpremultiply(p[0]),
                            unpremultiply(p[1]),
                            unpremultiply(p[2]),
                            p[3],
                        ]
                    }
                    SwashContent::SubpixelMask => {
                        let p = &image.data[source * 4..source * 4 + 4];
                        [255, 255, 255, p[0].max(p[1]).max(p[2])]
                    }
                };
                let index = (((y + gy) * side + x + gx) * 4) as usize;
                pixels[index..index + 4].copy_from_slice(&rgba);
            }
        }
        page.image.revision += 1;
        let glyph = CachedGlyph {
            texture: page.image.id,
            uv: Rect::from_min_size(
                Vec2::new(x as f32, y as f32) / side as f32,
                Vec2::new(width as f32, height as f32) / side as f32,
            ),
            offset: Vec2::new(image.placement.left as f32, -image.placement.top as f32),
            colored: image.content == SwashContent::Color,
            size: Vec2::new(width as f32, height as f32),
        };
        self.glyphs.insert(key, Some(glyph));
        Some(glyph)
    }

    pub fn textures(&self) -> Vec<TextureImage> {
        self.pages.iter().map(|page| page.image.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text() -> TextSystem {
        TextSystem::new(
            FontArc::try_from_slice(include_bytes!("../assets/Lato-Regular.ttf")).unwrap(),
        )
    }

    #[test]
    fn measured_layout_is_shared_by_paint_and_live_frames_only() {
        let mut text = text();
        text.begin_frame();
        let caption = "AV kerning\nUnicode λ and wrapped words";
        let size = text.measure(caption, 16.0, 90.0);
        let layout = text.layout(caption, 16.0, 90.0);
        assert_eq!(layout.size, size);
        assert!(Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 90.0)));
        let color = Color::rgba(150, 80, 200, 120);
        let mut mesh = Mesh::default();
        text.paint(&mut mesh, caption, Vec2::ZERO, 16.0, 90.0, color, 1.5);
        assert!(Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 90.0)));
        assert!(!mesh.vertices.is_empty());
        assert!(mesh.vertices.iter().all(|v| v.color == color.linear()));
        assert!(!Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 180.0)));
        assert!(!Arc::ptr_eq(&layout, &text.layout(caption, 20.0, 90.0)));
        text.end_frame();
        text.begin_frame();
        assert!(Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 90.0)));
        text.end_frame();
        assert_eq!(text.layouts.len(), 1);
        text.begin_frame();
        text.measure("replacement", 16.0, 90.0);
        text.end_frame();
        assert_eq!(text.layouts.len(), 1);
        assert!(!Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 90.0)));
    }

    #[test]
    fn cache_key_collision_cannot_substitute_another_caption() {
        let mut text = text();
        let wanted = text.build_layout("AV", 16.0, f32::INFINITY);
        text.layouts.insert(
            Id::new(("AV", 16.0_f32.to_bits(), f32::INFINITY.to_bits())),
            CachedLayout {
                text: "wrong".into(),
                size: 16.0_f32.to_bits(),
                wrap: f32::INFINITY.to_bits(),
                layout: Arc::new(text.build_layout("wrong", 16.0, f32::INFINITY)),
                last_frame: 0,
            },
        );
        let layout = text.layout("AV", 16.0, f32::INFINITY);
        assert_eq!(layout.carets, wanted.carets);
        assert_eq!(layout.glyphs.len(), wanted.glyphs.len());
        assert_eq!(layout.size, wanted.size);
    }

    #[test]
    #[cfg(feature = "bundled-emoji")]
    fn unicode_clusters_use_real_glyphs_and_color_emoji_at_multiple_scales() {
        let mut text = text();
        for sample in ["е\u{301}", "👩‍💻", "🇷🇺", "👍🏽"] {
            let layout = text.layout(sample, 16.0, f32::INFINITY);
            assert!(
                layout.glyphs.iter().all(|(g, _)| g.glyph_id != 0),
                "{sample}"
            );
            if sample == "е\u{301}" {
                // A combining accent can be a separate positioned glyph in a
                // valid font; the grapheme still has only its two caret edges.
                assert!((1..=2).contains(&layout.glyphs.len()));
            } else {
                assert_eq!(layout.glyphs.len(), 1, "{sample} must shape as one glyph");
            }
            assert_eq!(layout.carets.len(), 2, "{sample} has one grapheme");
            assert_eq!(layout.carets[0], (0, 0.0));
            assert!((layout.carets[1].1 - layout.size.x).abs() < 0.001);
            for scale in [1.0, 1.5, 2.0] {
                let mut mesh = Mesh::default();
                text.paint(
                    &mut mesh,
                    sample,
                    Vec2::ZERO,
                    16.0,
                    f32::INFINITY,
                    Color::WHITE,
                    scale,
                );
                assert!(!mesh.vertices.is_empty());
                if sample != "е\u{301}" {
                    assert!(text.glyphs.values().flatten().any(|g| g.colored));
                    assert!(text.pages.iter().any(|page| page
                        .image
                        .pixels
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|p| p[3] != 0 && (p[0] != p[1] || p[1] != p[2]))));
                }
            }
        }
    }

    #[test]
    fn caret_positions_follow_shaped_clusters_without_skipping_adjacent_letters() {
        let mut text = text();
        let layout = text.layout("AV ffi е\u{301}👩‍💻", 16.0, f32::INFINITY);
        assert!(layout.carets.windows(2).all(|pair| pair[0].1 <= pair[1].1));
        assert!(layout.carets[1].1 > 0.0);
        let v = layout.glyphs.iter().find(|(g, _)| g.start == 1).unwrap();
        assert!((layout.carets[1].1 - v.0.x).abs() < 0.001);
        assert!(layout.carets.iter().all(|(i, _)| "AV ffi е\u{301}👩‍💻"
            .grapheme_indices(true)
            .any(|(start, _)| start == *i)
            || *i == "AV ffi е\u{301}👩‍💻".len()));
    }

    #[test]
    fn centered_field_baseline_matches_the_fonts_cap_and_descender_band() {
        let mut text = text();
        for size in [13.0, 14.0, 20.0] {
            let layout = text.layout("Привет", size, f32::INFINITY);
            let em_scale = size * text.font.height_unscaled() / text.font.units_per_em().unwrap();
            let h = text
                .font
                .outline_glyph(Glyph {
                    id: text.font.glyph_id('H'),
                    scale: em_scale.into(),
                    position: point(0.0, 0.0),
                })
                .unwrap()
                .px_bounds();
            let g = text
                .font
                .outline_glyph(Glyph {
                    id: text.font.glyph_id('g'),
                    scale: em_scale.into(),
                    position: point(0.0, 0.0),
                })
                .unwrap()
                .px_bounds();
            let offset = text.centered_line_offset("Привет", size);
            let top = offset + layout.baseline + h.min.y;
            let bottom = offset + layout.baseline + g.max.y;
            assert!((top + bottom - layout.size.y).abs() < 0.001);
            assert!((offset - text.centered_line_offset("Placeholder", size)).abs() < 0.001);
        }
    }
}
