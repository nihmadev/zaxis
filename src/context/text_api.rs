//! Text measurement for components. Every call is keyed by size and weight and
//! resolves to the same cached layout that painting and carets use.

use super::Context;
use crate::{text::TextLayout, FontWeight, Vec2};
use std::sync::Arc;

impl Context {
    pub(crate) fn centered_line_offset(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
    ) -> f32 {
        self.text.centered_line_offset(text, size, weight)
    }

    pub(crate) fn text_carets(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
    ) -> Vec<(usize, f32)> {
        self.text.carets(text, size, weight)
    }

    pub(crate) fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        wrap: f32,
    ) -> Vec2 {
        self.text.measure(text, size, weight, wrap)
    }
    pub(crate) fn measure_text_layout(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        wrap: f32,
    ) -> (Vec2, f32) {
        self.text.measure_with_wrap(text, size, weight, wrap)
    }

    /// Shape one paragraph (no line breaks) through the same cache that painting uses.
    pub(crate) fn paragraph_layout(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        wrap: f32,
        tab: u16,
    ) -> Arc<TextLayout> {
        self.text.layout_with_tab(text, size, weight, wrap, tab)
    }

    /// Paragraph layouts shaped from scratch so far; cache reuse never increments it.
    pub fn text_layouts_built(&self) -> u64 {
        self.text.builds
    }
}
