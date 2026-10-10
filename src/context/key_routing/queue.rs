//! The bounded queue of events waiting for their owners.

use super::{KeyRouting, QUEUE_PER_WIDGET, QUEUE_TOTAL};
use crate::context::{key_events::KeyEvent, Id};

/// What became of an event offered to the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pushed {
    Queued,
    /// The queue was full. The first drop since the queue last emptied is `First`, so the
    /// caller reports it once.
    Dropped {
        first: bool,
    },
}

impl KeyRouting {
    /// Queue `event` for `id`. A release is always kept: it ends a press that was
    /// delivered, and there are at most as many as keys held. Presses and repeats beyond
    /// the limits are dropped, oldest events first in line.
    pub(crate) fn push(&mut self, id: Id, event: KeyEvent) -> Pushed {
        let release = event.is_release();
        if !release
            && (self.pending >= QUEUE_TOTAL
                || self
                    .queue
                    .get(&id)
                    .is_some_and(|waiting| waiting.len() >= QUEUE_PER_WIDGET))
        {
            self.dropped += 1;
            let first = !self.overflowed;
            self.overflowed = true;
            return Pushed::Dropped { first };
        }
        self.queue.entry(id).or_default().push_back(event);
        self.pending += 1;
        Pushed::Queued
    }

    /// The events addressed to `id`, in arrival order.
    pub(crate) fn take(&mut self, id: Id) -> Vec<KeyEvent> {
        match self.queue.remove(&id) {
            Some(waiting) => {
                self.pending -= waiting.len();
                waiting.into()
            }
            None => Vec::new(),
        }
    }
}
