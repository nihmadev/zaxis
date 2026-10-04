use super::*;

/// Everything about a font that changes shaping or glyph shapes. Part of every
/// layout key; the glyph atlas key carries the same face and weight through
/// cosmic-text's `CacheKey`. `style` is shaped but always `Normal` for now.
/// The family is part of the key (`family`, `monospace`), so changing one family
/// never touches the cached layouts of the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FontKey {
    pub family: Id,
    pub weight: FontWeight,
    pub style: fontdb::Style,
    pub monospace: bool,
    pub tabular: bool,
}

impl FontKey {
    pub(super) fn new(family: Id, weight: FontWeight, monospace: bool, tabular: bool) -> Self {
        Self {
            family,
            weight,
            style: fontdb::Style::Normal,
            monospace,
            tabular,
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
        font: impl Into<TextFont>,
        wrap: f32,
    ) -> (Vec2, f32) {
        self.measure_with_wrap_tab(text, size, font, wrap, DEFAULT_TAB)
    }
    /// [`Self::measure_with_wrap`] for text painted with an explicit tab width.
    pub(crate) fn measure_with_wrap_tab(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap: f32,
        tab: u16,
    ) -> (Vec2, f32) {
        let layout = self.layout_with_tab(text, size, font, wrap, tab);
        let effective = if layout.width_independent && wrap >= layout.size.x {
            f32::INFINITY
        } else {
            wrap
        };
        (layout.size, effective)
    }
    pub fn measure(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap_width: f32,
    ) -> Vec2 {
        self.layout(text, size, font, wrap_width).size
    }

    /// Cell width and line height of the monospace family at `size` and `weight`.
    pub fn monospace_metrics(&mut self, size: f32, weight: FontWeight) -> MonospaceMetrics {
        let font = TextFont::new(weight, TextFamily::Monospace, false);
        // The advance of `0` is the cell of the primary font. The layout is the cached one
        // that painting shares, so asking every frame costs nothing.
        let layout = self.layout("0", size, font, f32::INFINITY);
        MonospaceMetrics {
            cell_width: layout.size.x,
            line_height: layout.size.y,
        }
    }

    /// Insertion positions from the same kerning and fallback metrics used for painting.
    pub fn carets(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
    ) -> Vec<(usize, f32)> {
        self.layout(text, size, font, f32::INFINITY).carets.clone()
    }

    pub(super) fn layout(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap_width: f32,
    ) -> Arc<TextLayout> {
        self.layout_with_tab(text, size, font, wrap_width, DEFAULT_TAB)
    }

    /// The one shaping entry point. A tab is `tab` space advances wide and is part of the key.
    pub(crate) fn layout_with_tab(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap_width: f32,
        tab: u16,
    ) -> Arc<TextLayout> {
        let spec = font.into();
        let font = self.font_key(spec);
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
            // Tab stops of monospace text sit a whole number of cells apart.
            let stop = (font.monospace && text.contains('\t'))
                .then(|| self.layout("0", size, spec, f32::INFINITY).size.x * f32::from(tab));
            Arc::new(self.build_layout_with_tab(text, size, font, wrap_width, tab, stop))
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
}
