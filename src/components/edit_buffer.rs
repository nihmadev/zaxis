//! Shared byte-indexed editing operations. Every persistent endpoint is a grapheme boundary.
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use winit::keyboard::{KeyCode, ModifiersState};

#[derive(Clone, Default)]
pub(crate) struct EditBuffer {
    pub cursor: usize,
    pub anchor: usize,
}

pub(crate) fn boundaries(text: &str) -> Vec<usize> {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect()
}

pub(crate) fn word_at(text: &str, cursor: usize) -> Range<usize> {
    let range = text
        .split_word_bound_indices()
        .find(|(start, word)| cursor < start + word.len())
        .map(|(start, word)| start..start + word.len())
        .unwrap_or_else(|| {
            text.split_word_bound_indices()
                .last()
                .map_or(0..0, |(start, word)| start..start + word.len())
        });
    let points = boundaries(text);
    let start = *points.iter().rev().find(|&&p| p <= range.start).unwrap();
    let end = *points.iter().find(|&&p| p >= range.end).unwrap();
    start..end
}

impl EditBuffer {
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }
    pub fn select_all(&mut self, text: &str) {
        self.anchor = 0;
        self.cursor = text.len();
    }
    pub fn set_cursor(&mut self, cursor: usize, extend: bool) {
        self.cursor = cursor;
        if !extend {
            self.anchor = cursor;
        }
    }
    pub fn clamp(&mut self, text: &str) {
        let points = boundaries(text);
        self.cursor = *points.iter().rev().find(|&&p| p <= self.cursor).unwrap();
        self.anchor = *points.iter().rev().find(|&&p| p <= self.anchor).unwrap();
    }
    pub fn insert(&mut self, text: &mut String, value: &str) {
        let range = self.selection();
        let end = range.start + value.len();
        text.replace_range(range, value);
        // Inserting a combining mark or ZWJ can join the following cluster.
        let end = boundaries(text)
            .into_iter()
            .find(|&p| p >= end)
            .unwrap_or(text.len());
        self.set_cursor(end, false);
    }
    fn step(&self, text: &str, right: bool, word: bool) -> usize {
        if word {
            let words: Vec<_> = text
                .split_word_bound_indices()
                .filter(|(_, s)| !s.chars().all(char::is_whitespace))
                .collect();
            let point = if right {
                words
                    .iter()
                    .find_map(|(i, s)| (*i + s.len() > self.cursor).then_some(*i + s.len()))
                    .unwrap_or(text.len())
            } else {
                words
                    .iter()
                    .rev()
                    .find_map(|(i, _)| (*i < self.cursor).then_some(*i))
                    .unwrap_or(0)
            };
            let points = boundaries(text);
            return if right {
                *points.iter().find(|&&p| p >= point).unwrap()
            } else {
                *points.iter().rev().find(|&&p| p <= point).unwrap()
            };
        }
        let points = boundaries(text);
        if right {
            points
                .into_iter()
                .find(|&p| p > self.cursor)
                .unwrap_or(text.len())
        } else {
            points
                .into_iter()
                .rev()
                .find(|&p| p < self.cursor)
                .unwrap_or(0)
        }
    }
    pub fn key(
        &mut self,
        text: &mut String,
        key: KeyCode,
        modifiers: ModifiersState,
        read_only: bool,
    ) {
        let command = modifiers.control_key() || modifiers.super_key();
        let shift = modifiers.shift_key();
        match key {
            KeyCode::KeyA if command => self.select_all(text),
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Home | KeyCode::End => {
                let right = key == KeyCode::ArrowRight;
                let target = match key {
                    KeyCode::Home => 0,
                    KeyCode::End => text.len(),
                    _ if !shift && !command && !self.selection().is_empty() => {
                        if right {
                            self.selection().end
                        } else {
                            self.selection().start
                        }
                    }
                    _ => self.step(text, right, command),
                };
                self.set_cursor(target, shift);
            }
            KeyCode::Backspace | KeyCode::Delete if !read_only => {
                if self.selection().is_empty() {
                    self.anchor = self.step(text, key == KeyCode::Delete, command);
                }
                self.insert(text, "");
            }
            _ => {}
        }
    }
}
