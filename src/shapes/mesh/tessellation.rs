use super::Mesh;
use crate::{
    shapes::outline::{arc_segments, rounded_outline},
    Border, Color, CornerRadius, GradientDirection, Rect, Shape, Vec2,
};

impl Mesh {
    pub fn shape(&mut self, shape: &Shape, scale: f32) {
        match *shape {
            Shape::Gradient {
                rect,
                rounding,
                gradient,
            } => {
                let a = gradient.start.linear();
                let b = gradient.end.linear();
                let start = self.vertices.len();
                self.gradient(rect, rounding, &[Color::WHITE; 4], 2, 2, scale);
                for vertex in &mut self.vertices[start..] {
                    let p = ((Vec2::from_array(vertex.position) - rect.min) / rect.size())
                        .clamp(Vec2::ZERO, Vec2::ONE);
                    let t = match gradient.direction {
                        GradientDirection::Horizontal => p.x,
                        GradientDirection::Vertical => p.y,
                        GradientDirection::Diagonal => (p.x + p.y) * 0.5,
                    };
                    let coverage = vertex.color[3];
                    vertex.color = std::array::from_fn(|i| a[i] * (1.0 - t) + b[i] * t);
                    vertex.color[3] *= coverage;
                }
            }
            Shape::Shadow {
                rect,
                rounding,
                shadow,
            } => {
                if rect.is_empty() || !shadow.offset.is_finite() || shadow.color.0[3] == 0 {
                    return;
                }
                let finite = |v: f32| {
                    if v.is_finite() {
                        v.clamp(0.0, 64.0)
                    } else {
                        0.0
                    }
                };
                let blur = finite(shadow.blur_radius);
                let spread = finite(shadow.spread);
                let radii = rounding.values(rect).map(|r| r + spread);
                let rect = Rect::from_min_max(
                    rect.min - Vec2::splat(spread),
                    rect.max + Vec2::splat(spread),
                )
                .translate(shadow.offset);
                if blur == 0.0 {
                    let rounding = CornerRadius {
                        top_left: radii[0],
                        top_right: radii[1],
                        bottom_right: radii[2],
                        bottom_left: radii[3],
                    };
                    self.shape(
                        &Shape::Rect {
                            rect,
                            rounding,
                            fill: shadow.color,
                            border: Border::NONE,
                        },
                        scale,
                    );
                    return;
                }
                let extent = blur * 3.0;
                let max_inset = rect.size().min_element() * 0.5;
                let inner = extent.min(max_inset);
                let segments = radii.map(|r| arc_segments((r + extent) * scale));
                let outline = |inset: f32| {
                    let bounds = if inset < 0.0 {
                        Rect::from_min_max(
                            rect.min + Vec2::splat(inset),
                            rect.max - Vec2::splat(inset),
                        )
                    } else {
                        rect.shrink(inset)
                    };
                    let r = radii.map(|r| (r - inset).max(0.0));
                    let r = CornerRadius {
                        top_left: r[0],
                        top_right: r[1],
                        bottom_right: r[2],
                        bottom_left: r[3],
                    }
                    .values(bounds);
                    rounded_outline(bounds, r, segments)
                };
                let color_at = |inset: f32| {
                    let mut color = shadow.color.linear();
                    color[3] *= (0.5 + 0.5 * (inset / blur).tanh()).clamp(0.0, 1.0);
                    color
                };
                let steps = ((extent + inner) * scale).ceil().clamp(12.0, 48.0) as usize;
                let mut outer = outline(-extent);
                let mut outer_color = shadow.color.linear();
                outer_color[3] = 0.0;
                for step in 1..=steps {
                    let inset = -extent + (extent + inner) * step as f32 / steps as f32;
                    let points = outline(inset);
                    let color = color_at(inset);
                    self.ring(&outer, &points, outer_color, color);
                    outer = points;
                    outer_color = color;
                }
                self.polygon(&outer, outer_color);
            }
            Shape::Rect {
                rect,
                fill,
                rounding,
                border,
            } => {
                if rect.is_empty() {
                    return;
                }
                let radii = rounding.values(rect);
                let half_pixel = 0.5 / scale;
                let segments = radii.map(|r| arc_segments((r + half_pixel) * scale));
                self.antialiased_shape(
                    |inset| {
                        let inset_rect = if inset >= 0.0 {
                            rect.shrink(inset)
                        } else {
                            Rect::from_min_max(
                                rect.min + Vec2::splat(inset),
                                rect.max - Vec2::splat(inset),
                            )
                        };
                        let [top_left, top_right, bottom_right, bottom_left] =
                            radii.map(|r| (r - inset).max(0.0));
                        let inset_radii = CornerRadius {
                            top_left,
                            top_right,
                            bottom_right,
                            bottom_left,
                        }
                        .values(inset_rect);
                        rounded_outline(inset_rect, inset_radii, segments)
                    },
                    fill,
                    border,
                    rect.size().min_element() * 0.5,
                    scale,
                );
            }
            Shape::Circle {
                center,
                radius,
                fill,
                border,
            } => {
                if radius <= 0.0 {
                    return;
                }
                let segments = arc_segments(radius * scale + 0.5) * 4;
                let outline = |inset: f32| {
                    let r = (radius - inset).max(0.0);
                    (0..segments)
                        .map(|i| {
                            let angle = i as f32 * std::f32::consts::TAU / segments as f32;
                            center + Vec2::new(angle.cos(), angle.sin()) * r
                        })
                        .collect::<Vec<_>>()
                };
                self.antialiased_shape(outline, fill, border, radius, scale);
            }
            Shape::Line {
                start,
                end,
                width,
                color,
            } => {
                let delta = end - start;
                if delta.length_squared() == 0.0 || width <= 0.0 {
                    return;
                }
                let tangent = delta.normalize();
                let normal = Vec2::new(-tangent.y, tangent.x);
                self.antialiased_shape(
                    |inset| {
                        let half_width = (width * 0.5 - inset).max(0.0);
                        let cap = inset.min(delta.length() * 0.5);
                        let start = start + tangent * cap;
                        let end = end - tangent * cap;
                        vec![
                            start - normal * half_width,
                            end - normal * half_width,
                            end + normal * half_width,
                            start + normal * half_width,
                        ]
                    },
                    color,
                    Border::NONE,
                    width.min(delta.length()) * 0.5,
                    scale,
                );
            }
        }
    }
}
