//! Paragraph table of a multi-line field. Paragraphs (runs between line breaks) are the
//! unit of shaping, caching and invalidation: an edit replaces only the paragraphs it
//! touches, shaping happens on demand for paragraphs near the viewport, and the heights of
//! everything else are estimates until they are first measured.
use super::{doc_rich::slice_runs, text_input::hash_bytes};
use crate::{
    components::edit_buffer::Delta,
    context::Context,
    text::{StyleRun, TextLayout, VisualLine},
};
use std::sync::Arc;

/// Everything that changes how a paragraph is shaped or wrapped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Env {
    pub size: f32,
    pub font: crate::text::TextFont,
    /// Wrap width, infinite when lines do not wrap.
    pub wrap: f32,
    pub tab: u16,
    pub lh: f32,
}

pub(crate) struct Para {
    pub start: usize,
    /// Bytes of text, excluding the terminator.
    pub len: usize,
    /// Bytes of the line terminator (`\n`, `\r\n`, `\r`) or zero for the last paragraph.
    pub term: u8,
    pub lines: u32,
    pub top: f32,
    pub width: f32,
    /// Content hash when known (zero otherwise); used to keep measurements across resets.
    hash: u64,
    pub(super) measured: bool,
    pub(super) layout: Option<Arc<TextLayout>>,
    /// The runs of this paragraph, rebased to its start, when the document has styled runs.
    pub(super) rich: Option<Arc<[StyleRun]>>,
    used: u64,
}

impl Para {
    pub fn end(&self) -> usize {
        self.start + self.len
    }
    pub fn next_start(&self) -> usize {
        self.start + self.len + usize::from(self.term)
    }
}

/// A composition string shown inside one paragraph; replaced every frame.
struct Overlay {
    para: usize,
    layout: Arc<TextLayout>,
    text: String,
}

pub(crate) struct Doc {
    pub paras: Vec<Para>,
    pub(super) env: Env,
    /// Paragraphs `[..tops]` have valid `top`; the rest need a prefix sum.
    tops: usize,
    pub(super) widest: Option<f32>,
    overlay: Option<Overlay>,
    /// Styled runs over the whole text, sorted and covering it; static text only.
    pub(super) runs: Option<Arc<[StyleRun]>>,
    pub frame: u64,
}

fn find_break(bytes: &[u8], from: usize) -> Option<usize> {
    bytes[from..]
        .iter()
        .position(|b| matches!(b, b'\n' | b'\r'))
        .map(|p| from + p)
}

impl Doc {
    pub fn new(text: &str, env: Env) -> Self {
        let mut doc = Self {
            paras: Vec::new(),
            env,
            tops: 0,
            widest: None,
            overlay: None,
            runs: None,
            frame: 0,
        };
        doc.paras = doc.split(text, 0, true, true);
        doc
    }

    pub fn env(&self) -> Env {
        self.env
    }

    fn estimate(&self, len: usize) -> u32 {
        let wrap = self.env.wrap;
        if !wrap.is_finite() || wrap <= 0.0 {
            return 1;
        }
        ((len as f32 * self.env.size * 0.5 / wrap).ceil() as u32).max(1)
    }

    fn para(&self, base: usize, start: usize, end: usize, term: usize, hash: u64) -> Para {
        Para {
            start: base + start,
            len: end - start,
            term: term as u8,
            lines: self.estimate(end - start),
            top: 0.0,
            width: 0.0,
            hash,
            measured: false,
            layout: None,
            rich: None,
            used: 0,
        }
    }

