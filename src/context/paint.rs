//! Paint descriptions, tessellation, and per-element mesh caching.

use super::{geometry::Element, Context, Id};
use crate::{shapes::Mesh, Color, CornerRadius, Rect, Shape, Vec2};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Paint {
    Visual {
        paint: Vec<Paint>,
        transform: crate::Transform,
        opacity: f32,
    },
    Shape(Shape),
    Image {
        rect: Rect,
        uv: Rect,
        rounding: CornerRadius,
        color: Color,
        opacity: f32,
        handle: crate::ImageHandle,
        texture: crate::TextureId,
        hidden: bool,
    },
    ScrollHint {
        rect: Rect,
        axis: usize,
        color: Color,
    },
    Gradient {
        rect: Rect,
        rounding: CornerRadius,
        colors: Vec<Color>,
        columns: usize,
        rows: usize,
    },
    Text {
        text: String,
        position: Vec2,
        size: f32,
        wrap_width: f32,
        color: Color,
    },
}

pub(super) struct CachedElement {
    pub(super) paint: Vec<Paint>,
    pub(super) scale: f32,
    pub(super) mesh: Arc<Mesh>,
    pub(super) bounds: Option<Rect>,
    pub(super) last_frame: u64,
}
pub(crate) struct VisualMesh {
    base: Arc<Mesh>,
    transform: crate::Transform,
    opacity: f32,
    mesh: Arc<Mesh>,
}

impl Context {
    pub(crate) fn record_paint_order(&mut self, id: Id) {
        let order = self.paint_order.len();
        self.paint_order.entry(id).or_insert(order);
    }
    pub(crate) fn centered_line_offset(&mut self, text: &str, size: f32) -> f32 {
        self.text.centered_line_offset(text, size)
    }

    pub(crate) fn text_carets(&mut self, text: &str, size: f32) -> Vec<(usize, f32)> {
        self.text.carets(text, size)
    }
    /// Paint a cached shape behind all panels. Call before building windows.
    /// This provides a backdrop for translucent panels and their blur effects.
    pub fn paint_background(&mut self, shape: impl Into<Shape>) {
        let id = Id::new(("background", self.elements.len()));
        self.paint(
            id,
            Id::new("background-layer"),
            self.viewport(),
            vec![Paint::Shape(shape.into())],
        );
    }

    pub(crate) fn paint_blur(&mut self, id: Id, layer: Id, clip: Rect, blur: crate::Blur) {
        if blur.radius <= 0.0 || blur.rect.is_empty() {
            return;
        }
        self.paint(
            id,
            layer,
            clip,
            vec![Paint::Shape(Shape::Rect {
                rect: blur.rect,
                fill: Color::WHITE,
                rounding: blur.rounding,
                border: crate::Border::NONE,
            })],
        );
        self.mark_blur(id, blur.radius);
    }

    pub(crate) fn mark_blur(&mut self, id: Id, radius: f32) {
        if let Some(paint) = self
            .placements
            .stack
            .last_mut()
            .and_then(|p| p.paints.last_mut())
            .filter(|p| p.id == id)
        {
            paint.blur = Some(radius);
        } else if let Some(paint) = self.scrolling.pending.last_mut().filter(|p| p.id == id) {
            paint.blur = Some(radius);
        } else {
            if let Some(element) = self.elements.last_mut().filter(|e| e.id == id) {
                element.blur = Some(radius);
            }
        }
    }

