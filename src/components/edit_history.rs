//! Per-field undo snapshots, including cursor and selection.
use super::edit_buffer::EditBuffer;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

struct Snapshot {
    text: String,
    buffer: EditBuffer,
}

#[derive(Default)]
pub(crate) struct EditHistory {
    undo: VecDeque<Snapshot>,
    redo: Vec<Snapshot>,
    typing: Option<Instant>,
}

impl EditHistory {
    pub fn break_group(&mut self) {
        self.typing = None;
    }

    pub fn record(&mut self, text: String, buffer: EditBuffer, typing: bool) {
        let now = Instant::now();
        let merge = typing
            && self
                .typing
                .is_some_and(|last| now.duration_since(last) <= Duration::from_secs(1));
        if !merge {
            self.undo.push_back(Snapshot { text, buffer });
            if self.undo.len() > 100 {
                self.undo.pop_front();
            }
        }
        self.redo.clear();
        self.typing = typing.then_some(now);
    }

    pub fn restore(&mut self, text: &mut String, buffer: &mut EditBuffer, redo: bool) {
        self.break_group();
        let snapshot = if redo {
            self.redo.pop()
        } else {
            self.undo.pop_back()
        };
        if let Some(snapshot) = snapshot {
            let current = Snapshot {
                text: std::mem::replace(text, snapshot.text),
                buffer: buffer.clone(),
            };
            *buffer = snapshot.buffer;
            if redo {
                self.undo.push_back(current);
            } else {
                self.redo.push(current);
            }
        }
    }
}
