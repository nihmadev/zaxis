//! The text model assistive technology navigates: one run per visual line, built from the
//! same shaped clusters that painting, hit testing and carets read.
//!
//! A "character" is a grapheme cluster, exactly the unit the caret of
//! [`TextEdit`](crate::TextEdit) moves by, so a position handed to a screen reader and a
//! position of the edit buffer are the same boundary.

use crate::{text::VisualLine, Rect, Vec2};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

/// Longest run in characters. Word starts are indices of one byte, so a longer visual line
/// is published as several runs linked on the same line.
const MAX_CHARACTERS: usize = 240;
/// Longest character in bytes that one length entry can hold.
const MAX_LENGTH: usize = u8::MAX as usize;

/// One visual line (or a part of a very long one).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextRun {
    /// The bytes of the node's text this run covers, including a hard line break.
    pub text: String,
    /// Byte offset of `text` in the node's text.
    pub start: usize,
    /// Bounds relative to the top left corner of the node, in logical pixels.
    pub rect: Rect,
    /// Byte length of each character.
    pub lengths: Vec<u8>,
    /// Distance of each character from the leading edge of the run, and its advance.
    pub positions: Vec<f32>,
    pub widths: Vec<f32>,
    /// Character indices at which a word starts.
    pub word_starts: Vec<u8>,
    pub rtl: bool,
    /// The run continues the visual line of the previous run.
    pub continues: bool,
}

impl TextRun {
    /// Character index of byte `offset` (relative to the run): the number of characters
    /// that end at or before it.
    pub fn character_at(&self, offset: usize) -> usize {
        let mut end = 0;
        self.lengths
            .iter()
            .take_while(|len| {
                end += usize::from(**len);
                end <= offset
            })
            .count()
    }

    /// Byte offset (relative to the run) of character `index`, clamped to the run.
    pub fn offset_of(&self, index: usize) -> usize {
        self.lengths
            .iter()
            .take(index)
            .map(|len| usize::from(*len))
            .sum()
    }
}

/// The visual lines of one text, top to bottom. Runs cover the text without gaps.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TextModel {
    pub runs: Vec<TextRun>,
}

struct Character {
    start: usize,
    end: usize,
    x0: f32,
    x1: f32,
    word: bool,
}