    pub(crate) fn paint(&mut self, id: Id, layer: Id, clip: Rect, paint: Vec<Paint>) {
        let paint = match self.defer_placement_paint(id, layer, clip, paint) {
            Ok(()) => return,
            Err(paint) => paint,
        };
        self.record_paint_order(id);
        let paint = match self.defer_scroll_paint(id, layer, clip, paint) {
            Ok(()) => return,
            Err(paint) => paint,
        };
        if matches!(paint.as_slice(), [Paint::Visual { .. }]) {
            let mut paint = paint;
            let mut transform = crate::Transform::IDENTITY;
            let mut opacity = 1.0;
            while matches!(paint.as_slice(), [Paint::Visual { .. }]) {
                let Paint::Visual {
                    paint: inner,
                    transform: t,
                    opacity: alpha,
                } = paint.pop().unwrap()
                else {
                    unreachable!()
                };
                transform = transform.compose(t);
                opacity *= alpha;
                paint = inner;
            }
            self.visual_materializing = true;
            let previous_scale = self.image_visual_scale;
            self.image_visual_scale *= transform.scale;
            self.paint(id, layer, transform.inverse().rect(clip), paint);
            self.image_visual_scale = previous_scale;
            self.visual_materializing = false;
            if let Some(element) = self.elements.last_mut().filter(|element| element.id == id) {
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
            return;
        }
        if !self.visual_materializing && self.visual_meshes.remove(&id).is_some() {
            self.modified.insert(id);
        }
        assert!(
            self.seen.insert(id),
            "duplicate widget/paint ID: use Ui::push_id or an explicit widget ID"
        );
        for primitive in &paint {
            if let Paint::Image {
                rect,
                uv,
                handle,
                texture,
                ..
            } = primitive
            {
                if !rect.intersect(clip).is_empty() {
                    self.images.request(
                        *handle,
                        rect.size() / uv.size().max(Vec2::splat(0.001))
                            * self.scale
                            * self.image_visual_scale,
                        *texture,
                    );
                }
            }
        }
        let paint_scale = if paint.iter().any(|p| matches!(p, Paint::Image { .. })) {
            self.scale * self.image_visual_scale
        } else {
            self.scale
        };
        // A pure translation keeps contours and glyph UVs intact, including DPI
        // and subpixel raster keys. Text translations are reusable only at whole
        // physical pixels, so fractional touchpad motion still rasterizes correctly.
        let mut translation = self.cache.get(&id).and_then(|cached| {
            if cached.scale != paint_scale {
                return None;
            }
            paint_translation(&cached.paint, &paint, self.scale)
        });
        if translation.is_none() {
            if let Some(cached) = self.cache.get_mut(&id) {
                if cached.scale == paint_scale {
                    if let Some(delta) = paint_translation(&cached.paint, &paint, 0.0) {
                        if cached.bounds.is_some_and(|bounds| {
                            let bounds = bounds.translate(delta);
                            let margin = Vec2::splat(1.0 / self.scale);
                            Rect::from_min_max(bounds.min - margin, bounds.max + margin)
                                .intersect(clip)
                                .is_empty()
                        }) {
                            cached.last_frame = self.frame;
                            return;
                        }
                    }
                }
            }
        }
        if let Some(delta) = translation.filter(|delta| *delta != Vec2::ZERO) {
            let cached = &self.cache[&id];
            if cached
                .bounds
                .is_some_and(|bounds| bounds.translate(delta).intersect(clip).is_empty())
            {
                self.cache.get_mut(&id).unwrap().last_frame = self.frame;
                return;
            }
            if !cached.paint.iter().all(|primitive| match primitive {
                Paint::Text {
                    text,
                    position,
                    size,
                    wrap_width,
                    ..
                } => self.text.translation_preserves_raster(
                    text,
                    *position,
                    *size,
                    *wrap_width,
                    delta,
                    self.scale,
                ),
                _ => true,
            }) {
                translation = None;
            }
        }
        if let Some(delta) = translation {
            let cached = self.cache.get_mut(&id).unwrap();
            cached.last_frame = self.frame;
            if cached
                .bounds
                .is_some_and(|bounds| bounds.translate(delta).intersect(clip).is_empty())
            {
                return;
            }
            if delta != Vec2::ZERO {
                for vertex in &mut Arc::make_mut(&mut cached.mesh).vertices {
                    vertex.position[0] += delta.x;
                    vertex.position[1] += delta.y;
                }
                cached.bounds = cached.bounds.map(|bounds| bounds.translate(delta));
                cached.paint = paint;
                self.modified.insert(id);
            }
            self.stats.reused_elements += 1;
            self.elements.push(Element {
                id,
                layer,
                clip,
                mesh: Arc::clone(&cached.mesh),
                blur: None,
                scroll_hint: paint_is_scroll_hint(&cached.paint),
            });
            return;
        }
        let reusable = self
            .cache
            .get(&id)
            .is_some_and(|cached| cached.paint == paint && cached.scale == paint_scale);
        if reusable {
            self.stats.reused_elements += 1;
            self.cache.get_mut(&id).unwrap().last_frame = self.frame;
        } else {
            let mut mesh = Mesh::default();
            for primitive in &paint {
                match primitive {
                    Paint::Visual { .. } => {
                        unreachable!("visual transforms must wrap the whole element")
                    }
                    Paint::ScrollHint { rect, axis, color } => {
                        let start = mesh.vertices.len();
                        mesh.quad(
                            *rect,
                            Rect::from_min_size(Vec2::ZERO, Vec2::ONE),
                            color.linear(),
                            crate::TextureId::WHITE,
                        );
                        if *axis == 0 {
                            for vertex in &mut mesh.vertices[start..] {
                                vertex.uv.swap(0, 1);
                            }
                        }
                    }
                    Paint::Shape(shape) => mesh.shape(shape, self.scale),
                    Paint::Image {
                        rect,
                        uv,
                        rounding,
                        color,
                        opacity,
                        texture,
                        hidden,
                        ..
                    } => {
                        if !hidden {
                            mesh.image(
                                *rect,
                                *uv,
                                *rounding,
                                *color,
                                *opacity,
                                *texture,
                                paint_scale,
                            );
                        }
                    }
                    Paint::Gradient {
                        rect,
                        rounding,
                        colors,
                        columns,
                        rows,
                    } => {
                        mesh.gradient(*rect, *rounding, colors, *columns, *rows, self.scale);
                    }
                    Paint::Text {
                        text,
                        position,
                        size,
                        wrap_width,
                        color,
                    } => self.text.paint(
                        &mut mesh,
                        text,
                        *position,
                        *size,
                        *wrap_width,
                        *color,
                        self.scale,
                    ),
                }
            }
            let bounds = mesh_bounds(&mesh);
            self.cache.insert(
                id,
                CachedElement {
                    paint,
                    scale: paint_scale,
                    mesh: Arc::new(mesh),
                    bounds,
                    last_frame: self.frame,
                },
            );
            self.modified.insert(id);
            self.stats.tessellated_elements += 1;
        }
        if self.cache[&id]
            .bounds
            .is_some_and(|bounds| bounds.intersect(clip).is_empty())
        {
            return;
        }
        let scroll_hint = paint_is_scroll_hint(&self.cache[&id].paint);
        self.elements.push(Element {
            id,
            layer,
            clip,
            mesh: Arc::clone(&self.cache[&id].mesh),
            blur: None,
            scroll_hint,
        });
    }

    pub(crate) fn measure_text(&mut self, text: &str, size: f32, wrap: f32) -> Vec2 {
        self.text.measure(text, size, wrap)
    }
}

fn paint_is_scroll_hint(paint: &[Paint]) -> bool {
    matches!(paint, [Paint::ScrollHint { .. }])
}

fn mesh_bounds(mesh: &Mesh) -> Option<Rect> {
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
fn paint_translation(old: &[Paint], new: &[Paint], scale: f32) -> Option<Vec2> {
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
    pub(super) fn translate(&mut self, delta: Vec2) {
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