    /// Split `text` (which starts at byte `base` of the document) into paragraphs.
    /// `last` keeps the trailing remainder even when it is empty.
    fn split(&self, text: &str, base: usize, last: bool, hashed: bool) -> Vec<Para> {
        let bytes = text.as_bytes();
        let hash = |start: usize, end: usize| {
            if hashed {
                hash_bytes(&bytes[start..end]).max(1)
            } else {
                0
            }
        };
        let mut paras = Vec::new();
        let mut start = 0;
        while let Some(end) = find_break(bytes, start) {
            let term = if bytes[end] == b'\r' && bytes.get(end + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            paras.push(self.para(base, start, end, term, hash(start, end)));
            start = end + term;
        }
        if last || start < bytes.len() {
            let end = bytes.len();
            paras.push(self.para(base, start, end, 0, hash(start, end)));
        }
        paras
    }

    /// Change shaping parameters. Wrapping or metrics changes keep line counts as
    /// estimates and drop cached layouts. Returns whether anything changed.
    pub fn set_env(&mut self, env: Env) -> bool {
        if env == self.env {
            return false;
        }
        let rewrap = env.wrap != self.env.wrap
            || env.size != self.env.size
            || env.font != self.env.font
            || env.tab != self.env.tab;
        self.env = env;
        if rewrap {
            for p in &mut self.paras {
                p.measured = false;
                p.layout = None;
                p.rich = None;
            }
            self.overlay = None;
            self.widest = None;
        }
        self.tops = 0;
        true
    }

    /// Reconcile with text changed outside the field. Paragraphs equal to the old ones
    /// at the start and end of the document keep their measurements.
    pub fn reset(&mut self, text: &str) {
        self.overlay = None;
        let mut fresh = self.split(text, 0, true, true);
        let mut old = std::mem::take(&mut self.paras);
        let same = |a: &Para, b: &Para| {
            a.hash != 0 && a.hash == b.hash && a.len == b.len && a.term == b.term
        };
        let limit = old.len().min(fresh.len());
        let prefix = (0..limit).take_while(|&k| same(&old[k], &fresh[k])).count();
        let (no, nf) = (old.len(), fresh.len());
        let suffix = (0..limit - prefix)
            .take_while(|&k| same(&old[no - 1 - k], &fresh[nf - 1 - k]))
            .count();
        let pairs = (0..prefix)
            .map(|k| (k, k))
            .chain((0..suffix).map(|k| (no - 1 - k, nf - 1 - k)));
        for (o, f) in pairs {
            let (src, dst) = (&mut old[o], &mut fresh[f]);
            dst.lines = src.lines;
            dst.width = src.width;
            dst.measured = src.measured;
            dst.layout = src.layout.take();
            dst.rich = src.rich.take();
            dst.used = src.used;
        }
        self.paras = fresh;
        self.tops = 0;
        self.widest = None;
    }
    /// Update the table after `text` replaced `old_len` bytes at `at` with `new_len` bytes.
    pub fn apply(&mut self, text: &str, delta: Delta) {
        let Delta {
            at,
            old_len,
            new_len,
        } = delta;
        self.overlay = None;
        if self.paras.is_empty() || at + old_len > self.paras.last().map_or(0, Para::next_start) {
            return self.reset(text);
        }
        let mut first = self.para_at(at);
        // A new line break can fuse with a `\r` that ended the previous paragraph.
        if at == self.paras[first].start && first > 0 {
            first -= 1;
        }
        let last = self.para_at(at + old_len);
        let region = self.paras[first].start;
        let old_end = self.paras[last].next_start();
        let new_end = old_end + new_len - old_len;
        if new_end > text.len() || !text.is_char_boundary(new_end) {
            return self.reset(text);
        }
        let keeps_tail = last + 1 == self.paras.len();
        let mut fresh = self.split(&text[region..new_end], region, keeps_tail, false);
        if first == last && fresh.len() == 1 {
            fresh[0].lines = self.paras[first].lines;
            fresh[0].width = self.paras[first].width;
        }
        let count = fresh.len();
        self.paras.splice(first..=last, fresh);
        for p in &mut self.paras[first + count..] {
            p.start = p.start + new_len - old_len;
        }
        self.tops = self.tops.min(first);
        self.widest = None;
    }

    pub fn para_at(&self, byte: usize) -> usize {
        self.paras
            .partition_point(|p| p.start <= byte)
            .saturating_sub(1)
    }

    fn fill_tops(&mut self) {
        let lh = self.env.lh;
        let mut y = match self.tops {
            0 => 0.0,
            n => self.paras[n - 1].top + self.paras[n - 1].lines as f32 * lh,
        };
        for p in &mut self.paras[self.tops..] {
            p.top = y;
            y += p.lines as f32 * lh;
        }
        self.tops = self.paras.len();
    }

    /// Total content height; lines of unmeasured paragraphs are estimates.
    pub fn height(&mut self) -> f32 {
        self.fill_tops();
        self.paras
            .last()
            .map_or(self.env.lh, |p| p.top + p.lines as f32 * self.env.lh)
    }

    /// The widest measured line; paragraphs that were never shown do not count.
    pub fn width(&mut self) -> f32 {
        *self
            .widest
            .get_or_insert_with(|| self.paras.iter().map(|p| p.width).fold(0.0, f32::max))
    }

    pub fn top(&mut self, i: usize) -> f32 {
        self.fill_tops();
        self.paras[i].top
    }

    pub fn para_at_y(&mut self, y: f32) -> usize {
        self.fill_tops();
        self.paras.partition_point(|p| p.top <= y).saturating_sub(1)
    }

    pub fn text_of<'t>(&self, text: &'t str, i: usize) -> &'t str {
        let p = &self.paras[i];
        &text[p.start..p.end()]
    }

    /// The shaped layout of paragraph `i`, built on first use and reused until the
    /// paragraph or the wrap changes. A composition overlay takes precedence.
    pub fn layout(&mut self, ctx: &mut Context, text: &str, i: usize) -> Arc<TextLayout> {
        if let Some(o) = self.overlay.as_ref().filter(|o| o.para == i) {
            return Arc::clone(&o.layout);
        }
        self.paras[i].used = self.frame;
        if let Some(layout) = &self.paras[i].layout {
            return Arc::clone(layout);
        }
        let env = self.env;
        let source = self.text_of(text, i);
        let layout = match self.runs.as_ref() {
            Some(runs) => {
                let own = slice_runs(runs, self.paras[i].start, self.paras[i].len);
                let layout =
                    ctx.rich_paragraph_layout(source, &own, env.size, env.font, env.wrap, env.tab);
                self.paras[i].rich = Some(own);
                layout
            }
            None => ctx.paragraph_layout(source, env.size, env.font, env.wrap, env.tab),
        };
        self.adopt(i, &layout);
        layout
    }

    fn adopt(&mut self, i: usize, layout: &Arc<TextLayout>) {
        let lines = layout.lines.len().max(1) as u32;
        let p = &mut self.paras[i];
        if p.lines != lines {
            p.lines = lines;
            self.tops = self.tops.min(i + 1);
        }
        p.width = layout.size.x;
        p.measured = true;
        p.layout = Some(Arc::clone(layout));
        self.widest = None;
    }

    /// Show `preedit` at `rel` bytes into paragraph `i` for this frame.
    pub fn compose(&mut self, ctx: &mut Context, text: &str, i: usize, rel: usize, preedit: &str) {
        let mut shown = self.text_of(text, i).to_owned();
        shown.insert_str(rel.min(shown.len()), preedit);
        let env = self.env;
        let layout = ctx.paragraph_layout(&shown, env.size, env.font, env.wrap, env.tab);
        self.adopt(i, &layout);
        self.paras[i].layout = None;
        self.paras[i].measured = false;
        self.overlay = Some(Overlay {
            para: i,
            layout,
            text: shown,
        });
    }

    /// The text painted for paragraph `i`: the composition overlay when it is there.
    pub fn shown<'a>(&'a self, text: &'a str, i: usize) -> &'a str {
        match &self.overlay {
            Some(o) if o.para == i => &o.text,
            _ => self.text_of(text, i),
        }
    }

    pub fn clear_overlay(&mut self) {
        if let Some(o) = self.overlay.take() {
            let last = self.paras.len() - 1;
            self.paras[o.para.min(last)].measured = false;
        }
    }

    /// Drop layouts that were not used recently so memory follows the viewport.
    pub fn trim(&mut self) {
        let frame = self.frame;
        let held = self.paras.iter().filter(|p| p.layout.is_some()).count();
        if held > 256 {
            for p in &mut self.paras {
                if p.used + 1 < frame {
                    p.layout = None;
                }
            }
        }
    }

    /// Shape paragraphs from `first` until their tops pass `bottom`; returns the last index.
    pub fn measure_range(
        &mut self,
        ctx: &mut Context,
        text: &str,
        first: usize,
        bottom: f32,
    ) -> usize {
        let mut y = self.top(first);
        let mut i = first;
        loop {
            let lines = self.layout(ctx, text, i).lines.len().max(1);
            y += lines as f32 * self.env.lh;
            if y >= bottom || i + 1 >= self.paras.len() {
                return i;
            }
            i += 1;
        }
    }

    pub fn line_of(lines: &[VisualLine], rel: usize, upstream: bool) -> usize {
        let mut index = lines.partition_point(|l| l.start <= rel).saturating_sub(1);
        if upstream && index > 0 && lines[index].start == rel {
            index -= 1;
        }
        index
    }
}
