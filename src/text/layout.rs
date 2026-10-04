use super::*;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};

/// Everything about a font that changes shaping or glyph shapes. Part of every
/// layout key; the glyph atlas key carries the same face and weight through
/// cosmic-text's `CacheKey`. `style` is shaped but always `Normal` for now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FontKey {
    pub family: Id,
    pub weight: FontWeight,
    pub style: fontdb::Style,
}

impl FontKey {
    pub(super) fn new(family: Id, weight: FontWeight) -> Self {
        Self {
            family,
            weight,
            style: fontdb::Style::Normal,
        }
    }
}

/// Width of a tab in space advances; cosmic-text's own default.
pub(crate) const DEFAULT_TAB: u16 = 8;

pub(super) struct CachedLayout {
    pub text: String,
    pub size: u32,
    pub wrap: u32,
    pub font: FontKey,
    pub tab: u16,
    pub layout: Arc<TextLayout>,
    pub last_frame: u64,
}

impl TextSystem {
    pub(crate) fn measure_with_wrap(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        wrap: f32,
    ) -> (Vec2, f32) {
        let layout = self.layout(text, size, weight, wrap);
        let effective = if layout.width_independent && wrap >= layout.size.x {
            f32::INFINITY
        } else {
            wrap
        };
        (layout.size, effective)
    }
    pub fn measure(&mut self, text: &str, size: f32, weight: FontWeight, wrap_width: f32) -> Vec2 {
        self.layout(text, size, weight, wrap_width).size
    }

    /// Insertion positions from the same kerning and fallback metrics used for painting.
    pub fn carets(&mut self, text: &str, size: f32, weight: FontWeight) -> Vec<(usize, f32)> {
        self.layout(text, size, weight, f32::INFINITY)
            .carets
            .clone()
    }

    pub(super) fn layout(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        wrap_width: f32,
    ) -> Arc<TextLayout> {
        self.layout_with_tab(text, size, weight, wrap_width, DEFAULT_TAB)
    }

    /// The one shaping entry point. A tab is `tab` space advances wide and is part of the key.
    pub(crate) fn layout_with_tab(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        wrap_width: f32,
        tab: u16,
    ) -> Arc<TextLayout> {
        let font = self.font_key(weight);
        let key = Id::new((text, size.to_bits(), wrap_width.to_bits(), font, tab));
        if let Some(cached) = self.layouts.get_mut(&key) {
            // Verify the key contents as well: a hash collision must never reuse
            // another string's glyph positions.
            if cached.text == text
                && cached.size == size.to_bits()
                && cached.wrap == wrap_width.to_bits()
                && cached.font == font
                && cached.tab == tab
            {
                cached.last_frame = self.frame;
                return Arc::clone(&cached.layout);
            }
        }
        let text_key = Id::new((text, size.to_bits(), font, tab));
        // An unwrapped LTR layout has identical glyph positions at every width
        // large enough to contain it. Reflow only when the wrapping threshold is crossed.
        let reusable = self
            .layout_keys
            .get(&text_key)
            .and_then(|key| self.layouts.get(key))
            .filter(|c| {
                c.text == text
                    && c.size == size.to_bits()
                    && c.font == font
                    && c.tab == tab
                    && c.layout.width_independent
                    && wrap_width >= c.layout.size.x
            })
            .map(|c| Arc::clone(&c.layout));
        let layout = reusable.unwrap_or_else(|| {
            self.builds += 1;
            Arc::new(self.build_layout_with_tab(text, size, font, wrap_width, tab))
        });
        self.layout_keys.insert(text_key, key);
        self.layouts.insert(
            key,
            CachedLayout {
                text: text.to_owned(),
                size: size.to_bits(),
                wrap: wrap_width.to_bits(),
                font,
                tab,
                layout: Arc::clone(&layout),
                last_frame: self.frame,
            },
        );
        layout
    }

    #[cfg(test)]
    pub(super) fn build_layout(
        &self,
        text: &str,
        size: f32,
        font: FontKey,
        wrap_width: f32,
    ) -> TextLayout {
        self.build_layout_with_tab(text, size, font, wrap_width, DEFAULT_TAB)
    }

    fn build_layout_with_tab(
        &self,
        text: &str,
        size: f32,
        font: FontKey,
        wrap_width: f32,
        tab: u16,
    ) -> TextLayout {
        let line_height = size * 1.25;
        let mut fonts = font_system().lock().unwrap();
        let mut buffer = Buffer::new(&mut fonts, Metrics::new(size, line_height));
        buffer.set_tab_width(&mut fonts, tab);
        buffer.set_wrap(
            &mut fonts,
            if wrap_width.is_finite() {
                Wrap::WordOrGlyph
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
            &Attrs::new()
                .family(Family::Name(&self.family.name))
                .weight(font.weight.to_fontdb())
                .style(font.style),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut fonts, false);
        let mut glyphs = Vec::new();
        let mut lines = Vec::new();
        let mut width = 0.0_f32;
        let mut height = line_height;
        let mut baseline = 0.0;
        let mut previous_run = None;
        let mut width_independent = true;
        for run in buffer.layout_runs() {
            width_independent &= previous_run != Some(run.line_i) && !run.rtl;
            previous_run = Some(run.line_i);
            if glyphs.is_empty() {
                baseline = run.line_y;
            }
            width = width.max(run.line_w);
            height = height.max(run.line_top + run.line_height);
            lines.push(lines::build_line(&run));
            glyphs.extend(run.glyphs.iter().cloned().map(|g| (g, run.line_y)));
        }
        if lines.is_empty() {
            lines.push(lines::empty_line(line_height));
        }
        TextLayout {
            glyphs,
            baseline,
            carets: lines::carets(&lines),
            lines,
            size: Vec2::new(width, height),
            width_independent,
        }
    }
}
