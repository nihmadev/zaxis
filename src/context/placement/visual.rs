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
        self.transform_portal(&p, transform);
        let wrap =
            |paint: &mut Vec<Paint>, blur: &mut Option<f32>, material: &mut Option<MaterialUse>| {
                *paint = vec![Paint::Visual {
                    paint: std::mem::take(paint),
                    transform,
                    opacity,
                }];
                if let Some(radius) = blur {
                    *radius *= transform.scale;
                }
                if let Some(material) = material {
                    material.scale(transform.scale);
                }
            };
        let outer = p.outer_clip(clip, viewport);
        for paint in &mut p.paints {
            wrap(&mut paint.paint, &mut paint.blur, &mut paint.material);
            paint.clip = transform.rect(paint.clip).intersect(outer(paint.layer));
        }
        for paint in &mut self.scrolling.pending[p.paints_range.clone()] {
            if paint.layer != p.window {
                continue;
            }
            wrap(&mut paint.paint, &mut paint.blur, &mut paint.material);
            paint.clip = transform.rect(paint.clip).intersect(clip);
        }
        let mut map_hit = |hit: &mut HitRegion| {
            self.visuals.compose(hit.id, transform);
            hit.rect = transform.rect(hit.rect);
            hit.clip = transform.rect(hit.clip).intersect(outer(hit.window));
            block_hidden(hit, opacity, interactive);
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
