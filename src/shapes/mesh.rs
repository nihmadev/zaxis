mod gradient;
mod tessellation;

use std::ops::Range;

use super::{Border, Color, Rect, TextureId, Vertex};
use crate::Vec2;

#[derive(Clone, Debug, Default)]
pub(crate) struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub batches: Vec<(Range<u32>, TextureId)>,
}

impl Mesh {
    /// Reuse the core rounded contour and coverage tessellation, with actual UVs.
    pub fn image(
        &mut self,
        rect: Rect,
        uv: Rect,
        rounding: super::CornerRadius,
        tint: Color,
        opacity: f32,
        texture: TextureId,
        scale: f32,
    ) {
        if rect.is_empty() {
            return;
        }
        let mut contour = Mesh::default();
        contour.shape(
            &super::Shape::Rect {
                rect,
                fill: tint,
                rounding,
                border: Border::NONE,
            },
            scale,
        );
        let base = self.vertices.len() as u32;
        let start = self.indices.len() as u32;
        for mut vertex in contour.vertices {
            let p = ((Vec2::from_array(vertex.position) - rect.min) / rect.size())
                .clamp(Vec2::ZERO, Vec2::ONE);
            vertex.uv = (uv.min + p * uv.size()).to_array();
            vertex.color[3] *= opacity;
            self.vertices.push(vertex);
        }
        self.indices
            .extend(contour.indices.iter().map(|i| i + base));
        self.batch(start, texture);
    }
    pub fn quad(&mut self, rect: Rect, uv: Rect, color: [f32; 4], texture: TextureId) {
        if rect.is_empty() {
            return;
        }
        let base = self.vertices.len() as u32;
        let start = self.indices.len() as u32;
        for (position, uv) in [
            (rect.min, uv.min),
            (
                Vec2::new(rect.max.x, rect.min.y),
                Vec2::new(uv.max.x, uv.min.y),
            ),
            (rect.max, uv.max),
            (
                Vec2::new(rect.min.x, rect.max.y),
                Vec2::new(uv.min.x, uv.max.y),
            ),
        ] {
            self.vertices.push(Vertex {
                position: position.to_array(),
                uv: uv.to_array(),
                color,
            });
        }
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        self.batch(start, texture);
    }

    fn batch(&mut self, start: u32, texture: TextureId) {
        let end = self.indices.len() as u32;
        if let Some((range, previous)) = self.batches.last_mut() {
            if *previous == texture && range.end == start {
                range.end = end;
                return;
            }
        }
        self.batches.push((start..end, texture));
    }

    fn polygon(&mut self, points: &[Vec2], color: [f32; 4]) {
        if points.len() < 3 || color[3] == 0.0 {
            return;
        }
        let start = self.indices.len() as u32;
        let base = self.vertices.len() as u32;
        self.vertices.extend(points.iter().map(|p| Vertex {
            position: p.to_array(),
            uv: [0.5; 2],
            color,
        }));
        for i in 1..points.len() - 1 {
            self.indices
                .extend([base, base + i as u32, base + i as u32 + 1]);
        }
        self.batch(start, TextureId::WHITE);
    }

    fn ring(
        &mut self,
        outer: &[Vec2],
        inner: &[Vec2],
        outer_color: [f32; 4],
        inner_color: [f32; 4],
    ) {
        let start = self.indices.len() as u32;
        let base = self.vertices.len() as u32;
        for (a, b) in outer.iter().zip(inner) {
            for (p, color) in [(a, outer_color), (b, inner_color)] {
                self.vertices.push(Vertex {
                    position: p.to_array(),
                    uv: [0.5; 2],
                    color,
                });
            }
        }
        for i in 0..outer.len() as u32 {
            let a = base + 2 * i;
            let b = base + 2 * ((i + 1) % outer.len() as u32);
            self.indices.extend([a, b, b + 1, a, b + 1, a + 1]);
        }
        self.batch(start, TextureId::WHITE);
    }

    /// Non-overlapping contour bands cover both edges of a border. Mixing in
    /// premultiplied linear color avoids halos and double blending translucent fills.
    fn antialiased_shape(
        &mut self,
        outline: impl Fn(f32) -> Vec<Vec2>,
        fill: Color,
        border: Border,
        max_inset: f32,
        scale: f32,
    ) {
        let width = if border.color.0[3] > 0 {
            border.width.max(0.0).min(max_inset)
        } else {
            0.0
        };
        if fill.0[3] == 0 && width == 0.0 {
            return;
        }
        let pixel = 1.0 / scale;
        let half_pixel = pixel * 0.5;
        let mut insets = vec![-half_pixel, half_pixel.min(max_inset)];
        if max_inset < half_pixel {
            insets.push(2.0 * max_inset - half_pixel);
        }
        if width > 0.0 {
            insets.extend([
                (width - half_pixel).clamp(-half_pixel, max_inset),
                (width + half_pixel).min(max_inset),
            ]);
            if max_inset - width < half_pixel {
                insets.push((2.0 * max_inset - width - half_pixel).min(max_inset));
            }
        }
        insets.sort_by(f32::total_cmp);
        insets.dedup();
        let fill = fill.linear();
        let stroke = border.color.linear();
        let color_at = |inset: f32| {
            // Subtract the opposite edge when a shape or its interior is
            // thinner than a pixel, preserving coverage instead of brightening it.
            let outer = (0.5 + inset / pixel).clamp(0.0, 1.0)
                - (0.5 + (inset - 2.0 * max_inset) / pixel).clamp(0.0, 1.0);
            let inner = if width == max_inset && width > 0.0 {
                // A border that consumes the shape has no interior edge.
                0.0
            } else {
                (0.5 + (inset - width) / pixel).clamp(0.0, 1.0)
                    - (0.5 + (inset - (2.0 * max_inset - width)) / pixel).clamp(0.0, 1.0)
            };
            let fill_alpha = inner * fill[3];
            let stroke_alpha = (outer - inner) * stroke[3];
            let alpha = fill_alpha + stroke_alpha;
            let mut color = [0.0, 0.0, 0.0, alpha];
            if alpha > 0.0 {
                for channel in 0..3 {
                    color[channel] =
                        (fill[channel] * fill_alpha + stroke[channel] * stroke_alpha) / alpha;
                }
            }
            color
        };
        let mut outer = outline(insets[0]);
        let mut outer_color = color_at(insets[0]);
        for &inset in &insets[1..] {
            let inner = outline(inset);
            let inner_color = color_at(inset);
            self.ring(&outer, &inner, outer_color, inner_color);
            outer = inner;
            outer_color = inner_color;
        }
        self.polygon(&outer, outer_color);
    }
}
