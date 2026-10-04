//! Per-field undo history stored as text replacements plus cursor and selection, so an
//! edit costs the size of the change rather than the size of the document.
use super::edit_buffer::{Change, Delta, EditBuffer};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const MAX_ENTRIES: usize = 200;
/// Memory cap for retained text; the newest entry is always kept.
const MAX_BYTES: usize = 8 << 20;
const GROUP: Duration = Duration::from_secs(1);

struct Entry {
    change: Change,
    before: EditBuffer,
    after: EditBuffer,
}

impl Entry {
    fn size(&self) -> usize {
        self.change.removed.len() + self.change.inserted.len() + 96
    }
}

#[derive(Default)]
pub(crate) struct EditHistory {
    undo: VecDeque<Entry>,
    redo: Vec<Entry>,
    typing: Option<Instant>,
    bytes: usize,
}

impl EditHistory {
    pub fn break_group(&mut self) {
        self.typing = None;
    }

    /// Record an applied change. Consecutive typing within a second joins one undo step.
    pub fn record(&mut self, change: Change, before: EditBuffer, after: EditBuffer, typing: bool) {
        let now = Instant::now();
        let merge = typing
            && self
                .typing
                .is_some_and(|last| now.duration_since(last) <= GROUP)
            && self.undo.back().is_some_and(|last| {
                last.change.at + last.change.inserted.len() == change.at
                    && change.removed.is_empty()
            });
        if merge {
            let last = self.undo.back_mut().expect("merge needs an entry");
            self.bytes += change.inserted.len();
            last.change.inserted.push_str(&change.inserted);
            last.after = after;
        } else {
            let entry = Entry {
                change,
                before,
                after,
            };
            self.bytes += entry.size();
            self.undo.push_back(entry);
        }
        self.bytes -= self.redo.drain(..).map(|e| e.size()).sum::<usize>();
        while self.undo.len() > 1 && (self.undo.len() > MAX_ENTRIES || self.bytes > MAX_BYTES) {
            if let Some(old) = self.undo.pop_front() {
                self.bytes -= old.size();
            }
        }
        self.typing = typing.then_some(now);
    }

    /// Undo or redo one step. Returns the replacement applied to `text`.
    pub fn restore(
        &mut self,
        text: &mut String,
        buffer: &mut EditBuffer,
        redo: bool,
    ) -> Option<Delta> {
        self.break_group();
        let entry = if redo {
            self.redo.pop()
        } else {
            self.undo.pop_back()
        }?;
        let Change {
            at,
            removed,
            inserted,
        } = &entry.change;
        let (old, new) = if redo {
            (removed, inserted)
        } else {
            (inserted, removed)
        };
        let end = at + old.len();
        if text.get(*at..end) != Some(old.as_str()) {
            // The text no longer matches the history: nothing safe to restore.
            *self = Self::default();
            return None;
        }
        text.replace_range(*at..end, new);
        *buffer = if redo {
            entry.after.clone()
        } else {
            entry.before.clone()
        };
        let delta = Delta {
            at: *at,
            old_len: old.len(),
            new_len: new.len(),
        };
        if redo {
            self.undo.push_back(entry);
        } else {
            self.redo.push(entry);
        }
        Some(delta)
    }

    #[cfg(test)]
    pub fn entries(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
}
