//! Placing measured content: everything recorded for a placement moves by the measured
//! offset (content of the open popup by the popup's own shift), meets the placed region,
//! and is handed on to the enclosing placement, the scroll queues or the frame.

use super::{portal::Portals, Placement};
use crate::{
    context::{scroll::PendingPaint, Context, HitRegion},
    Rect, Vec2,
};
use std::collections::HashSet;

impl Context {
    pub(crate) fn place(&mut self, p: Placement, delta: Vec2, clip: Rect) {
        let viewport = self.viewport();
        let portals = self.move_portals(&p, delta);
        self.move_scroll_content(&p, delta, clip, viewport, &portals);
        self.place_ime(&p, delta, clip, viewport, &portals);
        self.place_targets(&p, delta);
        let (outer, clipped) = (p.outer_clip(clip, viewport), p.clipped_by());
        let Placement { paints, hits, .. } = p;
        let hit_ids: HashSet<_> = hits.iter().map(|(hit, _)| hit.id).collect();
        for mut paint in paints {
            let delta = portals.shift(paint.layer, delta);
            // A paint without a region of its own still reports where it is displayed.
            if delta != Vec2::ZERO && !hit_ids.contains(&paint.id) {
                self.visuals.translate(paint.id, delta);
            }
            paint.translate(delta);
            if clipped(paint.scope) {
                paint.clip = paint.clip.intersect(outer(paint.layer));
            }
            self.emit_placed_paint(paint);
        }
        let mut translated = HashSet::new();
        for (mut hit, scope) in hits {
            let delta = portals.hit_shift(hit.window, hit.id, delta);
            if delta != Vec2::ZERO && translated.insert(hit.id) {
                self.visuals.translate(hit.id, delta);
            }
            hit.translate(delta);
            if clipped(scope) {
                hit.clip = hit.clip.intersect(outer(hit.window));
            }
            self.emit_placed_hit(hit, scope);
        }
        self.placements.outstanding -= 1;
        self.flush_scroll();
    }

    /// Content of scroll areas opened inside the placement is still queued: move it too.
    /// Descendant scroll scopes clip at their viewport, not at content origin.
    fn move_scroll_content(
        &mut self,
        p: &Placement,
        delta: Vec2,
        clip: Rect,
        viewport: Rect,
        portals: &Portals,
    ) {
        let (outer, clipped) = (p.outer_clip(clip, viewport), p.clipped_by());
        for pending in &mut self.scrolling.pending[p.paints_range.clone()] {
            if !portals.moves(p, pending.layer) {
                continue;
            }
            pending.translate(portals.shift(pending.layer, delta));
            if clipped(Some(pending.scope)) {
                pending.clip = pending.clip.intersect(outer(pending.layer));
            }
        }
        for (hit, scope) in &mut self.scrolling.hits[p.hits_range.clone()] {
            if !portals.moves(p, hit.window) {
                continue;
            }
            hit.translate(portals.shift(hit.window, delta));
            if clipped(Some(*scope)) {
                hit.clip = hit.clip.intersect(outer(hit.window));
            }
        }
        for scope in &mut self.scrolling.scopes[p.scopes_range.clone()] {
            if !portals.moves(p, scope.window) {
                continue;
            }
            scope.translate(portals.shift(scope.window, delta));
            if clipped(scope.parent) {
                scope.outer_clip = scope.outer_clip.intersect(outer(scope.window));
            }
        }
    }

    /// The IME area of a field in the placement goes to the enclosing placement, its
    /// scroll area or the frame.
    fn place_ime(
        &mut self,
        p: &Placement,
        delta: Vec2,
        clip: Rect,
        viewport: Rect,
        portals: &Portals,
    ) {
        let Some((window, scope, rect)) = p.ime else {
            return;
        };
        let mut rect = rect.translate(portals.shift(window, delta));
        if p.clipped_by()(scope) {
            rect = rect.intersect(p.outer_clip(clip, viewport)(window));
        }
        if let Some(parent) = self.collecting(window) {
            parent.ime = Some((window, scope, rect));
        } else if let Some(scope) = scope {
            self.scrolling.ime = Some((scope, rect));
        } else {
            self.ime_area = (!rect.is_empty()).then_some(rect);
        }
    }

    /// Scroll targets in content coordinates of a scope opened outside the placement move
    /// with it; the enclosing placement records them in turn.
    fn place_targets(&mut self, p: &Placement, delta: Vec2) {
        for &(scope, target) in &p.targets {
            let target = if scope < p.scopes_range.start {
                target.translate(delta)
            } else {
                target
            };
            self.scrolling.scopes[scope].target = Some(target);
            self.record_placement_target(p.window, scope, target);
        }
    }

    /// A placed paint goes to the enclosing placement, the scroll queue of its scope, or
    /// the frame.
    fn emit_placed_paint(&mut self, paint: super::PlacedPaint) {
        if let Some(parent) = self.collecting(paint.layer) {
            parent.paints.push(paint);
        } else if let Some(scope) = paint.scope {
            self.record_paint_order(paint.id);
            self.scrolling.pending.push(PendingPaint {
                id: paint.id,
                layer: paint.layer,
                clip: paint.clip,
                paint: paint.paint,
                blur: paint.blur,
                material: paint.material,
                scope,
            });
        } else {
            self.paint(paint.id, paint.layer, paint.clip, paint.paint);
            if let Some(material) = paint.material {
                self.mark_material(paint.id, material, paint.blur);
            } else if let Some(sigma) = paint.blur {
                self.mark_blur(paint.id, sigma);
            }
        }
    }

    /// A placed hit goes to the enclosing placement, the scroll queue of its scope, or
    /// this pass's regions.
    fn emit_placed_hit(&mut self, hit: HitRegion, scope: Option<usize>) {
        if let Some(parent) = self.collecting(hit.window) {
            parent.hits.push((hit, scope));
        } else if let Some(scope) = scope {
            self.scrolling.hits.push((hit, scope));
        } else {
            self.route_hit(hit);
        }
    }
}
