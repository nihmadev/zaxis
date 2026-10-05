//! Ellipsis. A block that is limited to some lines, or that does not wrap and does not fit,
//! shows a prefix of its text and a `…`. The cut is found from the clusters of the layout
//! that is painted, so it falls on a grapheme boundary and the ellipsis really fits.

use super::state::BlockState;
use crate::{
    components::text_edit::doc::Doc,
    context::Context,
    text::{TextFont, TextLayout},
};

pub(crate) const ELLIPSIS: &str = "…";

/// Source byte at which the text must be cut so that at most `max_lines` lines remain and
/// the last one ends with an ellipsis within `avail`; `None` when everything fits.
/// `doc` must hold the full text.
pub(crate) fn cut_point(
    ctx: &mut Context,
    doc: &mut Doc,
    text: &str,
    max_lines: usize,
    avail: f32,
    wrap: bool,
    size: f32,
    font: TextFont,
) -> Option<usize> {
    let mut remaining = max_lines;
    for i in 0..doc.paras.len() {
        let layout: std::sync::Arc<TextLayout> = doc.layout(ctx, text, i);
        let count = layout.lines.len();
        if count < remaining {
            let overflow = !wrap && avail.is_finite() && line_width(&layout, count - 1) > avail;
            if !overflow {
                remaining -= count;
                continue;
            }
            remaining = count;
        }
        let k = remaining - 1;
        let more = k + 1 < count || i + 1 < doc.paras.len();
        let overflow = !wrap && avail.is_finite() && line_width(&layout, k) > avail;
        if !more && !overflow {
            return None;
        }
        let ellipsis = ctx.measure_text(ELLIPSIS, size, font, f32::INFINITY).x;
        let line = &layout.lines[k];
        let room = if avail.is_finite() {
            avail - ellipsis
        } else {
            f32::INFINITY
        };
        let end = line
            .clusters
            .iter()
            .filter(|c| c.end <= line.end && c.x0.max(c.x1) <= room)
            .map(|c| c.end)
            .max()
            .unwrap_or(line.start);
        return Some(doc.paras[i].start + end);
    }
    None
}

fn line_width(layout: &TextLayout, line: usize) -> f32 {
    layout.lines[line]
        .clusters
        .iter()
        .map(|c| c.x1.max(c.x0))
        .fold(0.0, f32::max)
}

impl BlockState {
    /// Show `text[..cut]` and an ellipsis, or the whole text for `None`.
    pub fn set_cut(&mut self, cut: Option<usize>) {
        if self.cut == cut {
            return;
        }
        self.cut = cut;
        self.display = cut.map(|k| format!("{}{ELLIPSIS}", &self.text[..k]));
        self.built = None;
    }
}
