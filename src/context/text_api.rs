//! Text measurement for components. Every call is keyed by size and weight and
//! resolves to the same cached layout that painting and carets use.

use super::Context;
use crate::{
    text::{TextFont, TextLayout},
    FontWeight, MonospaceMetrics, Vec2,
};
use std::sync::Arc;

impl Context {
    pub(crate) fn centered_line_offset(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
    ) -> f32 {
        self.text.centered_line_offset(text, size, font)
    }

    pub(crate) fn text_carets(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
    ) -> Vec<(usize, f32)> {
        self.text.carets(text, size, font)
    }

    pub(crate) fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap: f32,
    ) -> Vec2 {
        self.text.measure(text, size, font, wrap)
    }
    pub(crate) fn measure_text_layout(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap: f32,
    ) -> (Vec2, f32) {
        self.text.measure_with_wrap(text, size, font, wrap)
    }

    /// [`Context::measure_text_layout`] for text painted with an explicit tab width.
    pub(crate) fn measure_text_layout_tab(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap: f32,
        tab: u16,
    ) -> (Vec2, f32) {
        self.text.measure_with_wrap_tab(text, size, font, wrap, tab)
    }

    /// Shape one paragraph (no line breaks) through the same cache that painting uses.
    pub(crate) fn paragraph_layout(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap: f32,
        tab: u16,
    ) -> Arc<TextLayout> {
        self.text.layout_with_tab(text, size, font, wrap, tab)
    }

    /// Cell width and line height of the monospace family at `size` and `weight`, for sizing
    /// code views and [`ScrollArea::show_rows`](crate::ScrollArea::show_rows) without
    /// estimates. They come from the layout that paints monospace text: a string of `n`
    /// ASCII characters is `n * cell_width` wide.
    pub fn monospace_metrics(&mut self, size: f32, weight: FontWeight) -> MonospaceMetrics {
        self.text.monospace_metrics(size, weight)
    }

    /// Paragraph layouts shaped from scratch so far; cache reuse never increments it.
    pub fn text_layouts_built(&self) -> u64 {
        self.text.builds
    }
}
