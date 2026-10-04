//! Repaint invalidation and deadline scheduling.

use super::Context;
use std::time::{Duration, Instant};

impl Context {
    /// Request another UI redraw, for example for an animation or custom interaction.
    /// The host must also wake its event loop. Model changes do not need cache invalidation;
    /// requesting a native redraw is sufficient for [`Self::run`] to observe them.
    pub fn request_repaint(&mut self) {
        self.dirty = true;
    }

    /// Schedule a future UI pass for an animation or timer, using the earliest deadline.
    /// The host should use [`winit::event_loop::ControlFlow::WaitUntil`] with this deadline.
    pub fn request_repaint_after(&mut self, delay: Duration) {
        if delay.is_zero() {
            self.request_repaint();
            return;
        }
        let now = if self.in_pass {
            self.frame_time
        } else {
            Instant::now()
        };
        if let Some(deadline) = now.checked_add(delay) {
            self.next_repaint = Some(self.next_repaint.map_or(deadline, |old| old.min(deadline)));
        }
    }

    pub fn next_repaint(&self) -> Option<Instant> {
        let deadline = match (self.next_repaint, self.animations.deadline()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let deadline = match (deadline, self.scrolling.auto_deadline) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        match (deadline, self.drag.deadline) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }
    pub fn needs_repaint(&self) -> bool {
        self.needs_repaint_at(Instant::now())
    }
    /// Check a deterministic host clock, matching `run_at`.
    pub fn needs_repaint_at(&self, now: Instant) -> bool {
        self.dirty
            || self.shared_state_changed()
            || self.next_repaint().is_some_and(|time| time <= now)
    }
    /// A visible continuous track or active middle-button scrolling wants the next presented frame. Vsync hosts
    /// should request redraw immediately after presenting and let the swapchain
    /// pace it. Immediate hosts should honor `next_repaint` instead. Delayed,
    /// paused, completed, cancelled and hidden channels return false.
    pub fn wants_animation_frame(&self) -> bool {
        self.animations.wants_frame()
            || self.scrolling.auto_deadline.is_some()
            || self.drag.scrolling
    }
}
