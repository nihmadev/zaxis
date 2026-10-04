//! Retained scroll routing and deferred content paint. Layout runs once; after measuring,
//! offset corrections move both paint and hits before either becomes visible to the host.
use std::collections::HashMap;

use super::{Context, HitAction, HitRegion, Id, Paint};
use crate::{Rect, Vec2};

#[derive(Clone, Debug)]
pub(crate) struct ScrollState {
    pub offset: Vec2,
    pub content: Vec2,
    pub viewport: Rect,
    pub visual_scale: f32,
    pub clip: Rect,
    pub parent: Option<Id>,
    pub window: Id,
    pub axes: [bool; 2],
    pub enabled: bool,
    pub middle_mouse_scroll: bool,
    pub last_frame: u64,
    pub drag_origin: Vec2,
    pub travel: Vec2,
}
impl ScrollState {
    pub fn new(window: Id) -> Self {
        Self {
            offset: Vec2::ZERO,
            content: Vec2::ZERO,
            viewport: Rect::default(),
            visual_scale: 1.0,
            clip: Rect::default(),
            parent: None,
            window,
            axes: [false; 2],
            enabled: true,
            middle_mouse_scroll: true,
            last_frame: 0,
            drag_origin: Vec2::ZERO,
            travel: Vec2::ZERO,
        }
    }
    pub fn max_offset(&self) -> Vec2 {
        let mut max = (self.content - self.viewport.size() / self.visual_scale).max(Vec2::ZERO);
        for axis in 0..2 {
            if !self.axes[axis] {
                max[axis] = 0.0;
            }
        }
        max
    }
}

