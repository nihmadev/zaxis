//! One-shot pointer and focus events behind `Response`: double and secondary
//! clicks, drag start/delta/stop, focus changes.
//!
//! Input handlers record events between passes; the pass that follows reads them
//! through `Response` and `finish_frame` clears them, so every user input is
//! reported exactly once. Built-in controls and `Ui::interact` share this path.

use super::{Context, HitAction, HitRegion, Id};
use crate::Vec2;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

const DRAG_THRESHOLD_SQ: f32 = 16.0;
const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(500);

struct DragGesture {
    id: Id,
    last: Vec2,
    origin: Vec2,
    started: bool,
}
struct SecondaryPress {
    ids: [Option<Id>; 2],
}
struct LastClick {
    id: Id,
    position: Vec2,
    time: Instant,
}

/// Events of one widget for the current pass.
#[derive(Clone, Copy, Default)]
pub(crate) struct Events {
    pub double_clicked: bool,
    pub secondary_clicked: bool,
    pub drag_started: bool,
    pub dragging: bool,
    pub drag_stopped: bool,
    pub drag_delta: Vec2,
    pub gained_focus: bool,
    pub lost_focus: bool,
}

#[derive(Default)]
pub(crate) struct Gestures {
    double_clicked: HashSet<Id>,
    secondary: HashMap<Id, Vec2>,
    drag_started: HashSet<Id>,
    drag_stopped: HashSet<Id>,
    drag_delta: HashMap<Id, Vec2>,
    focus_gained: HashSet<Id>,
    focus_lost: HashSet<Id>,
    drag: Option<DragGesture>,
    secondary_press: Option<SecondaryPress>,
    last_click: Option<LastClick>,
}

impl Gestures {
    pub(crate) fn events(&self, id: Id) -> Events {
        Events {
            double_clicked: self.double_clicked.contains(&id),
            secondary_clicked: self.secondary.contains_key(&id),
            drag_started: self.drag_started.contains(&id),
            dragging: self
                .drag
                .as_ref()
                .is_some_and(|drag| drag.id == id && drag.started)
                || self.drag_stopped.contains(&id),
            drag_stopped: self.drag_stopped.contains(&id),
            drag_delta: self.drag_delta.get(&id).copied().unwrap_or_default(),
            gained_focus: self.focus_gained.contains(&id),
            lost_focus: self.focus_lost.contains(&id),
        }
    }
    /// Pointer position of a secondary click delivered to `id` this pass.
    pub(crate) fn secondary_position(&self, id: Id) -> Option<Vec2> {
        self.secondary.get(&id).copied()
    }
    /// Drop delivered events; gestures still in progress survive.
    pub(super) fn finish_frame(&mut self) {
        self.double_clicked.clear();
        self.secondary.clear();
        self.drag_started.clear();
        self.drag_stopped.clear();
        self.drag_delta.clear();
        self.focus_gained.clear();
        self.focus_lost.clear();
    }
    pub(super) fn focus_changed(&mut self, lost: Option<Id>, gained: Option<Id>) {
        self.focus_lost.extend(lost);
        self.focus_gained.extend(gained);
    }
}

impl Context {
    /// A primary press was captured by `hit`.
    pub(super) fn gesture_press(&mut self, hit: HitRegion, pointer: Vec2) {
        self.gestures.drag = hit.action.sense().drag().then_some(DragGesture {
            id: hit.id,
            last: pointer,
            origin: pointer,
            started: false,
        });
    }

    /// The capture vanished (focus loss, removed widget): end gestures without a click.
    pub(super) fn gesture_cancel(&mut self) {
        self.gesture_end();
        self.gestures.secondary_press = None;
    }

    /// A drag-and-drop source took over the press: the source's `Response` reports the
    /// same `drag_started`, `drag_delta` and `drag_stopped` as any dragged control.
    pub(super) fn gesture_adopt(&mut self, id: Id, origin: Vec2, pointer: Vec2) {
        self.gestures.drag = Some(DragGesture {
            id,
            last: origin,
            origin,
            started: true,
        });
        self.gestures.drag_started.insert(id);
        self.gesture_move(pointer);
    }

    pub(super) fn gesture_move(&mut self, pointer: Vec2) {
        let Some(drag) = self.gestures.drag.as_mut() else {
            return;
        };
        if !drag.started {
            if (pointer - drag.origin).length_squared() <= DRAG_THRESHOLD_SQ {
                return;
            }
            drag.started = true;
            self.gestures.drag_started.insert(drag.id);
        }
        let delta = pointer - drag.last;
        drag.last = pointer;
        *self.gestures.drag_delta.entry(drag.id).or_default() += delta;
    }

    /// The primary button was released while `hit` held the capture.
    pub(super) fn gesture_release(&mut self, hit: HitRegion) {
        let dragged = self.gesture_end().is_some_and(|id| id == hit.id);
        if !dragged
            && hit.action.sense().click()
            && self
                .input
                .pointer
                .and_then(|p| self.hit_test(p))
                .is_some_and(|under| under.id == hit.id)
        {
            self.register_click(hit.id);
        }
    }

    /// Finish a started drag, if any, and report the dragged widget.
    pub(super) fn gesture_end(&mut self) -> Option<Id> {
        let drag = self.gestures.drag.take()?;
        drag.started.then(|| {
            self.gestures.drag_stopped.insert(drag.id);
            drag.id
        })
    }

    pub(super) fn register_click(&mut self, id: Id) {
        self.clicked.insert(id);
        let Some(pointer) = self.input.pointer else {
            return;
        };
        let now = Instant::now();
        let double = self.gestures.last_click.take().is_some_and(|last| {
            last.id == id
                && now.saturating_duration_since(last.time) <= DOUBLE_CLICK_TIME
                && (pointer - last.position).length_squared() <= DRAG_THRESHOLD_SQ
        });
        if double {
            self.gestures.double_clicked.insert(id);
        } else {
            self.gestures.last_click = Some(LastClick {
                id,
                position: pointer,
                time: now,
            });
        }
    }

    /// Record what a secondary press hit; returns whether anything responds.
    pub(super) fn gesture_secondary_press(&mut self, pointer: Vec2) -> bool {
        let ids = self.secondary_targets(pointer);
        self.gestures.secondary_press = Some(SecondaryPress { ids });
        ids.iter().any(Option::is_some)
    }

    pub(super) fn gesture_secondary_release(&mut self) {
        let Some(press) = self.gestures.secondary_press.take() else {
            return;
        };
        let Some(pointer) = self.input.pointer else {
            return;
        };
        let now = self.secondary_targets(pointer);
        for id in press.ids.into_iter().flatten() {
            if now.contains(&Some(id)) {
                self.gestures.secondary.insert(id, pointer);
            }
        }
    }

    /// The topmost control and the topmost context-menu anchor under `pointer`.
    fn secondary_targets(&self, pointer: Vec2) -> [Option<Id>; 2] {
        let window = self.top_window(pointer);
        let anchor = self
            .previous_hits
            .iter()
            .rev()
            .find(|hit| {
                Some(hit.window) == window
                    && hit.action == HitAction::ContextMenu
                    && hit.rect.contains(pointer)
                    && hit.clip.contains(pointer)
            })
            .map(|hit| hit.id);
        [self.hit_test(pointer).map(|hit| hit.id), anchor]
    }
}
