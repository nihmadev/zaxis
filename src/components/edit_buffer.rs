//! Shared byte-indexed editing operations. Every persistent endpoint is a grapheme boundary.
//!
//! One buffer serves single- and multi-line fields. Anything that depends on how text is
//! wrapped (visual lines, vertical motion, hit testing) goes through [`Surface`]; all other
//! logic works on a paragraph-local window of the text, never the whole document.
use crate::Vec2;
use std::ops::Range;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
use winit::keyboard::{KeyCode, ModifiersState};

/// A text position plus which side of a soft wrap the caret is drawn on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pos {
    pub byte: usize,
    /// At a soft wrap the byte ends one visual line and starts the next; upstream draws
    /// the caret at the end of the earlier line.
    pub upstream: bool,
}

/// Wrap-dependent geometry, supplied by the field's layout.
pub trait Surface {
    /// Logical byte range of the visual line holding `pos`, and whether it ends at a soft wrap.
    fn line(&mut self, text: &str, pos: Pos) -> (Range<usize>, bool);
    /// Move `lines` visual lines keeping the desired column. `None`: not supported.
    fn vertical(
        &mut self,
        text: &str,
        pos: Pos,
        column: Option<f32>,
        lines: i32,
    ) -> Option<(Pos, f32)>;
    /// Position under a point in content coordinates, and the start of the grapheme there.
    fn hit(&mut self, text: &str, point: Vec2) -> (Pos, usize);
    /// Visual lines per page for PageUp/PageDown.
    fn page_lines(&self) -> i32 {
        1
    }
}

/// Surface of a field with no vertical structure: one line, no vertical motion.
pub(crate) struct OneLine;
impl Surface for OneLine {
    fn line(&mut self, text: &str, _: Pos) -> (Range<usize>, bool) {
        (0..text.len(), false)
    }
    fn vertical(&mut self, _: &str, _: Pos, _: Option<f32>, _: i32) -> Option<(Pos, f32)> {
        None
    }
    fn hit(&mut self, _: &str, _: Vec2) -> (Pos, usize) {
        (Pos::default(), 0)
    }
}