fn finite(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

/// `at` moved down to a character boundary of `text`.
fn floor_boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Byte offsets at which a word starts. Whitespace belongs to the word before it, the
/// convention screen readers expect; `Ctrl+arrows` in the editor stop at the same starts.
fn word_starts(paragraph: &str) -> Vec<usize> {
    paragraph
        .split_word_bound_indices()
        .filter(|(_, word)| !word.chars().all(char::is_whitespace))
        .map(|(start, _)| start)
        .collect()
}

impl TextModel {
    /// Append the visual lines of one shaped paragraph.
    ///
    /// `paragraph` is the text that was shaped, without its line break; `base` is its byte
    /// offset in the node's text and `origin` the paragraph's top left corner relative to
    /// the node. `terminator` is the line break that follows (`"\n"`, or empty at the end).
    pub fn push_lines(
        &mut self,
        paragraph: &str,
        base: usize,
        lines: &[VisualLine],
        origin: Vec2,
        terminator: &str,
    ) {
        let words = word_starts(paragraph);
        let mut at = 0;
        for (k, line) in lines.iter().enumerate() {
            let last = k + 1 == lines.len();
            // Whitespace a soft wrap swallowed has no cluster; it stays with the line it
            // ends, so the runs cover the paragraph without gaps.
            let end = match lines.get(k + 1) {
                Some(next) => floor_boundary(paragraph, next.start).max(at),
                None => paragraph.len(),
            };
            let rtl = line.clusters.first().is_some_and(|c| c.rtl);
            let (left, right) = line
                .clusters
                .iter()
                .fold(None, |edges: Option<(f32, f32)>, c| {
                    let (x0, x1) = (finite(c.x0), finite(c.x1));
                    Some(edges.map_or((x0, x1), |(l, r)| (l.min(x0), r.max(x1))))
                })
                .unwrap_or((0.0, 0.0));
            let mut edge = if rtl { right } else { left };
            let mut characters = Vec::new();
            let mut cluster = 0;
            for (offset, grapheme) in paragraph[at..end].grapheme_indices(true) {
                let start = at + offset;
                while line.clusters.get(cluster).is_some_and(|c| c.start < start) {
                    cluster += 1;
                }
                let (x0, x1) = line
                    .clusters
                    .get(cluster)
                    .filter(|c| c.start == start)
                    .map_or((edge, edge), |c| (finite(c.x0), finite(c.x1)));
                edge = if rtl { x0 } else { x1 };
                characters.push(Character {
                    start,
                    end: start + grapheme.len(),
                    x0,
                    x1,
                    word: words.binary_search(&start).is_ok(),
                });
            }
            let top = finite(line.top);
            let bottom = finite(lines.get(k + 1).map_or(line.top + line.height, |n| n.top));
            let vertical = (origin.y + top, origin.y + bottom.max(top));
            let hard_break = (last && !terminator.is_empty()).then_some((terminator, edge));
            self.push_line(
                paragraph,
                base,
                &characters,
                at..end,
                rtl,
                origin.x,
                vertical,
                hard_break,
            );
            at = end;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_line(
        &mut self,
        paragraph: &str,
        base: usize,
        characters: &[Character],
        range: std::ops::Range<usize>,
        rtl: bool,
        x: f32,
        (top, bottom): (f32, f32),
        hard_break: Option<(&str, f32)>,
    ) {
        let chunks = characters.chunks(MAX_CHARACTERS).count().max(1);
        for n in 0..chunks {
            let chunk = characters.chunks(MAX_CHARACTERS).nth(n).unwrap_or(&[]);
            let start = chunk.first().map_or(range.start, |c| c.start);
            let end = chunk.last().map_or(range.end, |c| c.end);
            let (left, right) = chunk
                .iter()
                .fold(None, |edges: Option<(f32, f32)>, c| {
                    Some(edges.map_or((c.x0, c.x1), |(l, r)| (l.min(c.x0), r.max(c.x1))))
                })
                .unwrap_or_else(|| hard_break.map_or((0.0, 0.0), |(_, edge)| (edge, edge)));
            let mut run = TextRun {
                text: paragraph[start..end].to_owned(),
                start: base + start,
                rect: Rect::from_min_max(Vec2::new(x + left, top), Vec2::new(x + right, bottom)),
                lengths: Vec::with_capacity(chunk.len() + 1),
                positions: Vec::with_capacity(chunk.len() + 1),
                widths: Vec::with_capacity(chunk.len() + 1),
                word_starts: Vec::new(),
                rtl,
                continues: n > 0,
            };
            for c in chunk {
                let position = if rtl { right - c.x1 } else { c.x0 - left };
                if c.word && run.lengths.len() <= usize::from(u8::MAX) {
                    run.word_starts.push(run.lengths.len() as u8);
                }
                // A cluster longer than a length entry is published in pieces that share
                // its position; only the first one has a width.
                let mut piece = c.start;
                while piece < c.end {
                    let next = floor_boundary(paragraph, (piece + MAX_LENGTH).min(c.end));
                    let next = if next > piece { next } else { c.end };
                    run.lengths.push((next - piece).min(MAX_LENGTH) as u8);
                    run.positions.push(position.max(0.0));
                    run.widths.push(if piece == c.start {
                        (c.x1 - c.x0).max(0.0)
                    } else {
                        0.0
                    });
                    piece = next;
                }
            }
            if let Some((terminator, edge)) = hard_break.filter(|_| n + 1 == chunks) {
                run.text.push_str(terminator);
                run.lengths.push(terminator.len().min(MAX_LENGTH) as u8);
                run.positions
                    .push(if rtl { right - edge } else { edge - left }.max(0.0));
                run.widths.push(0.0);
            }
            self.runs.push(run);
        }
    }

    /// Append every visual line of a text shaped as a whole (static labels): `lines` come
    /// from one layout of `text`, whose line ranges restart at each paragraph.
    pub fn push_text(&mut self, text: &str, lines: &[VisualLine], origin: Vec2) {
        let mut base = 0;
        let mut first = 0;
        let mut paragraphs = text.split('\n').peekable();
        while let Some(piece) = paragraphs.next() {
            let paragraph = piece.strip_suffix('\r').unwrap_or(piece);
            // The lines of a paragraph are those up to the next restart of the byte range.
            let mut count = 1.min(lines.len() - first.min(lines.len()));
            while lines
                .get(first + count)
                .is_some_and(|line| line.start > lines[first + count - 1].start)
            {
                count += 1;
            }
            let more = paragraphs.peek().is_some();
            let terminator = if more {
                &text[base + paragraph.len()..base + piece.len() + 1]
            } else {
                ""
            };
            let slice = lines.get(first..first + count).unwrap_or(&[]);
            if slice.is_empty() {
                self.runs
                    .push(TextRun::unshaped(paragraph, terminator, base, origin));
            } else {
                self.push_lines(paragraph, base, slice, origin, terminator);
            }
            first += count;
            base += piece.len() + 1;
        }
    }

    /// The run holding byte `offset` of the node's text and the character index in it.
    /// An offset on a line boundary belongs to the line it starts.
    pub fn position(&self, offset: usize) -> Option<(usize, usize)> {
        let run = self
            .runs
            .iter()
            .rposition(|run| run.start <= offset)
            .or((!self.runs.is_empty()).then_some(0))?;
        let inside = offset.saturating_sub(self.runs[run].start);
        Some((run, self.runs[run].character_at(inside)))
    }

    /// Byte offset in the node's text of character `index` of run `run`.
    pub fn offset(&self, run: usize, index: usize) -> Option<usize> {
        let run = self.runs.get(run)?;
        Some(run.start + run.offset_of(index))
    }
}

impl TextRun {
    /// A line that has no layout: characters without positions.
    fn unshaped(paragraph: &str, terminator: &str, base: usize, origin: Vec2) -> Self {
        let mut run = TextRun {
            text: format!("{paragraph}{terminator}"),
            start: base,
            rect: Rect::from_min_size(origin, Vec2::ZERO),
            lengths: Vec::new(),
            positions: Vec::new(),
            widths: Vec::new(),
            word_starts: Vec::new(),
            rtl: false,
            continues: false,
        };
        for character in run.text.chars() {
            run.lengths.push(character.len_utf8() as u8);
        }
        run.positions = vec![0.0; run.lengths.len()];
        run.widths = vec![0.0; run.lengths.len()];
        run
    }
}

/// A shared text model. Equal when it is the same allocation, or the same content.
#[derive(Clone, Debug)]
pub(crate) struct TextRef(pub Arc<TextModel>);

impl PartialEq for TextRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

impl From<TextModel> for TextRef {
    fn from(model: TextModel) -> Self {
        Self(Arc::new(model))
    }
}
