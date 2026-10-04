//! Pointer lifecycle of a drag: arm on press, begin after the threshold, end on
//! release, Escape, focus loss or resize. Every path ends through `drag_end`,
//! which publishes exactly one result.
use super::super::{interaction::Capture, Context, HitAction, HitRegion};
use super::state::{Ended, Pending, Returning, Session};
use crate::components::drag_drop::{DragEnd, DragReason};
use crate::Vec2;
use std::time::Instant;
use winit::window::CursorIcon;

/// Only plain activation surfaces may turn into a drag. Text selection, sliders,
/// resize handles, scroll thumbs, window chrome and custom widgets that sense drags
/// themselves keep their own gestures.
fn arms_drag(action: HitAction) -> bool {
    matches!(
        action,
        HitAction::Activate | HitAction::Focus | HitAction::Block | HitAction::TreeRow { .. }
    ) || matches!(action, HitAction::Interact(sense) if !sense.drag())
}

impl Context {
    /// Primary press. `top` is the interactive hit that captured it, if any.
    pub(crate) fn drag_press(&mut self, top: Option<HitRegion>) {
        self.drag.pending = None;
        if self.drag.session.as_ref().is_some_and(|s| s.keyboard) {
            self.drag_cancel(DragReason::Cancelled);
        }
        if self.drag.session.is_some() || top.is_some_and(|hit| !arms_drag(hit.action)) {
            return;
        }
        let Some(pointer) = self.input.pointer else {
            return;
        };
        let Some(window) = self.top_window(pointer) else {
            return;
        };
        let mut best = None;
        for hit in self.previous_hits.iter() {
            let HitAction::DragSource { slot } = hit.action else {
                continue;
            };
            if hit.window != window || !hit.rect.contains(pointer) || !hit.clip.contains(pointer) {
                continue;
            }
            let Some(info) = self.drag.last_sources.get(slot as usize) else {
                continue;
            };
            if best.is_none_or(|(_, deepest): (HitRegion, u16)| info.depth >= deepest) {
                best = Some((*hit, info.depth));
            }
        }
        let Some((hit, _)) = best else { return };
        let HitAction::DragSource { slot } = hit.action else {
            return;
        };
        let info = self.drag.last_sources[slot as usize];
        // Nothing but the source itself (or inert backdrop) was pressed, so it
        // reports the click that the capture would otherwise swallow.
        let passive = top.is_none_or(|hit| {
            hit.action == HitAction::Block
                || (hit.action == HitAction::Focus && Some(hit.id) == info.focus)
        });
        self.drag.pending = Some(Pending {
            hit,
            info,
            press: pointer,
            time: Instant::now(),
            passive,
        });
    }

    pub(crate) fn drag_move(&mut self, pointer: Vec2) {
        self.drag_move_at(pointer, Instant::now());
    }

    pub(crate) fn drag_move_at(&mut self, pointer: Vec2, now: Instant) {
        if let Some(session) = self.drag.session.as_mut() {
            if !session.keyboard {
                session.pointer = pointer;
                session.outside = false;
                self.drag_refresh_hover();
                self.dirty = true;
            }
            return;
        }
        let Some(pending) = &self.drag.pending else {
            return;
        };
        if !self.input.primary_down {
            self.drag.pending = None;
            return;
        }
        let resolved = pending.info.resolved;
        if (pointer - pending.press).length() <= resolved.threshold {
            return;
        }
        // Moving before the hold time elapsed means some other gesture (a scroll
        // drag, a selection) owns the press.
        if now.saturating_duration_since(pending.time) < resolved.delay {
            self.drag.pending = None;
            return;
        }
        let pending = self.drag.pending.take().unwrap();
        self.drag_begin(pending, pointer);
    }

