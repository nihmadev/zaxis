//! Geometry of shaped paragraphs, shared by [`TextEdit`](super::TextEdit) and static text.
//! Hit testing, selection rectangles and pixel snapping read the lines and clusters of
//! one cached layout, the same positions the paragraph is painted with, so a point maps
//! to the glyph under it and a range maps to exactly the glyphs it covers.

use crate::{
    text::{TextLayout, VisualLine},
    Rect, Vec2,
};
use std::ops::Range;

/// The visual line holding `y` (paragraph space); the last line for any `y` below it.
pub(crate) fn line_at(lines: &[VisualLine], y: f32) -> usize {
    lines
        .partition_point(|l| l.top + l.height <= y)
        .min(lines.len().saturating_sub(1))
}

/// Bottom edge of line `k`: the top of the next line when there is one, so adjacent lines
/// share an edge exactly and never leave a seam.
pub(crate) fn line_bottom(lines: &[VisualLine], k: usize) -> f32 {
    lines
        .get(k + 1)
        .map_or(lines[k].top + lines[k].height, |next| next.top)
}

/// Where a point lands in a paragraph.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Hit {
    pub line: usize,
    /// The grapheme boundary nearest to the point.
    pub byte: usize,
    /// Start of the grapheme under (or nearest to) the point.
    pub cell: usize,
}

/// Hit test of `point` (paragraph space) against `layout`.
pub(crate) fn hit(layout: &TextLayout, point: Vec2) -> Hit {
    let line = line_at(&layout.lines, point.y);
    let (byte, cell) = layout.lines[line].hit(point.x);
    Hit { line, byte, cell }
}

/// Rectangles covering `range` (paragraph-relative bytes) in a paragraph of `len` bytes whose
/// top is `top`. Bidi text can give several rectangles per line. When the paragraph's line
/// break is part of the selection (`range` reaches past `len`), a `slab`-wide stub marks it.
pub(crate) fn paragraph_rects(
    layout: &TextLayout,
    range: Range<usize>,
    len: usize,
    slab: f32,
    top: f32,
    out: &mut Vec<Rect>,
) {
    let mut slabs = Vec::new();
    paragraph_slabs(layout, range, len, slab, top, &mut slabs);
    out.extend(slabs.into_iter().map(|(rect, _)| rect));
}

/// [`paragraph_rects`] with the baseline (paragraph space plus `top`) of each rectangle's line,
/// for decorations such as underlines.
pub(crate) fn paragraph_slabs(
    layout: &TextLayout,
    range: Range<usize>,
    len: usize,
    slab: f32,
    top: f32,
    out: &mut Vec<(Rect, f32)>,
) {
    let count = layout.lines.len();
    for (k, l) in layout.lines.iter().enumerate() {
        let last = k + 1 == count;
        let end = if last { len } else { l.end };
        let (a, b) = (range.start.max(l.start), range.end.min(end));
        let bottom = line_bottom(&layout.lines, k);
        let baseline = top + l.baseline;
        let at = |x0: f32, x1: f32| {
            (
                Rect::from_min_max(Vec2::new(x0, top + l.top), Vec2::new(x1, top + bottom)),
                baseline,
            )
        };
        if a < b {
            out.extend(l.spans(a..b).into_iter().map(|(x0, x1)| at(x0, x1)));
        }
        if last && range.start <= len && range.end > len {
            let edge = l.clusters.last().map_or(0.0, |c| c.trailing());
            out.push(at(edge, edge + slab));
        }
    }
}

/// `value` on the physical pixel grid of a surface with `scale` pixels per logical pixel.
pub(crate) fn snap(value: f32, scale: f32) -> f32 {
    if scale > 0.0 && value.is_finite() {
        (value * scale).round() / scale
    } else {
        value
    }
}

/// `rect` with every edge on the pixel grid. Neighbours that share an edge still share it,
/// so stacked line backgrounds neither overlap nor leave a gap at any scale factor.
pub(crate) fn snap_rect(rect: Rect, scale: f32) -> Rect {
    Rect::from_min_max(
        Vec2::new(snap(rect.min.x, scale), snap(rect.min.y, scale)),
        Vec2::new(snap(rect.max.x, scale), snap(rect.max.y, scale)),
    )
}
