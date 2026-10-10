//! A visual effect around placed content: paints are wrapped in the transform and
//! opacity, hits and scroll scopes are mapped (and composed into the transforms input
//! reads), and content that is not interactive blocks input instead of receiving it.

use super::Placement;
use crate::{
    context::{Context, HitAction, HitRegion, MaterialUse, Paint},
    Rect, Transform, Vec2,
};

impl Context {
    /// Transform measured paint and routing together, before placement/scroll emission.
    pub(crate) fn place_visual(
        &mut self,
        mut p: Placement,
        transform: Transform,
        opacity: f32,
        clip: Rect,
        interactive: bool,
    ) {
        if opacity <= 0.0 || clip.is_empty() {
            self.animations.hide_since(p.animation_start);
        }
        let viewport = self.viewport();
        let portal = self.transform_portals(&p, transform);
        let wrap = |paint: &mut Vec<Paint>,
                    blur: &mut Option<f32>,
                    material: &mut Option<MaterialUse>,
                    transform: Transform| {
            *paint = vec![Paint::Visual {
                paint: std::mem::take(paint),
                transform,
                opacity,
            }];
            if let Some(sigma) = blur {
                *sigma *= transform.scale;
            }
            if let Some(material) = material {
                material.scale(transform.scale);
            }
        };
        let outer = p.outer_clip(clip, viewport);
        for paint in &mut p.paints {
            let mapping = portal.mapping(paint.layer, paint.id, transform);
            wrap(
                &mut paint.paint,
                &mut paint.blur,
                &mut paint.material,
                mapping,
            );
            paint.clip = mapping.rect(paint.clip).intersect(outer(paint.layer));
        }
        for paint in &mut self.scrolling.pending[p.paints_range.clone()] {
            if !portal.moves(paint.layer, p.window) {
                continue;
            }
            let mapping = portal.mapping(paint.layer, paint.id, transform);
            wrap(
                &mut paint.paint,
                &mut paint.blur,
                &mut paint.material,
                mapping,
            );
            paint.clip = mapping.rect(paint.clip).intersect(outer(paint.layer));
        }
        let mut map_hit = |hit: &mut HitRegion| {
            let mapping = portal.mapping(hit.window, hit.id, transform);
            self.visuals.compose(hit.id, mapping);
            hit.rect = mapping.rect(hit.rect);
            hit.clip = mapping.rect(hit.clip).intersect(outer(hit.window));
            block_hidden(hit, opacity, interactive);
        };
        for (hit, _) in &mut p.hits {
            map_hit(hit);
        }
        for (hit, _) in &mut self.scrolling.hits[p.hits_range.clone()] {
            map_hit(hit);
        }
        for scope in &mut self.scrolling.scopes[p.scopes_range.clone()] {
            if !portal.moves(scope.window, p.window) {
                continue;
            }
            let mapping = portal.mapping(scope.window, scope.id, transform);
            scope.viewport = mapping.rect(scope.viewport);
            scope.region = mapping.rect(scope.region);
            scope.outer_clip = mapping
                .rect(scope.outer_clip)
                .intersect(outer(scope.window));
            scope.origin = mapping.point(scope.origin);
            scope.correction *= mapping.scale;
            scope.visual_scale *= mapping.scale;
            if !interactive {
                if let Some(state) = self.scrolling.states.get_mut(&scope.id) {
                    state.enabled = false;
                }
            }
        }
        if let Some((window, _, rect)) = &mut p.ime {
            let mapping = portal.mapping(*window, *window, transform);
            *rect = mapping.rect(*rect).intersect(outer(*window));
        }
        self.place(p, Vec2::ZERO, clip);
    }
}

/// Content that is not interactive blocks input with no area left to block. A geometry
/// region of the accessibility tree stays while its content is shown, even inert; what
/// is not shown at all comes back blocked, which hides its node.
fn block_hidden(hit: &mut HitRegion, opacity: f32, interactive: bool) {
    if hit.action == HitAction::Semantic {
        if opacity <= 0.0 {
            hit.action = HitAction::Block;
        }
    } else if !interactive {
        hit.action = HitAction::Block;
        hit.clip = Rect::default();
    }
}
