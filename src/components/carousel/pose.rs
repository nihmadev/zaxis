//! Where a layer is for a signed distance `d` (in pages) from the carousel position.
//! `d = 0` is the front layer, `d > 0` layers behind it, `d < 0` layers that have left.
use crate::{Rect, Transform, Vec2};

#[derive(Clone, Copy, Debug)]
pub(super) struct Pose {
    pub(super) transform: Transform,
    pub(super) opacity: f32,
    /// Opacity of the page content alone; sheets show their content only as they rise.
    pub(super) content: f32,
    /// Black overlay strength, 0..1.
    pub(super) dim: f32,
    /// Backdrop blur radius painted over the layer.
    pub(super) blur: f32,
}

/// A layer this close to the front is at rest: its pose is exactly the identity, so the end
/// of a spring does not leave text resampled at a hair's difference from its rest geometry.
fn rest(d: f32) -> f32 {
    if d.abs() < 0.002 {
        0.0
    } else {
        d
    }
}

/// Scale and rotate about `center`, then move by `offset`.
pub(super) fn about(center: Vec2, scale: f32, angle: f32, offset: Vec2) -> Transform {
    let mut t = Transform {
        scale,
        translation: Vec2::ZERO,
        angle,
    };
    t.translation = center + offset - t.vector(center);
    t
}

pub(super) struct StackGeometry {
    /// The front card at rest.
    pub(super) card: Rect,
    pub(super) direction: Vec2,
    pub(super) step: f32,
    pub(super) layers: usize,
    pub(super) scale_step: f32,
    pub(super) fade_step: f32,
    pub(super) exit: Vec2,
    pub(super) exit_rotation: f32,
}

/// Stack layer pose; `None` when the layer is not visible.
pub(super) fn stack_pose(g: &StackGeometry, d: f32) -> Option<Pose> {
    let d = rest(d);
    let size = g.card.size();
    let center = g.card.center();
    if d >= 0.0 {
        let layers = g.layers as f32;
        if d >= layers {
            return None;
        }
        let scale = (1.0 - g.scale_step * d).max(0.5);
        // The far edge of the sheet sits exactly `step * d` past the front card's edge.
        let extent = g.direction.abs().dot(size);
        let offset = g.direction * (extent * 0.5 * (1.0 - scale) + g.step * d);
        Some(Pose {
            transform: about(center, scale, 0.0, offset),
            opacity: (1.0 - g.fade_step * d).clamp(0.0, 1.0) * (layers - d).clamp(0.0, 1.0),
            content: (1.0 - d).clamp(0.0, 1.0),
            dim: 0.0,
            blur: 0.0,
        })
    } else {
        let gone = -d;
        if gone >= 1.0 {
            return None;
        }
        Some(Pose {
            transform: about(center, 1.0, g.exit_rotation * gone, g.exit * size * gone),
            opacity: 1.0 - gone,
            content: 1.0,
            dim: 0.0,
            blur: 0.0,
        })
    }
}

pub(super) struct SlideGeometry {
    /// The central slide at rest.
    pub(super) slide: Rect,
    /// Axis the slides are lined up along (0 horizontal, 1 vertical).
    pub(super) axis: usize,
    pub(super) neighbors: usize,
    pub(super) peek: f32,
    pub(super) scale_step: f32,
    pub(super) drop: f32,
    pub(super) dim: f32,
    pub(super) blur: f32,
}

/// Slide pose; `None` when the slide is further than the visible neighbours.
pub(super) fn slide_pose(g: &SlideGeometry, d: f32) -> Option<Pose> {
    let d = rest(d);
    let distance = d.abs();
    let reach = g.neighbors as f32 + 1.0;
    if distance >= reach {
        return None;
    }
    let scale = (1.0 - g.scale_step * distance).max(0.4);
    // Each neighbour's outer edge shows `peek` more than the nearer one's.
    let main = (g.slide.size()[g.axis] * 0.5 * (1.0 - scale) + g.peek * distance).copysign(d);
    let mut offset = Vec2::splat(g.drop * distance);
    offset[g.axis] = main;
    Some(Pose {
        transform: about(g.slide.center(), scale, 0.0, offset),
        opacity: (reach - distance).clamp(0.0, 1.0),
        content: 1.0,
        dim: (g.dim * distance).min(0.85),
        blur: g.blur * distance.min(1.0),
    })
}

/// Paint order of layers, farthest first; a layer that has left is above the rest.
pub(super) fn paint_order(a: f32, b: f32) -> std::cmp::Ordering {
    let rank = |d: f32| (d < 0.0, if d < 0.0 { d } else { -d });
    let (ra, rb) = (rank(a), rank(b));
    ra.0.cmp(&rb.0).then(ra.1.total_cmp(&rb.1))
}

/// Paint order of slides, farthest from the position first.
pub(super) fn depth_order(a: f32, b: f32) -> std::cmp::Ordering {
    b.abs().total_cmp(&a.abs())
}
