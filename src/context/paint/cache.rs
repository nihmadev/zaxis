//! Whether a description can reuse its cached mesh: unchanged, moved by a translation that
//! keeps its raster, or off-screen. Anything else is tessellated again.

use super::{state::PaintState, CachedElement, Paint};
use crate::{
    context::Id,
    shapes::Mesh,
    text::{TextSystem, DEFAULT_TAB},
    Rect, Vec2,
};
use std::sync::Arc;

/// What to do with a description that has a cache entry.
pub(super) enum Reuse {
    /// Its geometry misses the clip: the entry stays alive and nothing is drawn.
    Hidden,
    /// The cached mesh, moved by this delta (zero when nothing changed) without
    /// changing any glyph raster.
    Translated(Vec2),
    /// The same description at the same scale: the cached mesh as it is.
    Unchanged,
    Rebuild,
}

impl PaintState {
    /// Decide how `paint` reuses the cache entry of `id`. A pure translation keeps contours
    /// and glyph UVs intact, including DPI and subpixel raster keys; text translations are
    /// reusable only at whole physical pixels, so fractional touchpad motion still
    /// rasterizes correctly.
    pub(super) fn reuse(
        &self,
        id: Id,
        paint: &[Paint],
        clip: Rect,
        paint_scale: f32,
        scale: f32,
        text: &mut TextSystem,
    ) -> Reuse {
        let Some(cached) = self.cache.get(&id).filter(|c| c.scale == paint_scale) else {
            return Reuse::Rebuild;
        };
        let moved_out = |delta: Vec2, margin: Vec2| {
            cached.bounds.is_some_and(|bounds| {
                let bounds = bounds.translate(delta);
                Rect::from_min_max(bounds.min - margin, bounds.max + margin)
                    .intersect(clip)
                    .is_empty()
            })
        };
        match translation(&cached.paint, paint, scale) {
            Some(delta) if moved_out(delta, Vec2::ZERO) => Reuse::Hidden,
            // The raster changed; a translated description never equals the cached one.
            Some(delta)
                if delta != Vec2::ZERO && !preserves_raster(text, &cached.paint, delta, scale) =>
            {
                Reuse::Rebuild
            }
            Some(delta) => Reuse::Translated(delta),
            // Off the pixel grid but off-screen as well: nothing to rasterize.
            None if translation(&cached.paint, paint, 0.0)
                .is_some_and(|delta| moved_out(delta, Vec2::splat(1.0 / scale))) =>
            {
                Reuse::Hidden
            }
            None if cached.paint == paint => Reuse::Unchanged,
            None => Reuse::Rebuild,
        }
    }

    /// Keep the cache entry of `id` for this pass.
    pub(super) fn touch(&mut self, id: Id, frame: u64) {
        if let Some(cached) = self.cache.get_mut(&id) {
            cached.last_frame = frame;
        }
    }

    /// Move the cached mesh of `id` to `paint`, which is its translation by `delta`.
    pub(super) fn translate(&mut self, id: Id, delta: Vec2, paint: Vec<Paint>, frame: u64) {
        let cached = self.cache.get_mut(&id).expect("translated paint is cached");
        cached.last_frame = frame;
        if delta == Vec2::ZERO {
            return;
        }
        for vertex in &mut Arc::make_mut(&mut cached.mesh).vertices {
            vertex.position[0] += delta.x;
            vertex.position[1] += delta.y;
        }
        cached.bounds = cached.bounds.map(|bounds| bounds.translate(delta));
        cached.paint = paint;
        self.modified.insert(id);
    }

    /// Cache a freshly tessellated mesh for `id`.
    pub(super) fn store(&mut self, id: Id, paint: Vec<Paint>, scale: f32, mesh: Mesh, frame: u64) {
        let bounds = mesh_bounds(&mesh);
        self.cache.insert(
            id,
            CachedElement {
                paint,
                scale,
                mesh: Arc::new(mesh),
                bounds,
                last_frame: frame,
            },
        );
        self.modified.insert(id);
    }
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

/// The delta that turns `old` into `new`, if `new` is exactly its translation. With a
/// positive `scale`, text only translates by whole physical pixels.
fn translation(old: &[Paint], new: &[Paint], scale: f32) -> Option<Vec2> {
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
                weight: aweight,
                wrap_width: aw,
                color: ac,
            },
            Paint::Text {
                text: bt,
                position: bp,
                size: bz,
                weight: bweight,
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
            if at != bt
                || *ap + delta != *bp
                || az != bz
                || aweight != bweight
                || aw != bw
                || ac != bc
            {
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

/// Whether moving the text of `paint` by `delta` keeps every glyph's raster key. Integer
/// physical motion normally does; floating-point addition near a subpixel bin boundary
/// may not.
fn preserves_raster(text: &mut TextSystem, paint: &[Paint], delta: Vec2, scale: f32) -> bool {
    paint.iter().all(|primitive| match primitive {
        Paint::Text {
            text: string,
            position,
            size,
            weight,
            wrap_width,
            ..
        } => text.translation_preserves_raster(
            string,
            *position,
            *size,
            *weight,
            *wrap_width,
            DEFAULT_TAB,
            delta,
            scale,
        ),
        Paint::Paragraph {
            text: string,
            position,
            size,
            font,
            wrap_width,
            tab,
            ..
        } => text.translation_preserves_raster(
            string,
            *position,
            *size,
            *font,
            *wrap_width,
            *tab,
            delta,
            scale,
        ),
        Paint::Rich {
            text: string,
            runs,
            position,
            size,
            font,
            wrap_width,
            tab,
            ..
        } => text.rich_translation_preserves_raster(
            string,
            runs,
            *position,
            *size,
            *font,
            *wrap_width,
            *tab,
            delta,
            scale,
        ),
        _ => true,
    })
}
