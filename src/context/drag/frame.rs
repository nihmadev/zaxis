//! Pass lifecycle: registries are published with the hits they describe, lost
//! sources and missed releases are detected, and results are handed out once.
use super::super::Context;
use crate::components::drag_drop::{DragEnd, DragReason};

impl Context {
    pub(crate) fn drag_begin_frame(&mut self) {
        self.drag.depth = 0;
        if let Some(session) = self.drag.session.as_mut() {
            session.seen = false;
        }
        self.tick_drag_autoscroll();
    }

    /// Runs after the pass's hits become the published ones, so geometry,
    /// registries and the hovered target always describe the same frame.
    pub(crate) fn drag_finish_state(&mut self) {
        std::mem::swap(&mut self.drag.last_sources, &mut self.drag.sources);
        std::mem::swap(&mut self.drag.last_targets, &mut self.drag.targets);
        self.drag.sources.clear();
        self.drag.targets.clear();
        if self
            .drag
            .result
            .as_ref()
            .is_some_and(|(_, from)| self.frame >= *from)
        {
            self.drag.result = None;
        }
        // The pass had its chance to deliver the drop; unclaimed means the
        // target vanished or turned the payload down, so nothing happened.
        if let Some(ended) = self.drag.ended.take() {
            let claimed = ended.claimed;
            self.drag.result = Some((
                DragEnd {
                    reason: if claimed {
                        DragReason::Dropped
                    } else {
                        DragReason::Cancelled
                    },
                    source: ended.info.key,
                    target: claimed.then_some(ended.target.key),
                    position: ended.position,
                    insertion: ended.target.insertion.filter(|_| claimed),
                },
                self.frame + 1,
            ));
            self.request_repaint();
        }
        if let Some(session) = &self.drag.session {
            if !session.seen {
                let session = self.drag.session.take().unwrap();
                self.drag_conclude(session, DragReason::SourceLost);
            } else if !session.keyboard && !self.input.primary_down {
                // The release never arrived (lost event, host-side capture loss).
                self.drag_cancel(DragReason::Cancelled);
            }
        }
        self.drag_refresh_hover();
        self.drag.deadline = self
            .drag
            .scrolling
            .then(|| {
                self.frame_time
                    .checked_add(self.style.motion.frame_interval)
            })
            .flatten();
    }

    /// Application-facing id of the source being dragged, once the beginning
    /// has been reported.
    pub fn dragging(&self) -> Option<crate::Id> {
        self.drag
            .session
            .as_ref()
            .filter(|s| s.started)
            .map(|s| s.info.key)
    }

    /// The result of the session that just ended, for the one pass after it
    /// ended. Sources receive the same value in [`crate::DragOutput::finished`].
    pub fn drag_end(&self) -> Option<DragEnd> {
        self.drag
            .result
            .filter(|(_, from)| self.frame >= *from)
            .map(|(end, _)| end)
    }

    /// Cancel the current drag from application code, for example when a modal
    /// dialog opens. Reported as [`DragReason::Cancelled`].
    pub fn cancel_drag(&mut self) {
        self.drag_cancel(DragReason::Cancelled);
    }
}
