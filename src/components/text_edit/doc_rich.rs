//! Styled runs of a [`Doc`]: static text with mixed weights, families and colors keeps one
//! document over the whole string and slices the runs per paragraph.
use super::doc::Doc;
use crate::{
    context::{Context, Paint},
    text::StyleRun,
    Color, Vec2,
};
use std::sync::Arc;

impl Doc {
    /// Use styled `runs` (sorted, covering the text) for shaping; `None` for plain text.
    /// Cached layouts are dropped only when the runs actually differ.
    pub fn set_runs(&mut self, runs: Option<Arc<[StyleRun]>>) {
        if self.runs == runs {
            return;
        }
        let reshape = match (&self.runs, &runs) {
            (Some(old), Some(new)) => !crate::text::same_shape(old, new),
            _ => true,
        };
        self.runs = runs;
        for p in &mut self.paras {
            p.rich = None;
            if reshape {
                p.measured = false;
                p.layout = None;
            }
        }
        self.widest = None;
    }

    /// The paint primitive of paragraph `i` at `position`: styled when the document has runs.
    /// The layout is the one position queries read, so what is painted is what is hit.
    pub fn paint(
        &mut self,
        ctx: &mut Context,
        text: &str,
        i: usize,
        position: Vec2,
        color: Color,
    ) -> Paint {
        let env = self.env;
        let shown = self.text_of(text, i).to_owned();
        self.layout(ctx, text, i);
        if let (Some(runs), None) = (&self.runs, &self.paras[i].rich) {
            self.paras[i].rich = Some(slice_runs(runs, self.paras[i].start, self.paras[i].len));
        }
        match self.paras[i].rich.clone() {
            Some(runs) => Paint::Rich {
                text: shown,
                runs,
                position,
                size: env.size,
                font: env.font,
                wrap_width: env.wrap,
                tab: env.tab,
                color,
            },
            None => Paint::Paragraph {
                text: shown,
                position,
                size: env.size,
                font: env.font,
                wrap_width: env.wrap,
                tab: env.tab,
                color,
            },
        }
    }
}

/// The part of `runs` inside `start..start + len`, rebased to `start` and covering that range.
pub(super) fn slice_runs(runs: &[StyleRun], start: usize, len: usize) -> Arc<[StyleRun]> {
    let end = start + len;
    let first = runs.partition_point(|r| r.end <= start);
    let inside = runs[first..]
        .iter()
        .take_while(|r| r.start < end)
        .map(|r| StyleRun {
            start: r.start.max(start) - start,
            end: r.end.min(end) - start,
            ..*r
        })
        .collect::<Vec<_>>();
    StyleRun::cover(&inside, len).into()
}
