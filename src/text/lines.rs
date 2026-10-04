//! Visual lines and grapheme extents of one shaped paragraph. Every consumer of a
//! layout (carets, hit testing, selection rectangles, painting) reads these same
//! cosmic-text results; nothing here measures text a second way.

use std::ops::Range;

use cosmic_text::{LayoutGlyph, LayoutRun};
use unicode_segmentation::UnicodeSegmentation;

/// One grapheme cluster of a visual line, in paragraph byte coordinates.
/// `x0 <= x1` regardless of direction; `rtl` tells which edge is the leading one.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cluster {
    pub start: usize,
    pub end: usize,
    pub x0: f32,
    pub x1: f32,
    pub rtl: bool,
}

impl Cluster {
    pub fn leading(&self) -> f32 {
        if self.rtl {
            self.x1
        } else {
            self.x0
        }
    }
    pub fn trailing(&self) -> f32 {
        if self.rtl {
            self.x0
        } else {
            self.x1
        }
    }
}

/// A line as laid out by the wrapper. `start..end` is its logical byte range;
/// `clusters` are in logical order, so the visual order may differ in bidi text.
#[derive(Clone, Debug)]
pub(crate) struct VisualLine {
    pub top: f32,
    pub height: f32,
    pub start: usize,
    pub end: usize,
    pub clusters: Vec<Cluster>,
}

impl VisualLine {
    /// Horizontal caret position at a grapheme boundary of this line.
    pub fn caret_x(&self, byte: usize) -> Option<f32> {
        match self.clusters.binary_search_by_key(&byte, |c| c.start) {
            Ok(i) => Some(self.clusters[i].leading()),
            Err(i) if i == self.clusters.len() && byte == self.end => {
                Some(self.clusters.last().map_or(0.0, Cluster::trailing))
            }
            Err(_) => None,
        }
    }

    /// The boundary nearest to `x`, plus the start of the grapheme under `x`.
    pub fn hit(&self, x: f32) -> (usize, usize) {
        let mut best: Option<(f32, usize)> = None;
        let mut cell: Option<(f32, usize)> = None;
        let edge = |slot: &mut Option<(f32, usize)>, distance: f32, byte: usize| {
            if slot.is_none_or(|(d, _)| distance < d) {
                *slot = Some((distance, byte));
            }
        };
        for c in &self.clusters {
            edge(&mut best, (c.leading() - x).abs(), c.start);
            let outside = (c.x0 - x).max(x - c.x1).max(0.0);
            edge(&mut cell, outside, c.start);
        }
        if let Some(last) = self.clusters.last() {
            edge(&mut best, (last.trailing() - x).abs(), self.end);
        }
        (
            best.map_or(self.start, |b| b.1),
            cell.map_or(self.start, |c| c.1),
        )
    }

    /// Merged horizontal spans covering the clusters inside `range`. Bidi text can
    /// produce several disjoint spans for one logical range.
    pub fn spans(&self, range: Range<usize>) -> Vec<(f32, f32)> {
        let mut spans: Vec<(f32, f32)> = self
            .clusters
            .iter()
            .filter(|c| c.start >= range.start && c.end <= range.end)
            .map(|c| (c.x0, c.x1))
            .collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f32, f32)> = Vec::with_capacity(spans.len().min(4));
        for span in spans {
            match merged.last_mut() {
                Some(last) if span.0 <= last.1 + 0.5 => last.1 = last.1.max(span.1),
                _ => merged.push(span),
            }
        }
        merged
    }
}

struct Group {
    start: usize,
    end: usize,
    left: f32,
    right: f32,
    rtl: bool,
}

fn groups(glyphs: &[LayoutGlyph]) -> Vec<Group> {
    let mut order: Vec<&LayoutGlyph> = glyphs.iter().collect();
    order.sort_by_key(|g| (g.start, g.end));
    let mut groups: Vec<Group> = Vec::new();
    for g in order {
        match groups.last_mut() {
            Some(last) if last.start == g.start => {
                last.end = last.end.max(g.end);
                last.left = last.left.min(g.x);
                last.right = last.right.max(g.x + g.w);
            }
            _ => groups.push(Group {
                start: g.start,
                end: g.end,
                left: g.x,
                right: g.x + g.w,
                rtl: g.level.is_rtl(),
            }),
        }
    }
    groups
}

/// Grapheme extents of one cosmic-text run. A shaped cluster may be a ligature or
/// several glyphs; every grapheme boundary gets an edge, never one inside a grapheme.
pub(super) fn build_line(run: &LayoutRun<'_>, glyphs: &[LayoutGlyph]) -> VisualLine {
    let groups = groups(glyphs);
    let start = groups.first().map_or(0, |g| g.start);
    let end = groups.iter().map(|g| g.end).max().unwrap_or(start);
    let mut clusters = Vec::new();
    let mut next = 0;
    let mut edge = 0.0_f32;
    for (offset, grapheme) in run.text[start..end].grapheme_indices(true) {
        let (b, e) = (start + offset, start + offset + grapheme.len());
        while next < groups.len() && groups[next].end <= b {
            next += 1;
        }
        let (mut left, mut right, mut rtl, mut found) =
            (f32::INFINITY, f32::NEG_INFINITY, run.rtl, false);
        let mut j = next;
        while j < groups.len() && groups[j].start < e {
            let g = &groups[j];
            rtl = g.rtl;
            found = true;
            if g.start <= b && g.end >= e && g.end - g.start > e - b {
                let count = run.text[g.start..g.end].graphemes(true).count().max(1) as f32;
                let before = run.text[g.start..b].graphemes(true).count() as f32;
                let step = (g.right - g.left) / count;
                (left, right) = if g.rtl {
                    (g.right - step * (before + 1.0), g.right - step * before)
                } else {
                    (g.left + step * before, g.left + step * (before + 1.0))
                };
                break;
            }
            left = left.min(g.left);
            right = right.max(g.right);
            j += 1;
        }
        if !found {
            (left, right) = (edge, edge);
        }
        edge = if rtl { left } else { right };
        clusters.push(Cluster {
            start: b,
            end: e,
            x0: left,
            x1: right,
            rtl,
        });
    }
    VisualLine {
        top: run.line_top,
        height: run.line_height,
        start,
        end,
        clusters,
    }
}

/// Caret list of a whole layout (one entry per grapheme boundary), derived from its lines.
pub(super) fn carets(lines: &[VisualLine]) -> Vec<(usize, f32)> {
    let mut carets = Vec::new();
    for line in lines {
        carets.extend(line.clusters.iter().map(|c| (c.start, c.leading())));
        carets.push((
            line.end,
            line.clusters.last().map_or(0.0, Cluster::trailing),
        ));
    }
    carets.sort_by_key(|p| p.0);
    carets.dedup_by_key(|p| p.0);
    carets
}

pub(super) fn empty_line(height: f32) -> VisualLine {
    VisualLine {
        top: 0.0,
        height,
        start: 0,
        end: 0,
        clusters: Vec::new(),
    }
}