    fn drag_begin(&mut self, pending: Pending, pointer: Vec2) {
        let hit = pending.hit;
        self.gesture_adopt(pending.info.id, pending.press, pointer);
        // The press belonged to whatever was under it; from here on the source
        // owns the capture, so that widget sees neither release nor click.
        self.capture = Some(Capture {
            hit,
            pointer,
            rect: hit.rect,
        });
        self.text_click = None;
        self.keyboard_active = None;
        self.drag.session = Some(Session {
            info: pending.info,
            window: hit.window,
            rect: hit.rect,
            grab: pending.press - hit.rect.min,
            pointer,
            keyboard: false,
            outside: false,
            payload: None,
            started: false,
            seen: false,
            hover: None,
            cursor: CursorIcon::Grabbing,
            snapshot: None,
            last_tick: None,
        });
        self.request_repaint();
    }

    pub(crate) fn drag_cursor_left(&mut self) {
        if let Some(session) = self.drag.session.as_mut().filter(|s| !s.keyboard) {
            session.outside = true;
            self.drag_refresh_hover();
        }
    }

    /// Primary release. Returns `true` when it ended a session.
    pub(crate) fn drag_release(&mut self) -> bool {
        if let Some(pending) = self.drag.pending.take() {
            let inside = self
                .input
                .pointer
                .is_some_and(|p| pending.hit.rect.contains(p) && pending.hit.clip.contains(p));
            if pending.passive && inside {
                self.register_click(pending.info.id);
            }
        }
        match self.drag.session.take() {
            Some(session) if !session.keyboard => {
                self.drag_conclude(session, DragReason::Dropped);
                true
            }
            other => {
                self.drag.session = other;
                false
            }
        }
    }

    /// End any pending or active drag with `reason`. Safe to call repeatedly:
    /// the second call finds nothing to end, so no event is duplicated.
    pub(crate) fn drag_cancel(&mut self, reason: DragReason) {
        self.drag.pending = None;
        if let Some(session) = self.drag.session.take() {
            self.drag_conclude(session, reason);
        }
    }

    /// Confirm a keyboard drag on the current target.
    pub(crate) fn drag_confirm(&mut self) {
        if let Some(session) = self.drag.session.take() {
            self.drag_conclude(session, DragReason::Dropped);
        }
    }

    pub(super) fn drag_conclude(&mut self, mut session: Session, mut reason: DragReason) {
        if self
            .capture
            .is_some_and(|capture| capture.hit.id == session.info.id)
        {
            self.capture = None;
        }
        self.gesture_end();
        self.drag.deadline = None;
        self.drag.scrolling = false;
        self.request_repaint();
        if !session.started {
            // Never reached the UI: the application saw no beginning, so it
            // gets no end either.
            return;
        }
        if reason == DragReason::Dropped {
            let hover = if session.keyboard {
                session.hover
            } else if session.outside {
                None
            } else {
                self.drag_resolve(session.pointer)
            };
            match hover.filter(|hover| hover.accepts) {
                Some(target) => {
                    self.drag.ended = Some(Ended {
                        info: session.info,
                        target,
                        position: session.pointer,
                        payload: session.payload.take(),
                        claimed: false,
                    });
                    return;
                }
                None => reason = DragReason::Cancelled,
            }
        }
        self.drag.result = Some((
            DragEnd {
                reason,
                source: session.info.key,
                target: None,
                position: session.pointer,
                insertion: None,
            },
            self.frame + 1,
        ));
        if matches!(reason, DragReason::Cancelled | DragReason::Escape) {
            self.drag_start_return(&session);
        }
    }

    fn drag_start_return(&mut self, session: &Session) {
        let resolved = session.info.resolved;
        let Some(snapshot) = session
            .snapshot
            .clone()
            .filter(|_| !resolved.reduced_motion)
        else {
            return;
        };
        let scale = super::preview::fit_scale(snapshot.rect.size(), resolved.preview_max_size);
        let from = super::preview::origin(session, scale);
        self.drag.returning = Some(Returning {
            snapshot,
            from,
            to: session.rect.min,
            scale,
            opacity: resolved.preview_opacity,
            tween: self
                .style
                .drag
                .return_motion
                .clone()
                .unwrap_or_else(|| self.style.motion.reorder.clone()),
            source: session.info.id,
        });
        self.remove_animation(super::preview::return_id());
    }
}
