//! Deferred cell paint/hits. User code executes once; measured cells move as a unit.
use super::{Context, HitRegion, Id, Paint};
use crate::{Rect, Vec2};
use std::ops::Range;

pub(crate) struct PlacedPaint {
    pub id: Id,
    pub layer: Id,
    pub clip: Rect,
    pub paint: Vec<Paint>,
    pub blur: Option<f32>,
    scope: Option<usize>,
}
pub(crate) struct Placement {
    animation_start: usize,
    popup_start: usize,
    window: Id,
    pub paints: Vec<PlacedPaint>,
    pub hits: Vec<(HitRegion, Option<usize>)>,
    paints_range: Range<usize>,
    hits_range: Range<usize>,
    scopes_range: Range<usize>,
    ime: Option<(Option<usize>, Rect)>,
    targets: Vec<(usize, Rect)>,
}
#[derive(Default)]
pub(crate) struct Placements {
    pub stack: Vec<Placement>,
    pub outstanding: usize,
}
impl Context {
    pub(crate) fn hide_placement_animations(&mut self, p: &Placement) {
        self.animations.hide_since(p.animation_start);
    }
    /// Transform measured paint and routing together, before placement/scroll emission.
    pub(crate) fn place_visual(
        &mut self,
        mut p: Placement,
        transform: crate::Transform,
        opacity: f32,
        clip: Rect,
        interactive: bool,
    ) {
        if opacity <= 0.0 || clip.is_empty() {
            self.animations.hide_since(p.animation_start);
        }
        let wrap = |paint: &mut Vec<Paint>| {
            *paint = vec![Paint::Visual {
                paint: std::mem::take(paint),
                transform,
                opacity,
            }];
        };
        let viewport = self.viewport();
        if let Some(popup) = self
            .popup
            .as_mut()
            .filter(|popup| p.paints.iter().any(|paint| paint.layer == popup.id))
        {
            popup.anchor = transform.rect(popup.anchor);
            popup.rect = transform.rect(popup.rect);
        }
        for paint in &mut p.paints {
            wrap(&mut paint.paint);
            paint.clip = transform
                .rect(paint.clip)
                .intersect(if paint.layer == p.window {
                    clip
                } else {
                    viewport
                });
            if let Some(radius) = &mut paint.blur {
                *radius *= transform.scale;
            }
        }
        for paint in &mut self.scrolling.pending[p.paints_range.clone()] {
            if paint.layer != p.window {
                continue;
            }
            wrap(&mut paint.paint);
            paint.clip = transform.rect(paint.clip).intersect(clip);
            if let Some(radius) = &mut paint.blur {
                *radius *= transform.scale;
            }
        }
        let mut map_hit = |hit: &mut HitRegion| {
            let previous = self
                .current_transforms
                .get(&hit.id)
                .copied()
                .unwrap_or_default();
            self.current_transforms
                .insert(hit.id, transform.compose(previous));
            hit.rect = transform.rect(hit.rect);
            hit.clip = transform
                .rect(hit.clip)
                .intersect(if hit.window == p.window {
                    clip
                } else {
                    viewport
                });
            if !interactive {
                hit.action = super::HitAction::Block;
                hit.clip = Rect::default();
            }
        };
        for (hit, _) in &mut p.hits {
            map_hit(hit);
        }
        for (hit, _) in &mut self.scrolling.hits[p.hits_range.clone()] {
            map_hit(hit);
        }
        for scope in &mut self.scrolling.scopes[p.scopes_range.clone()] {
            if scope.window != p.window {
                continue;
            }
            scope.viewport = transform.rect(scope.viewport);
            scope.region = transform.rect(scope.region);
            scope.outer_clip = transform.rect(scope.outer_clip).intersect(clip);
            scope.origin = transform.point(scope.origin);
            scope.correction *= transform.scale;
            scope.visual_scale *= transform.scale;
            if !interactive {
                if let Some(state) = self.scrolling.states.get_mut(&scope.id) {
                    state.enabled = false;
                }
            }
        }
        if let Some((_, rect)) = &mut p.ime {
            *rect = transform.rect(*rect).intersect(clip);
        }
        self.place(p, Vec2::ZERO, clip);
    }
    pub(crate) fn begin_placement(&mut self, window: Id) {
        self.placements.outstanding += 1;
        self.placements.stack.push(Placement {
            animation_start: self.animations.observation(),
            popup_start: self.popup_layers.len(),
            window,
            paints: Vec::new(),
            hits: Vec::new(),
            paints_range: self.scrolling.pending.len()..0,
            hits_range: self.scrolling.hits.len()..0,
            scopes_range: self.scrolling.scopes.len()..0,
            ime: None,
            targets: Vec::new(),
        });
    }
    pub(crate) fn end_placement(&mut self) -> Placement {
        let mut p = self.placements.stack.pop().unwrap();
        p.paints_range.end = self.scrolling.pending.len();
        p.hits_range.end = self.scrolling.hits.len();
        p.scopes_range.end = self.scrolling.scopes.len();
        p
    }
    pub(crate) fn defer_placement_paint(
        &mut self,
        id: Id,
        layer: Id,
        clip: Rect,
        paint: Vec<Paint>,
    ) -> Result<(), Vec<Paint>> {
        let scope = self.scrolling.owner(layer);
        if let Some(p) =
            self.placements.stack.last_mut().filter(|p| {
                p.window == layer || self.popup_layers[p.popup_start..].contains(&layer)
            })
        {
            p.paints.push(PlacedPaint {
                id,
                layer,
                clip,
                paint,
                blur: None,
                scope,
            });
            Ok(())
        } else {
            Err(paint)
        }
    }
    pub(crate) fn defer_placement_hit(&mut self, hit: HitRegion) -> bool {
        let scope = self.scrolling.owner(hit.window);
        if let Some(p) = self.placements.stack.last_mut().filter(|p| {
            p.window == hit.window || self.popup_layers[p.popup_start..].contains(&hit.window)
        }) {
            p.hits.push((hit, scope));
            true
        } else {
            false
        }
    }
    pub(crate) fn defer_placement_ime(&mut self, window: Id, rect: Rect) -> bool {
        let scope = self.scrolling.owner(window);
        if let Some(p) =
            self.placements.stack.last_mut().filter(|p| {
                p.window == window || self.popup_layers[p.popup_start..].contains(&window)
            })
        {
            p.ime = Some((scope, rect));
            true
        } else {
            false
        }
    }
    pub(crate) fn record_placement_target(&mut self, window: Id, scope: usize, target: Rect) {
        if let Some(p) = self
            .placements
            .stack
            .last_mut()
            .filter(|p| p.window == window)
        {
            p.targets.push((scope, target));
        }
    }
    pub(crate) fn place(&mut self, p: Placement, delta: Vec2, clip: Rect) {
        let viewport = self.viewport();
        // Portals follow their anchor, then fit the viewport independently of the row clip.
        let portal = self
            .popup
            .as_mut()
            .filter(|popup| {
                delta != Vec2::ZERO && p.paints.iter().any(|paint| paint.layer == popup.id)
            })
            .map(|popup| {
                let old = popup.rect;
                let gap = if old.min.y >= popup.anchor.max.y {
                    old.min.y - popup.anchor.max.y
                } else {
                    popup.anchor.min.y - old.max.y
                };
                popup.anchor = popup.anchor.translate(delta);
                popup.rect = crate::components::popup::place(
                    popup.anchor,
                    old.size(),
                    viewport,
                    gap.max(0.0),
                )
                .0;
                (popup.id, popup.rect.min - old.min, popup.key_target)
            });
        let shift = |window| {
            portal
                .filter(|(id, _, _)| *id == window)
                .map_or(delta, |(_, shift, _)| shift)
        };
        for pending in &mut self.scrolling.pending[p.paints_range] {
            if pending.layer != p.window && portal.is_none_or(|(id, _, _)| id != pending.layer) {
                continue;
            }
            let delta = shift(pending.layer);
            pending.clip = pending.clip.translate(delta);
            // Descendant scroll scopes clip at their viewport, not at content origin.
            if pending.scope < p.scopes_range.start {
                pending.clip = pending.clip.intersect(if pending.layer == p.window {
                    clip
                } else {
                    viewport
                });
            }
            for paint in &mut pending.paint {
                paint.translate(delta);
            }
        }
        for (hit, scope) in &mut self.scrolling.hits[p.hits_range] {
            if hit.window != p.window && portal.is_none_or(|(id, _, _)| id != hit.window) {
                continue;
            }
            let delta = shift(hit.window);
            hit.rect = hit.rect.translate(delta);
            hit.clip = hit.clip.translate(delta);
            if *scope < p.scopes_range.start {
                hit.clip = hit.clip.intersect(if hit.window == p.window {
                    clip
                } else {
                    viewport
                });
            }
        }
        for n in p.scopes_range.clone() {
            let s = &mut self.scrolling.scopes[n];
            if s.window != p.window && portal.is_none_or(|(id, _, _)| id != s.window) {
                continue;
            }
            let delta = shift(s.window);
            s.viewport = s.viewport.translate(delta);
            s.region = s.region.translate(delta);
            s.outer_clip = s.outer_clip.translate(delta);
            if s.parent.is_none_or(|parent| parent < p.scopes_range.start) {
                s.outer_clip =
                    s.outer_clip
                        .intersect(if s.window == p.window { clip } else { viewport });
            }
            s.origin += delta;
        }
        if let Some((scope, rect)) = p.ime {
            let window = scope.map_or(p.window, |scope| self.scrolling.scopes[scope].window);
            let delta = shift(window);
            let mut rect = rect.translate(delta);
            if scope.is_none_or(|scope| scope < p.scopes_range.start) {
                rect = rect.intersect(if window == p.window { clip } else { viewport });
            }
            if let Some(parent) = self
                .placements
                .stack
                .last_mut()
                .filter(|parent| parent.window == p.window)
            {
                parent.ime = Some((scope, rect));
            } else if let Some(scope) = scope {
                self.scrolling.ime = Some((scope, rect));
            } else {
                self.ime_area = (!rect.is_empty()).then_some(rect);
            }
        }
        for (scope, target) in p.targets {
            let target = if scope < p.scopes_range.start {
                target.translate(delta)
            } else {
                target
            };
            self.scrolling.scopes[scope].target = Some(target);
            self.record_placement_target(p.window, scope, target);
        }
        for mut paint in p.paints {
            let delta = shift(paint.layer);
            if delta != Vec2::ZERO && !p.hits.iter().any(|(hit, _)| hit.id == paint.id) {
                self.current_transforms
                    .entry(paint.id)
                    .or_default()
                    .translation += delta;
            }
            for primitive in &mut paint.paint {
                primitive.translate(delta);
            }
            paint.clip = paint.clip.translate(delta);
            if paint.scope.is_none_or(|scope| scope < p.scopes_range.start) {
                paint.clip = paint.clip.intersect(if paint.layer == p.window {
                    clip
                } else {
                    viewport
                });
            }
            if let Some(parent) = self.placements.stack.last_mut().filter(|parent| {
                parent.window == paint.layer
                    || self.popup_layers[parent.popup_start..].contains(&paint.layer)
            }) {
                parent.paints.push(paint);
            } else if let Some(scope) = paint.scope {
                self.record_paint_order(paint.id);
                self.scrolling.pending.push(super::scroll::PendingPaint {
                    id: paint.id,
                    layer: paint.layer,
                    clip: paint.clip,
                    paint: paint.paint,
                    blur: paint.blur,
                    scope,
                });
            } else {
                self.paint(paint.id, paint.layer, paint.clip, paint.paint);
                if let Some(radius) = paint.blur {
                    self.mark_blur(paint.id, radius);
                }
            }
        }
        let mut translated_hits = std::collections::HashSet::new();
        for (mut hit, scope) in p.hits {
            let delta = if let Some((id, portal_shift, target)) =
                portal.filter(|(id, _, _)| *id == hit.window)
            {
                if hit.id == id.with("block") {
                    Vec2::ZERO
                } else if Some(hit.id) == target {
                    delta
                } else {
                    portal_shift
                }
            } else {
                delta
            };
            if delta != Vec2::ZERO && translated_hits.insert(hit.id) {
                self.current_transforms
                    .entry(hit.id)
                    .or_default()
                    .translation += delta;
            }
            hit.rect = hit.rect.translate(delta);
            hit.clip = hit.clip.translate(delta);
            if scope.is_none_or(|scope| scope < p.scopes_range.start) {
                hit.clip = hit.clip.intersect(if hit.window == p.window {
                    clip
                } else {
                    viewport
                });
            }
            if let Some(parent) = self.placements.stack.last_mut().filter(|parent| {
                parent.window == hit.window
                    || self.popup_layers[parent.popup_start..].contains(&hit.window)
            }) {
                parent.hits.push((hit, scope));
            } else if let Some(scope) = scope {
                self.scrolling.hits.push((hit, scope));
            } else {
                self.register_hit(hit);
            }
        }
        self.placements.outstanding -= 1;
        self.flush_scroll();
    }
}
