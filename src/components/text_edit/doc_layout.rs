//! Position queries over a [`Doc`]: caret location, hit testing, vertical motion and
//! selection rectangles. All of them read the cached paragraph layouts that painting uses.
use super::doc::Doc;
use crate::components::text_geometry::{hit, paragraph_rects};
use crate::{
    components::edit_buffer::{Pos, Surface},
    context::Context,
    Rect, Vec2,
};
use std::ops::Range;

/// Where a position sits: paragraph, visual line, and content-space caret coordinates.
pub(crate) struct Loc {
    pub para: usize,
    pub line: usize,
    pub x: f32,
    pub y: f32,
}

impl Doc {
    pub fn locate(&mut self, ctx: &mut Context, text: &str, pos: Pos) -> Loc {
        let para = self.para_at(pos.byte.min(text.len()));
        let rel = pos
            .byte
            .saturating_sub(self.paras[para].start)
            .min(self.paras[para].len);
        self.locate_in(ctx, text, para, rel, pos.upstream)
    }

    /// Like [`Self::locate`] for a byte offset inside a paragraph's *shown* text, which
    /// is longer than the stored paragraph while a composition is displayed.
    pub fn locate_in(
        &mut self,
        ctx: &mut Context,
        text: &str,
        para: usize,
        rel: usize,
        upstream: bool,
    ) -> Loc {
        let layout = self.layout(ctx, text, para);
        let line = Doc::line_of(&layout.lines, rel, upstream && rel > 0);
        let l = &layout.lines[line];
        Loc {
            para,
            line,
            x: l.caret_x(rel).unwrap_or(0.0),
            y: self.top(para) + l.top,
        }
    }

    /// Position nearest to a content-space point, and the start of the grapheme under it.
    pub fn hit_point(&mut self, ctx: &mut Context, text: &str, point: Vec2) -> (Pos, usize) {
        let para = self.para_at_y(point.y.max(0.0));
        let layout = self.layout(ctx, text, para);
        let rel_y = point.y - self.top(para);
        let last = layout.lines.len() - 1;
        let h = hit(&layout, Vec2::new(point.x, rel_y));
        let start = self.paras[para].start;
        (
            Pos {
                byte: start + h.byte,
                upstream: h.line < last && h.byte == layout.lines[h.line].end,
            },
            start + h.cell,
        )
    }

    /// Move `delta` visual lines keeping column `x`. Past the first or last line the
    /// caret goes to the start or end of the document.
    pub fn move_lines(
        &mut self,
        ctx: &mut Context,
        text: &str,
        pos: Pos,
        column: Option<f32>,
        delta: i32,
    ) -> Option<(Pos, f32)> {
        let here = self.locate(ctx, text, pos);
        let x = column.unwrap_or(here.x);
        let (mut para, mut line) = (here.para, here.line as i64 + i64::from(delta));
        loop {
            if line < 0 {
                if para == 0 {
                    return Some((Pos::default(), x));
                }
                para -= 1;
                line += self.layout(ctx, text, para).lines.len() as i64;
                continue;
            }
            let layout = self.layout(ctx, text, para);
            let count = layout.lines.len() as i64;
            if line < count {
                let l = &layout.lines[line as usize];
                let (byte, _) = l.hit(x);
                let upstream = line + 1 < count && byte == l.end;
                let byte = self.paras[para].start + byte;
                return Some((Pos { byte, upstream }, x));
            }
            if para + 1 >= self.paras.len() {
                return Some((
                    Pos {
                        byte: text.len(),
                        upstream: false,
                    },
                    x,
                ));
            }
            line -= count;
            para += 1;
        }
    }

    /// Byte range of the visual line holding `pos` and whether it ends at a soft wrap.
    pub fn line_bounds(&mut self, ctx: &mut Context, text: &str, pos: Pos) -> (Range<usize>, bool) {
        let loc = self.locate(ctx, text, pos);
        let layout = self.layout(ctx, text, loc.para);
        let p = &self.paras[loc.para];
        let l = &layout.lines[loc.line];
        let last = loc.line + 1 == layout.lines.len();
        let start = p.start + if loc.line == 0 { 0 } else { l.start };
        let end = p.start + if last { p.len } else { l.end };
        (start..end, !last)
    }

    /// Rectangles (content space) covering `range` on paragraphs `first..=last`. Bidi text
    /// gives several rectangles per line; a selected line break adds a `slab`-wide stub.
    pub fn selection_rects(
        &mut self,
        ctx: &mut Context,
        text: &str,
        range: Range<usize>,
        paras: Range<usize>,
        slab: f32,
    ) -> Vec<Rect> {
        let mut rects = Vec::new();
        for i in paras.start..paras.end.min(self.paras.len()) {
            let (start, len, next) = {
                let p = &self.paras[i];
                (p.start, p.len, p.next_start())
            };
            if range.is_empty() || range.start >= next || range.end <= start {
                continue;
            }
            let layout = self.layout(ctx, text, i);
            let top = self.top(i);
            paragraph_rects(
                &layout,
                range.start.saturating_sub(start)..range.end - start,
                len,
                slab,
                top,
                &mut rects,
            );
        }
        rects
    }
}

/// [`Surface`] of a multi-line field.
pub(crate) struct AreaSurface<'a> {
    pub doc: &'a mut Doc,
    pub ctx: &'a mut Context,
    pub page: i32,
}

impl Surface for AreaSurface<'_> {
    fn line(&mut self, text: &str, pos: Pos) -> (Range<usize>, bool) {
        self.doc.line_bounds(self.ctx, text, pos)
    }
    fn vertical(
        &mut self,
        text: &str,
        pos: Pos,
        column: Option<f32>,
        lines: i32,
    ) -> Option<(Pos, f32)> {
        self.doc.move_lines(self.ctx, text, pos, column, lines)
    }
    fn hit(&mut self, text: &str, point: Vec2) -> (Pos, usize) {
        self.doc.hit_point(self.ctx, text, point)
    }
    fn page_lines(&self) -> i32 {
        self.page
    }
}
