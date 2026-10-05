//! What a block tells assistive technology: its text as the visual lines that are painted,
//! the selection, and one node per link.
//!
//! The lines come from the paragraph layouts the block paints and hit tests with, and the
//! model built from them is kept with the block until one of those layouts is replaced.
//! The node's value is always the whole source text: a block shortened with an ellipsis
//! publishes the lines it shows and the hidden rest as runs without positions.

use super::{links::LinkRun, select::Place, state::BlockState, Block};
use crate::{
    accessibility::text::{TextModel, TextRef, TextRun},
    components::{rich_text::LinkTarget, Ui},
    context::selection::Endpoint,
    text::TextLayout,
    AccessAction, AccessActionKind, AccessRole, Id, Rect, TypographyRole, Vec2,
};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

/// A block of at most this many visual lines publishes every line with positions; the
/// model then never depends on what is scrolled into view.
const FULL_LINES: usize = 512;
/// A longer block positions the paragraphs in view, widened to steps of this many, so
/// scrolling replaces the model once per step and not once per line.
const STEP: usize = 32;
/// Longest run in characters: word starts are indices of one byte.
const MAX_CHARACTERS: usize = 240;

/// The text model of a block and what it was built from.
pub(crate) struct Lines {
    key: Id,
    /// Layouts of the positioned paragraphs. Holding them keeps their addresses unique,
    /// so the same pointers mean the same shaping.
    layouts: Vec<Arc<TextLayout>>,
    model: TextRef,
}

/// Runs for `text` (a paragraph and its line break, at byte `start` of the block's text)
/// that the block did not lay out: characters and words without positions.
fn unplaced(model: &mut TextModel, text: &str, start: usize, rect: Rect, mut continues: bool) {
    let words: Vec<usize> = text
        .split_word_bound_indices()
        .filter(|(_, word)| !word.chars().all(char::is_whitespace))
        .map(|(at, _)| at)
        .collect();
    let graphemes: Vec<(usize, &str)> = text.grapheme_indices(true).collect();
    let mut chunks = graphemes.chunks(MAX_CHARACTERS).peekable();
    let blank = chunks.peek().is_none();
    for chunk in chunks.chain(blank.then_some(&[][..])) {
        let from = chunk.first().map_or(0, |(at, _)| *at);
        let to = chunk
            .last()
            .map_or(from, |(at, grapheme)| at + grapheme.len());
        let mut run = TextRun {
            text: text[from..to].to_owned(),
            start: start + from,
            rect,
            lengths: Vec::with_capacity(chunk.len()),
            positions: Vec::new(),
            widths: Vec::new(),
            word_starts: Vec::new(),
            rtl: false,
            continues,
        };
        for (at, grapheme) in chunk {
            if words.binary_search(at).is_ok() && run.lengths.len() <= usize::from(u8::MAX) {
                run.word_starts.push(run.lengths.len() as u8);
            }
            // A cluster longer than one length entry is published in pieces.
            let mut rest = *grapheme;
            while !rest.is_empty() {
                let mut take = rest.len().min(usize::from(u8::MAX));
                while !rest.is_char_boundary(take) {
                    take -= 1;
                }
                run.lengths.push(take as u8);
                rest = &rest[take..];
            }
        }
        run.positions = vec![0.0; run.lengths.len()];
        run.widths = vec![0.0; run.lengths.len()];
        model.runs.push(run);
        continues = true;
    }
}

/// The visual lines of the block's whole source text, relative to `origin`.
fn model(ui: &mut Ui<'_>, state: &mut BlockState, origin: Vec2) -> TextRef {
    let visible = ui.clip_rect();
    let BlockState {
        doc,
        text,
        display,
        cut,
        built,
        rev,
        access,
        ..
    } = state;
    let shown: &str = display.as_deref().unwrap_or(text);
    let count = doc.paras.len();
    let lines: usize = doc.paras.iter().map(|p| p.lines as usize).sum();
    let window = if lines <= FULL_LINES {
        0..count
    } else {
        let first = doc.para_at_y((visible.min.y - origin.y).max(0.0));
        let last = doc.para_at_y((visible.max.y - origin.y).max(0.0));
        first / STEP * STEP..((last.max(first) / STEP + 1) * STEP).min(count)
    };
    let layouts: Vec<Arc<TextLayout>> = window
        .clone()
        .map(|i| doc.layout(ui.context, shown, i))
        .collect();
    let key = Id::new((*rev, *built, *cut, window.start, window.end));
    let same = |old: &&Lines| {
        old.key == key
            && old.layouts.len() == layouts.len()
            && old
                .layouts
                .iter()
                .zip(&layouts)
                .all(|(a, b)| Arc::ptr_eq(a, b))
    };
    if let Some(lines) = access.as_ref().filter(same) {
        return lines.model.clone();
    }

    let line_height = doc.env().lh;
    let mut model = TextModel::default();
    for i in 0..count {
        let top = doc.top(i);
        let p = &doc.paras[i];
        // The last paragraph of a shortened text ends at the cut: the ellipsis is not text.
        let cut_here = cut.filter(|_| i + 1 == count);
        let end = cut_here.map_or(p.end(), |cut| cut.max(p.start));
        let paragraph = &text[p.start..end];
        let terminator = match cut_here {
            Some(_) => "",
            None => &text[p.end()..p.next_start()],
        };
        let placed = i
            .checked_sub(window.start)
            .and_then(|k| layouts.get(k))
            .map(|layout| &layout.lines[..])
            .filter(|lines| !lines.is_empty());
        match placed {
            // Nothing of the paragraph is shown: the hidden rest starts the line.
            _ if cut_here.is_some() && paragraph.is_empty() => {}
            Some(lines) => {
                let keep = match cut_here {
                    Some(_) => lines
                        .iter()
                        .take_while(|l| l.start < paragraph.len())
                        .count(),
                    None => lines.len(),
                };
                let position = Vec2::new(0.0, top);
                model.push_lines(
                    paragraph,
                    p.start,
                    &lines[..keep.max(1)],
                    position,
                    terminator,
                );
            }
            None => {
                let size = Vec2::new(p.width, p.lines as f32 * line_height);
                let rect = Rect::from_min_size(Vec2::new(0.0, top), size);
                let whole = &text[p.start..end + terminator.len()];
                unplaced(&mut model, whole, p.start, rect, false);
            }
        }
    }
    if let Some(cut) = *cut {
        // The hidden rest sits where the ellipsis is and continues the line it cut.
        let edge = model.runs.last().map_or(Rect::default(), |run| {
            Rect::from_min_max(Vec2::new(run.rect.max.x, run.rect.min.y), run.rect.max)
        });
        let mut continues = doc.paras.last().is_some_and(|p| cut > p.start);
        let mut at = cut.min(text.len());
        for piece in text[at..].split_inclusive('\n') {
            unplaced(&mut model, piece, at, edge, continues);
            at += piece.len();
            continues = false;
        }
    }
    let model = TextRef::from(model);
    *access = Some(Lines {
        key,
        layouts,
        model: model.clone(),
    });
    model
}

