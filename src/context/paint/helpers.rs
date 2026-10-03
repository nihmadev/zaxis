use super::*;
pub(super) fn paint_is_scroll_hint(paint: &[Paint]) -> bool {
    matches!(paint, [Paint::ScrollHint { .. }])
}

pub(super) fn mesh_bounds(mesh: &Mesh) -> Option<Rect> {
    let first = mesh.vertices.first()?;
    let mut min = Vec2::from_array(first.position);
    let mut max = min;
    for vertex in &mesh.vertices[1..] {
        let p = Vec2::from_array(vertex.position);
        min = min.min(p);
        max = max.max(p);
    }
    Some(Rect::from_min_max(min, max))
}
pub(super) fn paint_translation(old: &[Paint], new: &[Paint], scale: f32) -> Option<Vec2> {
    if old.len() != new.len() {
        return None;
    }
    let delta = new.first()?.origin() - old.first()?.origin();
    if !delta.is_finite() {
        return None;
    }
    for (a, b) in old.iter().zip(new) {
        if let (
            Paint::Text {
                text: at,
                position: ap,
                size: az,
                wrap_width: aw,
                color: ac,
            },
            Paint::Text {
                text: bt,
                position: bp,
                size: bz,
                wrap_width: bw,
                color: bc,
            },
        ) = (a, b)
        {
            let physical = delta * scale;
            if scale > 0.0 && (physical.x != physical.x.round() || physical.y != physical.y.round())
            {
                return None;
            }
            if at != bt || *ap + delta != *bp || az != bz || aw != bw || ac != bc {
                return None;
            }
        } else {
            let mut translated = a.clone();
            translated.translate(delta);
            if translated != *b {
                return None;
            }
        }
    }
    Some(delta)
}
impl Paint {
    fn origin(&self) -> Vec2 {
        match self {
            Self::Visual {
                paint, transform, ..
            } => transform.point(paint.first().map_or(Vec2::ZERO, Paint::origin)),
            Self::Text { position, .. } => *position,
            Self::Image { rect, .. }
            | Self::ScrollHint { rect, .. }
            | Self::Gradient { rect, .. } => rect.min,
            Self::Shape(shape) => match shape {
                Shape::Rect { rect, .. }
                | Shape::Gradient { rect, .. }
                | Shape::Shadow { rect, .. } => rect.min,
                Shape::Circle { center, .. } => *center,
                Shape::Line { start, .. } => *start,
            },
        }
    }
    pub(crate) fn translate(&mut self, delta: Vec2) {
        match self {
            Self::Visual {
                paint, transform, ..
            } => {
                for primitive in paint {
                    primitive.translate(delta);
                }
                transform.translation += delta - transform.vector(delta);
            }
            Self::Text { position, .. } => *position += delta,
            Self::Image { rect, .. }
            | Self::ScrollHint { rect, .. }
            | Self::Gradient { rect, .. } => *rect = rect.translate(delta),
            Self::Shape(shape) => match shape {
                Shape::Rect { rect, .. }
                | Shape::Gradient { rect, .. }
                | Shape::Shadow { rect, .. } => *rect = rect.translate(delta),
                Shape::Circle { center, .. } => *center += delta,
                Shape::Line { start, end, .. } => {
                    *start += delta;
                    *end += delta;
                }
            },
        }
    }
}
