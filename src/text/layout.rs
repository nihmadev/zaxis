use super::*;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};
use unicode_segmentation::UnicodeSegmentation;

impl TextSystem {
    pub(crate) fn measure_with_wrap(&mut self, text: &str, size: f32, wrap: f32) -> (Vec2, f32) {
        let layout = self.layout(text, size, wrap);
        let effective = if layout.width_independent && wrap >= layout.size.x {
            f32::INFINITY
        } else {
            wrap
        };
        (layout.size, effective)
    }
    pub fn measure(&mut self, text: &str, size: f32, wrap_width: f32) -> Vec2 {
        self.layout(text, size, wrap_width).size
    }

    /// Insertion positions from the same kerning and fallback metrics used for painting.
    pub fn carets(&mut self, text: &str, size: f32) -> Vec<(usize, f32)> {
        self.layout(text, size, f32::INFINITY).carets.clone()
    }

    pub(super) fn layout(&mut self, text: &str, size: f32, wrap_width: f32) -> Arc<TextLayout> {
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
        let text_key = Id::new((text, size.to_bits()));
        // An unwrapped LTR layout has identical glyph positions at every width
        // large enough to contain it. Reflow only when the wrapping threshold is crossed.
        let reusable = self
            .layout_keys
            .get(&text_key)
            .and_then(|key| self.layouts.get(key))
            .filter(|c| {
                c.text == text
                    && c.size == size.to_bits()
                    && c.layout.width_independent
                    && wrap_width >= c.layout.size.x
            })
            .map(|c| Arc::clone(&c.layout));
        let layout =
            reusable.unwrap_or_else(|| Arc::new(self.build_layout(text, size, wrap_width)));
        self.layout_keys.insert(text_key, key);
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

    pub(super) fn build_layout(&self, text: &str, size: f32, wrap_width: f32) -> TextLayout {
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
        let mut previous_run = None;
        let mut width_independent = true;
        for run in buffer.layout_runs() {
            width_independent &= previous_run != Some(run.line_i) && !run.rtl;
            previous_run = Some(run.line_i);
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
            width_independent,
        }
    }
}