/// A text replacement, enough to undo, redo, and update incremental layout state.
#[derive(Clone, Debug)]
pub struct Change {
    pub at: usize,
    pub removed: String,
    pub inserted: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Delta {
    pub at: usize,
    pub old_len: usize,
    pub new_len: usize,
}

impl Change {
    pub fn delta(&self) -> Delta {
        Delta {
            at: self.at,
            old_len: self.removed.len(),
            new_len: self.inserted.len(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditBuffer {
    pub cursor: usize,
    pub anchor: usize,
    pub upstream: bool,
    /// Desired x kept across consecutive vertical moves; any other motion clears it.
    pub column: Option<f32>,
}

pub(crate) fn boundaries(text: &str) -> Vec<usize> {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect()
}

pub(crate) fn is_boundary(text: &str, i: usize) -> bool {
    if i == 0 || i == text.len() {
        return true;
    }
    i < text.len()
        && text.is_char_boundary(i)
        && GraphemeCursor::new(i, text.len(), true)
            .is_boundary(text, 0)
            .unwrap_or(true)
}

/// The next grapheme boundary strictly after `i`.
pub(crate) fn next_boundary(text: &str, i: usize) -> usize {
    let mut i = i.min(text.len());
    if i == text.len() {
        return i;
    }
    while !text.is_char_boundary(i) {
        i += 1;
    }
    GraphemeCursor::new(i, text.len(), true)
        .next_boundary(text, 0)
        .ok()
        .flatten()
        .unwrap_or(text.len())
}

/// The previous grapheme boundary strictly before `i`.
pub(crate) fn prev_boundary(text: &str, i: usize) -> usize {
    let mut i = i.min(text.len());
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    if i == 0 {
        return 0;
    }
    GraphemeCursor::new(i, text.len(), true)
        .prev_boundary(text, 0)
        .ok()
        .flatten()
        .unwrap_or(0)
}

/// The greatest grapheme boundary at or before `i`.
pub(crate) fn snap_down(text: &str, i: usize) -> usize {
    let mut i = i.min(text.len());
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    if is_boundary(text, i) {
        i
    } else {
        prev_boundary(text, i)
    }
}

/// The least grapheme boundary at or after `i`.
pub(crate) fn snap_up(text: &str, i: usize) -> usize {
    let mut i = i.min(text.len());
    while !text.is_char_boundary(i) {
        i += 1;
    }
    if is_boundary(text, i) {
        i
    } else {
        next_boundary(text, i)
    }
}

const BREAKS: [char; 2] = ['\n', '\r'];

/// Start of the paragraph (line break delimited) containing `i`.
pub(crate) fn para_start(text: &str, i: usize) -> usize {
    text[..i].rfind(BREAKS).map_or(0, |p| p + 1)
}

/// End of the paragraph containing `i`, before its terminator.
pub(crate) fn para_end(text: &str, i: usize) -> usize {
    text[i..].find(BREAKS).map_or(text.len(), |p| i + p)
}

/// Length of the line terminator starting at `i` (`\r\n`, `\n`, `\r`), or zero.
pub(crate) fn terminator_len(text: &str, i: usize) -> usize {
    match text.as_bytes().get(i) {
        Some(b'\r') if text.as_bytes().get(i + 1) == Some(&b'\n') => 2,
        Some(b'\n' | b'\r') => 1,
        _ => 0,
    }
}

/// The paragraph around `i` including its terminator.
pub(crate) fn paragraph_at(text: &str, i: usize) -> Range<usize> {
    let start = para_start(text, i);
    let end = para_end(text, i);
    start..end + terminator_len(text, end)
}

pub(crate) fn word_at(text: &str, cursor: usize) -> Range<usize> {
    let (start, end) = (para_start(text, cursor), para_end(text, cursor));
    let slice = &text[start..end];
    let rel = cursor - start;
    let range = slice
        .split_word_bound_indices()
        .find(|(s, word)| rel < s + word.len())
        .map(|(s, word)| s..s + word.len())
        .unwrap_or_else(|| {
            slice
                .split_word_bound_indices()
                .next_back()
                .map_or(0..0, |(s, word)| s..s + word.len())
        });
    snap_down(text, start + range.start)..snap_up(text, start + range.end)
}

/// One grapheme or word step; words skip whitespace and treat a line break as a stop.
pub(crate) fn step(text: &str, cursor: usize, right: bool, word: bool) -> usize {
    if !word {
        return if right {
            next_boundary(text, cursor)
        } else {
            prev_boundary(text, cursor)
        };
    }
    let (start, end) = (para_start(text, cursor), para_end(text, cursor));
    if right && cursor >= end {
        return (end + terminator_len(text, end)).min(text.len());
    }
    if !right && cursor <= start {
        return match start {
            0 => 0,
            _ if text[..start].ends_with("\r\n") => start - 2,
            _ => start - 1,
        };
    }
    let slice = &text[start..end];
    let rel = cursor - start;
    let mut words = slice
        .split_word_bound_indices()
        .filter(|(_, s)| !s.chars().all(char::is_whitespace));
    if right {
        let point = words
            .find_map(|(i, s)| (i + s.len() > rel).then_some(i + s.len()))
            .unwrap_or(slice.len());
        snap_up(text, start + point)
    } else {
        let point = words
            .rev()
            .find_map(|(i, _)| (i < rel).then_some(i))
            .unwrap_or(0);
        snap_down(text, start + point)
    }
}

impl EditBuffer {
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }
    pub fn pos(&self) -> Pos {
        Pos {
            byte: self.cursor,
            upstream: self.upstream,
        }
    }
    pub fn select_all(&mut self, text: &str) {
        self.anchor = 0;
        self.cursor = text.len();
        self.upstream = false;
        self.column = None;
    }
    pub fn set_cursor(&mut self, cursor: usize, extend: bool) {
        self.cursor = cursor;
        if !extend {
            self.anchor = cursor;
        }
        self.upstream = false;
        self.column = None;
    }
    pub fn set_pos(&mut self, pos: Pos, extend: bool) {
        self.set_cursor(pos.byte, extend);
        self.upstream = pos.upstream;
    }
    pub fn clamp(&mut self, text: &str) {
        self.cursor = snap_down(text, self.cursor);
        self.anchor = snap_down(text, self.anchor);
    }
    /// Replace the selection with `value`, leaving the cursor after it.
    pub fn insert(&mut self, text: &mut String, value: &str) {
        self.replace_selection(text, value);
    }
    /// Like [`Self::insert`], reporting the replacement for history and layout caches.
    pub fn replace_selection(&mut self, text: &mut String, value: &str) -> Option<Change> {
        let range = self.selection();
        let change = (!range.is_empty() || !value.is_empty()).then(|| Change {
            at: range.start,
            removed: text[range.clone()].to_owned(),
            inserted: value.to_owned(),
        });
        let end = range.start + value.len();
        text.replace_range(range, value);
        // Inserting a combining mark or ZWJ can join the following cluster.
        self.set_cursor(snap_up(text, end), false);
        change
    }
    pub fn key(
        &mut self,
        text: &mut String,
        key: KeyCode,
        modifiers: ModifiersState,
        read_only: bool,
    ) {
        self.key_in(text, key, modifiers, read_only, &mut OneLine);
    }
    pub fn key_in(
        &mut self,
        text: &mut String,
        key: KeyCode,
        modifiers: ModifiersState,
        read_only: bool,
        surface: &mut dyn Surface,
    ) -> Option<Change> {
        let command = modifiers.control_key() || modifiers.super_key();
        let shift = modifiers.shift_key();
        match key {
            KeyCode::KeyA if command => self.select_all(text),
            KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                let right = key == KeyCode::ArrowRight;
                let target = if !shift && !command && !self.selection().is_empty() {
                    let selection = self.selection();
                    if right {
                        selection.end
                    } else {
                        selection.start
                    }
                } else {
                    step(text, self.cursor, right, command)
                };
                self.set_cursor(target, shift);
            }
            KeyCode::Home | KeyCode::End => {
                let end = key == KeyCode::End;
                let (target, upstream) = if command {
                    (if end { text.len() } else { 0 }, false)
                } else {
                    let (range, soft) = surface.line(text, self.pos());
                    if end {
                        (range.end, soft)
                    } else {
                        (range.start, false)
                    }
                };
                self.set_cursor(target, shift);
                self.upstream = upstream;
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::PageUp | KeyCode::PageDown => {
                let page = surface.page_lines();
                let lines = match key {
                    KeyCode::ArrowUp => -1,
                    KeyCode::ArrowDown => 1,
                    KeyCode::PageUp => -page,
                    _ => page,
                };
                if let Some((pos, x)) = surface.vertical(text, self.pos(), self.column, lines) {
                    self.set_pos(pos, shift);
                    self.column = Some(x);
                }
            }
            KeyCode::Backspace | KeyCode::Delete if !read_only => {
                if self.selection().is_empty() {
                    self.anchor = step(text, self.cursor, key == KeyCode::Delete, command);
                }
                return self.replace_selection(text, "");
            }
            _ => {}
        }
        None
    }
}
