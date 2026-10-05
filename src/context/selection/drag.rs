//! A selection gesture in progress: which widget holds the pointer, the endpoint the pointer
//! currently names, and the scrolling of the containers around the press while the pointer
//! is held outside them.

use super::{Endpoint, Id};
use crate::time::Instant;
use crate::{context::Context, Vec2};
use std::time::Duration;

/// Pixels per second of scrolling for every pixel the pointer is outside a container.
const SPEED: f32 = 8.0;
const MAX_SPEED: f32 = 1800.0;
const TICK: Duration = Duration::from_millis(16);

pub(crate) struct Drag {
    /// The widget that captured the press; the gesture lasts while it keeps the capture.
    pub owner: Id,
    pub scope: Id,
    /// Press position, used to find the scroll containers the text lives in.
    pub origin: Vec2,
    /// The press was a double or triple click: its word or paragraph stays selected until
    /// the pointer really moves.
    pub multi: bool,
    /// The last endpoint offered by an item during the current pass.
    pub candidate: Option<Endpoint>,
    /// The drag started during this pass: items built before the pressed one could not
    /// offer yet, so the pass only places the anchor.
    fresh: bool,
    /// An offer was withheld (a multi-click that has not moved): keep the selection as is.
    held: bool,
    last_tick: Option<Instant>,
}

impl Context {
    pub(crate) fn selection_drag_begin(&mut self, owner: Id, scope: Id, origin: Vec2, multi: bool) {
        self.selection.drag = Some(Drag {
            owner,
            scope,
            origin,
            multi,
            candidate: None,
            fresh: true,
            held: false,
            last_tick: None,
        });
    }

    /// Whether the selection of `scope` is being dragged right now.
    pub(crate) fn selection_dragging(&self, scope: Id) -> bool {
        self.selection.drag.as_ref().is_some_and(|drag| {
            drag.scope == scope
                && (self.selection.released == Some(drag.owner)
                    || (self.input.primary_down
                        && self
                            .interaction
                            .capture
                            .is_some_and(|c| c.hit.id == drag.owner)))
        })
    }

    /// An item reports the endpoint the pointer names inside it; later items win, so the
    /// last item whose top is above the pointer decides, in document order.
    pub(crate) fn selection_offer(&mut self, endpoint: Endpoint) {
        let moved = self
            .input
            .pointer
            .zip(self.selection.drag.as_ref())
            .is_some_and(|(p, d)| (p - d.origin).length_squared() > 16.0);
        if let Some(drag) = self.selection.drag.as_mut() {
            if moved || !drag.multi {
                drag.candidate = Some(endpoint);
            } else {
                drag.held = true;
            }
        }
    }

    /// End of a pass over the scope: the head moves to the offered endpoint, or to the very
    /// start when the pointer is above the first item.
    pub(crate) fn selection_commit(&mut self, scope: Id) {
        if !self.selection_dragging(scope) {
            return;
        }
        let first = self
            .selection
            .scopes
            .get(&scope)
            .and_then(|s| s.order.first().copied());
        let Some(drag) = self.selection.drag.as_mut() else {
            return;
        };
        if std::mem::take(&mut drag.fresh) | std::mem::take(&mut drag.held) {
            drag.candidate = None;
            return;
        }
        let head = drag.candidate.take().or_else(|| {
            first.map(|item| Endpoint {
                item,
                byte: 0,
                ord: 0,
            })
        });
        if let Some(head) = head {
            self.selection_extend(scope, head);
        }
    }

    /// Scroll the containers around the press while the pointer is held outside them. Runs
    /// at the start of a pass; schedules the next tick only while it is scrolling.
    pub(crate) fn tick_selection_autoscroll(&mut self) {
        let now = self.frame_time;
        let Some(drag) = self.selection.drag.as_ref() else {
            return;
        };
        let (origin, scope) = (drag.origin, drag.scope);
        let pointer = self.input.pointer;
        let Some(pointer) = pointer.filter(|_| self.selection_dragging(scope)) else {
            if let Some(drag) = self.selection.drag.as_mut() {
                drag.last_tick = None;
            }
            return;
        };
        let dt = drag
            .last_tick
            .map_or(TICK, |last| now.saturating_duration_since(last))
            .min(Duration::from_millis(100))
            .as_secs_f32();
        let areas: Vec<Id> = self
            .scrolling
            .previous_order
            .iter()
            .rev()
            .copied()
            .collect();
        for id in areas {
            let Some(state) = self.scrolling.states.get(&id) else {
                continue;
            };
            if !state.enabled || !state.clip.contains(origin) {
                continue;
            }
            let mut velocity = Vec2::ZERO;
            for axis in 0..2 {
                if state.axes[axis] {
                    let outside = (pointer[axis] - state.clip.max[axis]).max(0.0)
                        + (pointer[axis] - state.clip.min[axis]).min(0.0);
                    velocity[axis] = (outside * SPEED).clamp(-MAX_SPEED, MAX_SPEED);
                }
            }
            if velocity == Vec2::ZERO
                || !crate::context::drag::autoscroll::can_move(state, velocity)
            {
                continue;
            }
            self.scroll_container(id, velocity * dt);
            if let Some(drag) = self.selection.drag.as_mut() {
                drag.last_tick = Some(now);
            }
            self.request_repaint_after(TICK);
            return;
        }
        if let Some(drag) = self.selection.drag.as_mut() {
            drag.last_tick = None;
        }
    }
}
