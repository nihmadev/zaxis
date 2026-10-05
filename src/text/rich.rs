//! Styled runs inside one paragraph. A run changes weight, family or color of a byte range
//! without leaving the shared shaping pass, so wrapping, kerning, clusters and carets stay
//! those of a single layout. Size is uniform; decorations (underlines, link areas) are not
//! part of shaping and are drawn from the layout's lines by the component.

use super::*;

/// One styled byte range of a paragraph. Runs of a layout are contiguous and cover the
/// whole text; `StyleRun::cover` fills the gaps of a sparse list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StyleRun {
    pub start: usize,
    pub end: usize,
    pub weight: Option<FontWeight>,
    pub monospace: bool,
    pub color: Option<Color>,
}

impl StyleRun {
    pub(crate) const fn plain(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            weight: None,
            monospace: false,
            color: None,
        }
    }

    /// Whether shaping treats the two runs alike. Color is applied when painting and never
    /// changes glyph positions, so it is not part of the shape.
    pub(crate) fn same_shape(&self, other: &Self) -> bool {
        (self.start, self.end, self.weight, self.monospace)
            == (other.start, other.end, other.weight, other.monospace)
    }

    /// The font of this run on top of the paragraph's base font.
    pub(crate) fn font(&self, base: TextFont) -> TextFont {
        TextFont::new(
            self.weight.unwrap_or(base.weight),
            if self.monospace {
                TextFamily::Monospace
            } else {
                base.family
            },
            base.tabular,
        )
    }

    /// `runs` (sorted, non-overlapping, inside `0..len`) with the gaps between them filled
    /// by plain runs; invalid or empty ranges are dropped.
    pub(crate) fn cover(runs: &[StyleRun], len: usize) -> Vec<StyleRun> {
        let mut out: Vec<StyleRun> = Vec::with_capacity(runs.len() * 2 + 1);
        let mut at = 0;
        for run in runs {
            if run.start < at || run.end <= run.start || run.end > len {
                continue;
            }
            if run.start > at {
                out.push(Self::plain(at, run.start));
            }
            out.push(*run);
            at = run.end;
        }
        if at < len {
            out.push(Self::plain(at, len));
        }
        out
    }
}

/// Hashes runs without their colors, so a color change reuses the cached shaping.
struct Shape<'a>(&'a [StyleRun]);

impl std::hash::Hash for Shape<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for run in self.0 {
            (run.start, run.end, run.weight, run.monospace).hash(state);
        }
    }
}

/// Whether two run lists shape alike; see [`StyleRun::same_shape`].
pub(crate) fn same_shape(a: &[StyleRun], b: &[StyleRun]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.same_shape(b))
}

pub(super) struct RichCached {
    text: String,
    size: u32,
    wrap: u32,
    font: FontKey,
    tab: u16,
    runs: Arc<[StyleRun]>,
    layout: Arc<TextLayout>,
    pub last_frame: u64,
}

impl TextSystem {
    /// Layout of one paragraph with styled runs through a cache of its own, bounded like
    /// the plain one: entries not used during a frame are dropped at its end. `runs` must
    /// cover `text` (see [`StyleRun::cover`]).
    pub(crate) fn rich_layout(
        &mut self,
        text: &str,
        runs: &Arc<[StyleRun]>,
        size: f32,
        font: impl Into<TextFont>,
        wrap_width: f32,
        tab: u16,
    ) -> Arc<TextLayout> {
        let spec = font.into();
        let font = self.font_key(spec);
        let key = Id::new((
            text,
            size.to_bits(),
            wrap_width.to_bits(),
            font,
            tab,
            Shape(runs),
        ));
        if let Some(cached) = self.rich.get_mut(&key) {
            if cached.text == text
                && cached.size == size.to_bits()
                && cached.wrap == wrap_width.to_bits()
                && cached.font == font
                && cached.tab == tab
                && same_shape(&cached.runs, runs)
            {
                cached.last_frame = self.frame;
                return Arc::clone(&cached.layout);
            }
        }
        self.builds += 1;
        let layout = Arc::new(self.build_layout_runs(
            text,
            size,
            font,
            wrap_width,
            tab,
            None,
            Some((runs, spec)),
        ));
        self.rich.insert(
            key,
            RichCached {
                text: text.to_owned(),
                size: size.to_bits(),
                wrap: wrap_width.to_bits(),
                font,
                tab,
                runs: Arc::clone(runs),
                layout: Arc::clone(&layout),
                last_frame: self.frame,
            },
        );
        layout
    }

    /// [`Self::translation_preserves_raster`] for a layout with styled runs.
    pub(crate) fn rich_translation_preserves_raster(
        &mut self,
        text: &str,
        runs: &Arc<[StyleRun]>,
        position: Vec2,
        size: f32,
        font: TextFont,
        wrap_width: f32,
        tab: u16,
        delta: Vec2,
        scale: f32,
    ) -> bool {
        let layout = self.rich_layout(text, runs, size, font, wrap_width, tab);
        let moved = position + delta;
        layout.glyphs.iter().all(|(glyph, baseline)| {
            let old = glyph.physical((position.x * scale, (position.y + baseline) * scale), scale);
            let new = glyph.physical((moved.x * scale, (moved.y + baseline) * scale), scale);
            old.cache_key == new.cache_key
        })
    }

    #[allow(clippy::too_many_arguments)]
    /// Paint a layout built by [`Self::rich_layout`]; glyphs take the color of their run,
    /// or `color`, scaled by the alpha of `color`.
    pub(crate) fn paint_rich(
        &mut self,
        mesh: &mut Mesh,
        text: &str,
        runs: &Arc<[StyleRun]>,
        position: Vec2,
        size: f32,
        font: TextFont,
        wrap_width: f32,
        tab: u16,
        color: Color,
        scale: f32,
    ) {
        let layout = self.rich_layout(text, runs, size, font, wrap_width, tab);
        let mut store = self.store.lock().expect("glyph store mutex");
        for (glyph, baseline) in &layout.glyphs {
            let physical =
                glyph.physical((position.x * scale, (position.y + baseline) * scale), scale);
            let Some(cached) = store.glyph(physical.cache_key) else {
                continue;
            };
            let tint = match runs.get(glyph.metadata).and_then(|run| run.color) {
                Some(mut own) => {
                    own.0[3] = (u16::from(own.0[3]) * u16::from(color.0[3]) / 255) as u8;
                    own
                }
                None => color,
            };
            let tint = if cached.colored {
                Color::rgba(255, 255, 255, tint.0[3])
            } else {
                tint
            };
            let min =
                Vec2::new(physical.x as f32, physical.y as f32) / scale + cached.offset / scale;
            mesh.quad(
                Rect::from_min_size(min, cached.size / scale),
                cached.uv,
                tint.linear(),
                cached.texture,
            );
        }
    }
}
