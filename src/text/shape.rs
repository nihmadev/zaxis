//! Shaping with cosmic-text: one buffer per layout, with the family and figure
//! features of the requested font applied.

use super::*;
use cosmic_text::{
    Attrs, Buffer, Family, FeatureTag, FontFeatures, LayoutGlyph, Metrics, Shaping, Wrap,
};

const TABULAR_FIGURES: FeatureTag = FeatureTag::new(b"tnum");

/// OpenType features of a request. Monospace text turns the code ligatures off: a
/// glyph that spans several cells would otherwise differ from the characters it replaces.
fn features(font: FontKey) -> FontFeatures {
    let mut features = FontFeatures::new();
    if font.tabular {
        features.enable(TABULAR_FIGURES);
    }
    if font.monospace {
        features
            .disable(FeatureTag::STANDARD_LIGATURES)
            .disable(FeatureTag::CONTEXTUAL_LIGATURES)
            .disable(FeatureTag::CONTEXTUAL_ALTERNATES);
    }
    features
}

impl TextSystem {
    /// Shape without touching the caches.
    pub fn build_layout(
        &self,
        text: &str,
        size: f32,
        font: FontKey,
        wrap_width: f32,
    ) -> TextLayout {
        self.build_layout_with_tab(text, size, font, wrap_width, DEFAULT_TAB, None)
    }

    /// Shape `text`. Monospace text is shaped like any other: the width of a line is its real
    /// shaped width. Glyphs of the primary font advance one cell each; a glyph from a
    /// fallback font (CJK, emoji) keeps its own advance, which is not a whole number of cells.
    /// `tab_stop` is the distance between tab stops of monospace text, see [`snap_tabs`].
    pub(super) fn build_layout_with_tab(
        &self,
        text: &str,
        size: f32,
        font: FontKey,
        wrap_width: f32,
        tab: u16,
        tab_stop: Option<f32>,
    ) -> TextLayout {
        self.build_layout_runs(text, size, font, wrap_width, tab, tab_stop, None)
    }

    /// The attributes a face request is shaped with.
    fn attrs(&self, font: FontKey) -> Attrs<'_> {
        let name = if font.monospace {
            self.mono.as_ref().map(|r| r.name.as_str())
        } else {
            Some(self.family.name.as_str())
        };
        let family = name.map_or(Family::Monospace, Family::Name);
        let mut attrs = Attrs::new()
            .family(family)
            .weight(font.weight.to_fontdb())
            .style(font.style);
        if font.monospace || font.tabular {
            attrs = attrs.font_features(features(font));
        }
        attrs
    }

    /// [`Self::build_layout_with_tab`] with styled runs: each run is shaped with its own
    /// weight and family inside the one buffer, so kerning, wrapping and bidi treat the
    /// text as a single paragraph. `run.metadata` of every glyph is the run index.
    pub(super) fn build_layout_runs(
        &self,
        text: &str,
        size: f32,
        font: FontKey,
        wrap_width: f32,
        tab: u16,
        tab_stop: Option<f32>,
        runs: Option<(&[StyleRun], TextFont)>,
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
        let attrs = self.attrs(font);
        match runs {
            None => buffer.set_text(&mut fonts, text, &attrs, Shaping::Advanced, None),
            Some((runs, base)) => {
                let spans = runs.iter().enumerate().map(|(i, run)| {
                    let spec = run.font(base);
                    (
                        &text[run.start..run.end],
                        self.attrs(self.font_key(spec)).metadata(i),
                    )
                });
                buffer.set_rich_text(&mut fonts, spans, &attrs, Shaping::Advanced, None);
            }
        }
        buffer.shape_until_scroll(&mut fonts, false);
        let mut glyphs = Vec::new();
        let mut lines = Vec::new();
        let mut width = 0.0_f32;
        let mut height = line_height;
        let mut baseline = 0.0;
        let mut previous_run = None;
        let mut width_independent = true;
        let (mut tab_line, mut tab_base) = (usize::MAX, 0.0);
        for run in buffer.layout_runs() {
            width_independent &= previous_run != Some(run.line_i) && !run.rtl;
            previous_run = Some(run.line_i);
            if glyphs.is_empty() {
                baseline = run.line_y;
            }
            let mut run_glyphs = run.glyphs.to_vec();
            let mut line_w = run.line_w;
            if let Some(stop) = tab_stop.filter(|_| !run.rtl) {
                if tab_line != run.line_i {
                    (tab_line, tab_base) = (run.line_i, 0.0);
                }
                line_w += snap_tabs(run.text, &mut run_glyphs, tab_base, stop);
                tab_base += line_w;
            }
            width = width.max(line_w);
            height = height.max(run.line_top + run.line_height);
            lines.push(lines::build_line(&run, &run_glyphs));
            glyphs.extend(run_glyphs.into_iter().map(|g| (g, run.line_y)));
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

/// Put every tab of an unwrapped-or-wrapped LTR run on the next stop, `stop` apart, counted
/// from the start of the paragraph (`base` is the width of its earlier rows). cosmic-text
/// finds the stop with `floor(x / stop)`, and the rounding noise of summed glyph advances
/// makes that `floor` land one stop early when a tab begins exactly on a stop, which
/// collapses it to zero width. This repeats the same rule with a tolerance and moves the
/// glyphs behind each tab; it adds no second measurement. Returns the total shift.
fn snap_tabs(text: &str, glyphs: &mut [LayoutGlyph], base: f32, stop: f32) -> f32 {
    let mut shift = 0.0;
    for glyph in glyphs {
        glyph.x += shift;
        if stop > 0.0 && text.get(glyph.start..glyph.end) == Some("\t") {
            let x = base + glyph.x;
            let width = ((x / stop + 1e-3).floor() + 1.0) * stop - x;
            shift += width - glyph.w;
            glyph.w = width;
        }
    }
    shift
}
