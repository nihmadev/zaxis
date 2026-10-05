//! Shaping and measuring of a block: wraps the document of the retained state, applies the
//! ellipsis, and turns spans into the styled runs the shaper and painter read.

use super::{fit::cut_point, links::LinkRun, options::Look, state::BlockState, Block};
use crate::{
    components::{rich_text::Span, text_edit::doc::Env},
    context::Context,
    text::StyleRun,
    Color, Id, Vec2,
};
use std::sync::Arc;

/// Styled runs for the shown text. `colors` holds the color of each link, by link index;
/// without it links shape like the text around them (colors never change glyph positions).
/// `None` when no span changes anything, so plain text keeps the plain layout path.
pub(crate) fn runs(
    spans: &[Span],
    state: &BlockState,
    colors: Option<&[LinkRun]>,
) -> Option<Arc<[StyleRun]>> {
    let end = state.shown().len();
    let mut out = Vec::with_capacity(spans.len());
    let mut link = 0;
    for span in spans {
        let color = match &span.link {
            Some(_) => {
                link += 1;
                colors.and_then(|c| c.get(link - 1)).map(|l| l.color)
            }
            None => span.style.color,
        };
        let range = state.to_display(span.range.clone());
        let range = range.start.min(end)..range.end.min(end);
        let run = StyleRun {
            start: range.start,
            end: range.end,
            weight: span.style.weight,
            monospace: span.style.monospace,
            color: color.filter(|c| *c != Color::TRANSPARENT),
        };
        if range.start < range.end && state.cut.is_none_or(|cut| span.range.start < cut) {
            if out
                .last()
                .is_some_and(|last: &StyleRun| last.end > run.start)
            {
                continue;
            }
            out.push(run);
        }
    }
    let plain = out
        .iter()
        .all(|r| r.weight.is_none() && !r.monospace && r.color.is_none());
    (!out.is_empty() && !plain).then(|| StyleRun::cover(&out, end).into())
}

/// Bring the document up to date with the text, the typography and the available width, fit
/// the ellipsis, and measure. Returns the size of the shown text.
pub(crate) fn prepare(
    ctx: &mut Context,
    state: &mut BlockState,
    block: &Block,
    look: &Look,
    wrap: f32,
    avail: f32,
) -> Vec2 {
    let env = Env {
        size: look.size,
        font: look.font,
        wrap,
        tab: look.tab,
        lh: look.size * 1.25,
    };
    if state.set_text(&block.source) {
        state.display = None;
        state.cut = None;
        state.built = None;
        state.doc.reset(&block.source);
    }
    state.doc.set_env(env);
    let key = Id::new((
        state.rev,
        wrap.to_bits(),
        look.size.to_bits(),
        look.font,
        look.tab,
    ));
    if state.built != Some(key) {
        if state.display.is_some() {
            state.display = None;
            state.cut = None;
            state.doc.reset(&state.text);
        }
        state.doc.set_runs(runs(&block.spans, state, None));
        if let Some(lines) = block.text.max_lines {
            let cut = cut_point(
                ctx,
                &mut state.doc,
                &state.text,
                lines,
                avail,
                block.text.wrap,
                look.size,
                look.font,
            );
            if cut.is_some() {
                state.set_cut(cut);
                let shown = state.shown().to_owned();
                state.doc.reset(&shown);
            }
        }
        state.built = Some(key);
    }
    let runs = runs(&block.spans, state, None);
    state.doc.set_runs(runs);
    let BlockState {
        doc, text, display, ..
    } = state;
    let shown: &str = display.as_deref().unwrap_or(text);
    doc.measure_range(ctx, shown, 0, f32::INFINITY);
    let size = Vec2::new(doc.width(), doc.height());
    state.size = Vec2::new(
        if size.x.is_finite() { size.x } else { 0.0 },
        if size.y.is_finite() {
            size.y
        } else {
            look.size * 1.25
        },
    );
    state.size
}
