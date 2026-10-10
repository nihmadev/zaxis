//! Counters of what the overlay did, readable from any thread.

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// A snapshot of the overlay's counters; see [`Overlay::stats`](crate::Overlay::stats).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OverlayStats {
    /// Presents of the host the overlay saw.
    pub presents: u64,
    /// Frames the overlay drew into.
    pub frames_drawn: u64,
    /// Interface passes (`Context::run`). Fewer than `frames_drawn` when the interface rests.
    pub ui_passes: u64,
    /// Presents passed through because the overlay was hidden.
    pub skipped_hidden: u64,
    /// Presents passed through to catch up after a frame over budget.
    pub skipped_budget: u64,
    /// Presents passed through because the backend was not ready or failed this frame.
    pub skipped_backend: u64,
    /// Presents that arrived while a frame was running on the thread (re-entry).
    pub skipped_reentrant: u64,
    /// Presents from a thread other than the one that owns the interface.
    pub skipped_foreign_thread: u64,
    /// Frames that cost more than the budget.
    pub over_budget: u64,
    /// Backend errors (frame, lost target, unsupported).
    pub backend_errors: u64,
    /// Input events dropped because the queue was full.
    pub dropped_input: u64,
    /// CPU time of the last interface pass and of the last recording, in microseconds.
    pub last_ui_micros: u64,
    pub last_render_micros: u64,
}

#[derive(Default)]
pub(crate) struct Counters {
    pub presents: AtomicU64,
    pub frames_drawn: AtomicU64,
    pub ui_passes: AtomicU64,
    pub skipped_hidden: AtomicU64,
    pub skipped_budget: AtomicU64,
    pub skipped_backend: AtomicU64,
    pub skipped_reentrant: AtomicU64,
    pub skipped_foreign_thread: AtomicU64,
    pub over_budget: AtomicU64,
    pub backend_errors: AtomicU64,
    pub dropped_input: AtomicU64,
    pub last_ui_micros: AtomicU64,
    pub last_render_micros: AtomicU64,
}

impl Counters {
    pub fn bump(counter: &AtomicU64) {
        counter.fetch_add(1, Relaxed);
    }

    pub fn snapshot(&self) -> OverlayStats {
        OverlayStats {
            presents: self.presents.load(Relaxed),
            frames_drawn: self.frames_drawn.load(Relaxed),
            ui_passes: self.ui_passes.load(Relaxed),
            skipped_hidden: self.skipped_hidden.load(Relaxed),
            skipped_budget: self.skipped_budget.load(Relaxed),
            skipped_backend: self.skipped_backend.load(Relaxed),
            skipped_reentrant: self.skipped_reentrant.load(Relaxed),
            skipped_foreign_thread: self.skipped_foreign_thread.load(Relaxed),
            over_budget: self.over_budget.load(Relaxed),
            backend_errors: self.backend_errors.load(Relaxed),
            dropped_input: self.dropped_input.load(Relaxed),
            last_ui_micros: self.last_ui_micros.load(Relaxed),
            last_render_micros: self.last_render_micros.load(Relaxed),
        }
    }
}
