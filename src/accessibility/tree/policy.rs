//! When bounds that move with the UI are published.
//!
//! While the UI is in motion (a repaint was asked for in the pass, or an animation wants
//! the next frame), a node whose only change is where it is keeps the bounds it was last
//! published with: assistive technology does not need every frame of a slide. What is held
//! is published by the first pass after the motion stops, and at the latest by the first
//! pass `HOLD` after the pass that started holding. A pass that holds something schedules
//! a pass `HOLD` later, so the final bounds go out even when nothing else redraws. A pass
//! that holds nothing schedules nothing: an idle UI publishes nothing and stays idle.

use crate::time::Instant;
use std::time::Duration;

/// Bounds that change while the UI is in motion are sent at most this often.
const HOLD: Duration = Duration::from_millis(200);

/// Whether the UI is moving in this pass, and when the pass is.
#[derive(Clone, Copy)]
pub(super) struct Motion {
    pub moving: bool,
    pub now: Instant,
}

/// The hold on moving bounds, carried from pass to pass.
#[derive(Clone, Copy, Default)]
pub(super) struct MotionHold {
    /// The pass that started holding bounds that are still unpublished.
    since: Option<Instant>,
}

impl MotionHold {
    /// Whether bounds-only changes of this pass may keep their published bounds: the UI
    /// is in motion and nothing has been held for `HOLD` yet.
    pub(super) fn applies(&self, motion: Motion) -> bool {
        motion.moving
            && self
                .since
                .is_none_or(|since| motion.now.saturating_duration_since(since) < HOLD)
    }

    /// Note whether this pass held anything. Returns the delay after which a pass has to
    /// run to publish what was held, or `None` when everything is published.
    pub(super) fn settle(&mut self, held: bool, now: Instant) -> Option<Duration> {
        if held {
            self.since.get_or_insert(now);
            Some(HOLD)
        } else {
            self.since = None;
            None
        }
    }
}
