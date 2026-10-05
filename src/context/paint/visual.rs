//! Visual transforms around a whole element: the content is painted untransformed and
//! cached as usual, then a transformed copy with its opacity is materialized from it.

use super::{state::PaintState, Paint};
use crate::{
    context::{Context, Id},
    shapes::Mesh,
    Rect, Transform, Vec2,
};
use std::sync::Arc;

/// The transformed copy of an element's cached mesh, valid while that mesh, the transform
/// and the opacity stay the same.
pub struct VisualMesh {
    base: Arc<Mesh>,
    transform: Transform,
    opacity: f32,
    mesh: Arc<Mesh>,
}

/// Nested visuals around one element, composed into one.
pub(super) struct Visual {
    pub paint: Vec<Paint>,
    pub transform: Transform,
    pub opacity: f32,
}

/// Unwrap `paint` when it is a visual around the whole element; anything else comes back.
pub(super) fn flatten(paint: Vec<Paint>) -> Result<Visual, Vec<Paint>> {
    if !matches!(paint.as_slice(), [Paint::Visual { .. }]) {
        return Err(paint);
    }
    let mut paint = paint;
    let mut transform = Transform::IDENTITY;
    let mut opacity = 1.0;
    while matches!(paint.as_slice(), [Paint::Visual { .. }]) {
        let Some(Paint::Visual {
            paint: inner,
            transform: t,
            opacity: alpha,
        }) = paint.pop()
        else {
            unreachable!()
        };
        transform = transform.compose(t);
        opacity *= alpha;
        paint = inner;
    }
    Ok(Visual {
        paint,
        transform,
        opacity,
    })
}

impl Context {
    /// Paint the content of `visual` in its own coordinates (clip mapped back, images
    /// requested at the displayed scale), then wrap the resulting element.
    pub(super) fn paint_visual(&mut self, id: Id, layer: Id, clip: Rect, visual: Visual) {
        let Visual {
            paint,
            transform,
            opacity,
        } = visual;
        self.paint_state.materializing = true;
        let previous_scale = self.paint_state.image_visual_scale;
        self.paint_state.image_visual_scale *= transform.scale;
        self.paint(id, layer, transform.inverse().rect(clip), paint);
        self.paint_state.image_visual_scale = previous_scale;
        self.paint_state.materializing = false;
        self.paint_state.materialize(id, clip, transform, opacity);
    }
}

impl PaintState {
    /// Replace the mesh of the element just painted under `id` by its transformed copy,
    /// reusing the copy while base mesh, transform and opacity are unchanged.
    fn materialize(&mut self, id: Id, clip: Rect, transform: Transform, opacity: f32) {
        let Some(element) = self.elements.last_mut().filter(|element| element.id == id) else {
            return;
        };
        element.clip = clip;
        let base = &element.mesh;
        let reusable = self.visual_meshes.get(&id).is_some_and(|v| {
            Arc::ptr_eq(&v.base, base) && v.transform == transform && v.opacity == opacity
        });
        if !reusable {
            let mut mesh = (**base).clone();
            for vertex in &mut mesh.vertices {
                vertex.position = transform
                    .point(Vec2::from_array(vertex.position))
                    .to_array();
                // Colors in the protocol are straight alpha; shader premultiplies.
                vertex.color[3] *= opacity;
            }
            self.visual_meshes.insert(
                id,
                VisualMesh {
                    base: Arc::clone(base),
                    transform,
                    opacity,
                    mesh: Arc::new(mesh),
                },
            );
            self.modified.insert(id);
        }
        element.mesh = Arc::clone(&self.visual_meshes[&id].mesh);
    }
}