pub(crate) struct ScrollScope {
    pub visual_scale: f32,
    pub id: Id,
    pub window: Id,
    pub parent: Option<usize>,
    pub viewport: Rect,
    pub outer_clip: Rect,
    pub region: Rect,
    pub correction: Vec2,
    pub target: Option<Rect>,
    pub origin: Vec2,
}
pub(crate) struct PendingPaint {
    pub id: Id,
    pub(super) layer: Id,
    pub(super) clip: Rect,
    pub(super) paint: Vec<Paint>,
    pub blur: Option<f32>,
    pub(super) scope: usize,
}
#[derive(Default)]
pub(crate) struct Scrolling {
    pub states: HashMap<Id, ScrollState>,
    pub scopes: Vec<ScrollScope>,
    pub stack: Vec<usize>,
    pub pending: Vec<PendingPaint>,
    pub(super) hits: Vec<(HitRegion, usize)>,
    pub(super) ime: Option<(usize, Rect)>,
    pub order: Vec<Id>,
    pub(super) previous_order: Vec<Id>,
    pub(super) auto: Option<super::scroll_input::AutoScroll>,
    pub(super) auto_deadline: Option<std::time::Instant>,
}
impl Scrolling {
    pub(crate) fn remove_layer_hits(&mut self, layer: Id) {
        self.hits.retain(|(hit, _)| hit.window != layer);
        for state in self
            .states
            .values_mut()
            .filter(|state| state.window == layer)
        {
            state.enabled = false;
        }
    }
    pub fn begin_frame(&mut self) {
        self.order.clear();
    }
    pub fn finish_frame(&mut self, frame: u64) {
        self.previous_order = self.order.clone();
        // Offsets survive hiding/reordering; only the live tree routes input.
        self.previous_order
            .retain(|id| self.states[id].last_frame == frame);
    }
    pub(crate) fn owner(&self, window: Id) -> Option<usize> {
        self.stack
            .last()
            .copied()
            .filter(|&n| self.scopes[n].window == window)
    }
    fn shift(&self, mut scope: Option<usize>) -> Vec2 {
        let mut shift = Vec2::ZERO;
        while let Some(n) = scope {
            shift += self.scopes[n].correction;
            scope = self.scopes[n].parent;
        }
        shift
    }
    fn clip(&self, mut n: usize, mut clip: Rect) -> Rect {
        loop {
            let scope = &self.scopes[n];
            let shift = self.shift(scope.parent);
            clip = clip
                .intersect(scope.viewport.translate(shift))
                .intersect(scope.outer_clip.translate(shift));
            match scope.parent {
                Some(parent) => n = parent,
                None => return clip,
            }
        }
    }
}
impl Context {
    pub(crate) fn scroll_visible(&self, window: Id, rect: Rect, clip: Rect) -> bool {
        self.scrolling.owner(window).is_none()
            || !rect.intersect(self.scroll_clip(window, clip)).is_empty()
    }
    pub(crate) fn scroll_clip(&self, window: Id, clip: Rect) -> Rect {
        match self.scrolling.owner(window) {
            Some(n) => self.scrolling.clip(n, clip),
            None => clip,
        }
    }
    pub(crate) fn defer_scroll_paint(
        &mut self,
        id: Id,
        layer: Id,
        clip: Rect,
        paint: Vec<Paint>,
    ) -> Result<(), Vec<Paint>> {
        let Some(scope) = self.scrolling.owner(layer) else {
            return Err(paint);
        };
        self.scrolling.pending.push(PendingPaint {
            id,
            layer,
            clip,
            paint,
            blur: None,
            scope,
        });
        Ok(())
    }
    pub(crate) fn defer_scroll_hit(&mut self, hit: HitRegion) -> bool {
        let Some(scope) = self.scrolling.owner(hit.window) else {
            return false;
        };
        self.scrolling.hits.push((hit, scope));
        true
    }
    pub(crate) fn set_ime_area(&mut self, window: Id, rect: Rect) {
        if self.defer_placement_ime(window, rect) {
            return;
        }
        if let Some(n) = self.scrolling.owner(window) {
            self.scrolling.ime = Some((n, rect));
        } else {
            self.ime_area = Some(rect);
        }
    }
    pub(crate) fn end_scroll(&mut self) {
        self.scrolling.stack.pop();
        self.flush_scroll();
    }
    pub(crate) fn flush_scroll(&mut self) {
        if !self.scrolling.stack.is_empty() || self.placements.outstanding > 0 {
            return;
        }
        for pending in std::mem::take(&mut self.scrolling.pending) {
            let shift = self.scrolling.shift(Some(pending.scope));
            let clip = self
                .scrolling
                .clip(pending.scope, pending.clip.translate(shift));
            let mut paint = pending.paint;
            for primitive in &mut paint {
                primitive.translate(shift);
            }
            self.paint(pending.id, pending.layer, clip, paint);
            if let Some(element) = self.elements.last_mut().filter(|e| e.id == pending.id) {
                element.blur = pending.blur;
            }
        }
        for (mut hit, scope) in std::mem::take(&mut self.scrolling.hits) {
            let shift = self.scrolling.shift(Some(scope));
            if let Some(transform) = self.current_transforms.get_mut(&hit.id) {
                transform.translation += shift;
            }
            hit.rect = hit.rect.translate(shift);
            hit.clip = self.scrolling.clip(scope, hit.clip.translate(shift));
            self.hits.push(hit);
        }
        if let Some((scope, rect)) = self.scrolling.ime.take() {
            let rect = self
                .scrolling
                .clip(scope, rect.translate(self.scrolling.shift(Some(scope))));
            self.ime_area = (!rect.is_empty()).then_some(rect);
        }
        for n in 0..self.scrolling.scopes.len() {
            let scope = &self.scrolling.scopes[n];
            let viewport = scope.viewport.translate(self.scrolling.shift(scope.parent));
            let region = scope
                .region
                .translate(self.scrolling.shift(scope.parent))
                .intersect(
                    scope
                        .outer_clip
                        .translate(self.scrolling.shift(scope.parent)),
                );
            let clip = match scope.parent {
                Some(parent) => self.scrolling.clip(parent, region),
                None => region,
            };
            let state = self.scrolling.states.get_mut(&scope.id).unwrap();
            state.viewport = viewport;
            state.visual_scale = scope.visual_scale;
            state.clip = clip;
        }
        self.scrolling.scopes.clear();
    }
    pub(crate) fn scroll_target(&mut self, window: Id, rect: Rect) {
        if let Some(n) = self.scrolling.owner(window) {
            let scope = &mut self.scrolling.scopes[n];
            let target = rect.translate(-scope.origin);
            scope.target = Some(target);
            self.record_placement_target(window, n, target);
        }
    }
    pub(super) fn scroll_wheel(&mut self, delta: Vec2) -> bool {
        if !delta.is_finite() {
            return false;
        }
        let Some(pointer) = self.input.pointer else {
            return false;
        };
        let Some(window) = self.top_window(pointer) else {
            return false;
        };
        let target = self
            .scrolling
            .previous_order
            .iter()
            .rev()
            .copied()
            .find(|id| {
                let s = &self.scrolling.states[id];
                s.window == window && s.enabled && s.clip.contains(pointer)
            });
        target.is_some_and(|id| self.scroll_from(id, delta, false))
            || self.popup.is_some()
            || self.modal_active()
    }
    pub(super) fn scroll_from(&mut self, id: Id, mut delta: Vec2, middle: bool) -> bool {
        let mut current = Some(id);
        let mut changed = false;
        let window = self.scrolling.states[&id].window;
        while let Some(id) = current {
            let state = self.scrolling.states.get_mut(&id).unwrap();
            let old = state.offset;
            if state.enabled && (!middle || state.middle_mouse_scroll) {
                // A wheel without horizontal motion scrolls a horizontal-only area
                // (tab strips, chip rows); what it cannot consume passes outward.
                let remap = !middle && state.axes == [true, false] && delta.x == 0.0;
                let step = if remap {
                    Vec2::new(delta.y, 0.0)
                } else {
                    delta
                };
                state.offset =
                    (old + step / state.visual_scale).clamp(Vec2::ZERO, state.max_offset());
                for axis in 0..2 {
                    if !state.axes[axis] {
                        state.offset[axis] = old[axis];
                    }
                }
                let moved = (state.offset - old) * state.visual_scale;
                if remap {
                    delta.y -= moved.x;
                } else {
                    delta -= moved;
                }
                changed |= old != state.offset;
            }
            current = state.parent;
        }
        if changed {
            self.invalidate_scroll_hits(window);
            if !self.in_pass {
                self.request_repaint();
            }
        }
        changed
    }
    pub(super) fn invalidate_scroll_hits(&mut self, window: Id) {
        // Input can arrive in bursts before redraw. Never activate stale content geometry.
        self.previous_hits.retain(|h| {
            h.window != window
                || matches!(
                    h.action,
                    HitAction::Block
                        | HitAction::Move
                        | HitAction::Resize
                        | HitAction::ScrollThumb { .. }
                )
        });
    }
    pub(super) fn begin_scroll_drag(&mut self, id: Id, _axis: usize) {
        if let Some(s) = self.scrolling.states.get_mut(&id) {
            s.drag_origin = s.offset;
        }
    }
    pub(super) fn drag_scroll(&mut self, id: Id, axis: usize, delta: Vec2) {
        let Some(s) = self.scrolling.states.get_mut(&id) else {
            return;
        };
        if !s.enabled || s.travel[axis] <= 0.0 {
            return;
        }
        let max = s.max_offset()[axis];
        let old = s.offset;
        s.offset[axis] = (s.drag_origin[axis]
            + delta[axis] * max / (s.travel[axis] * s.visual_scale))
            .clamp(0.0, max);
        let window = s.window;
        if old != s.offset {
            self.invalidate_scroll_hits(window);
            self.request_repaint();
        }
    }
}