/// Anchor and focus of the selection inside the item `id`, when it covers part of it.
fn selection(ui: &Ui<'_>, place: Place, len: usize) -> Option<(usize, usize)> {
    let Place { id, scope, ord, .. } = place;
    let selection = ui
        .context
        .selection
        .selection
        .filter(|s| s.scope == scope)?;
    let range = ui.context.selection_range(scope, id, ord, len);
    if range.is_empty() {
        return None;
    }
    Some(if selection.ordered().0 == selection.anchor {
        (range.start, range.end)
    } else {
        (range.end, range.start)
    })
}

/// Apply what assistive technology asked of the block's text, through the selection state
/// a press and a drag change.
pub(crate) fn requests(ui: &mut Ui<'_>, state: &BlockState, place: Place) {
    let Place { id, scope, ord, .. } = place;
    for request in ui.context.take_access_actions(id) {
        if let AccessAction::SetTextSelection { anchor, focus } = request {
            let at = |byte: usize| Endpoint {
                item: id,
                byte: state.clamp(byte),
                ord,
            };
            ui.context.selection_begin(scope, at(anchor));
            ui.context.selection_extend(scope, at(focus));
        }
    }
}

fn link(ui: &mut Ui<'_>, run: &LinkRun, target: &LinkTarget, bounds: Rect, text: &str) {
    ui.a11y(run.id, bounds, AccessRole::Link, |node| {
        node.label(text).disabled(!run.enabled).visited(run.visited);
        if let Some(url) = &target.url {
            node.url(url.as_str());
        }
        if let Some(tip) = &target.tooltip {
            node.description(tip.as_str());
        }
        if run.enabled {
            node.clicks(run.id);
        }
    });
}

/// Describe the block: a label holding its text with one child per link, or, for a link
/// that is the whole block, the link alone.
pub(crate) fn describe(
    ui: &mut Ui<'_>,
    state: &mut BlockState,
    block: &Block,
    place: Place,
    selectable: bool,
    links: &[LinkRun],
) {
    if !ui.context.a11y_on() {
        state.access = None;
        return;
    }
    let Place { id, rect, .. } = place;
    let mut targets = block.spans.iter().filter_map(|span| span.link.as_ref());
    if let ([run], false) = (links, selectable) {
        if let Some(target) = targets.clone().next().filter(|_| run.id == id) {
            return link(ui, run, target, rect, &state.text);
        }
    }
    let lines = model(ui, state, rect.min);
    let selected = selectable
        .then(|| selection(ui, place, state.text.len()))
        .flatten();
    let (role, level) = match block.text.role {
        TypographyRole::Title => (AccessRole::Heading, Some(1)),
        TypographyRole::Heading => (AccessRole::Heading, Some(2)),
        _ => (AccessRole::Label, None),
    };
    let open = ui.a11y_begin(id, role, |node| {
        node.value(state.text.as_str())
            .text(lines, selected)
            .disabled(!block.enabled);
        if let Some(level) = level {
            node.level(level);
        }
        if let Some(tip) = &block.tooltip {
            node.description(tip.as_str());
        }
        if selectable {
            node.action(AccessActionKind::SetTextSelection);
        }
    });
    for run in links {
        let Some(target) = targets.next() else { break };
        // A link the ellipsis hid entirely has no place on screen and takes no input.
        if !run.frags.is_empty() {
            let text = state.text.get(run.range.clone()).unwrap_or_default();
            link(ui, run, target, run.bounds, text);
        }
    }
    ui.a11y_end(open, Some(rect));
}
