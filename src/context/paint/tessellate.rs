//! Tessellation of one element's primitives into a mesh.

use super::Paint;
use crate::{shapes::Mesh, text::TextSystem, Rect, Vec2};

/// The mesh of `paint` at `scale` physical pixels per logical pixel; images use
/// `image_scale`, which includes the scale of the visuals around them.
pub(super) fn mesh(text: &mut TextSystem, paint: &[Paint], scale: f32, image_scale: f32) -> Mesh {
    let mut mesh = Mesh::default();
    for primitive in paint {
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
            Paint::Shape(shape) => mesh.shape(shape, scale),
            Paint::Material {
                rect,
                rounding,
                color,
                opacity,
                image,
            } => mesh.image(
                *rect,
                Rect::from_min_size(Vec2::ZERO, Vec2::ONE),
                *rounding,
                *color,
                *opacity,
                image
                    .as_ref()
                    .map_or(crate::TextureId::WHITE, |i| i.texture),
                image_scale,
            ),
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
                        image_scale,
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
                mesh.gradient(*rect, *rounding, colors, *columns, *rows, scale);
            }
            Paint::Text {
                text: string,
                position,
                size,
                weight,
                wrap_width,
                color,
            } => text.paint(
                &mut mesh,
                string,
                *position,
                *size,
                *weight,
                *wrap_width,
                *color,
                scale,
            ),
            Paint::Paragraph {
                text: string,
                position,
                size,
                font,
                wrap_width,
                tab,
                color,
            } => text.paint_with_tab(
                &mut mesh,
                string,
                *position,
                *size,
                *font,
                *wrap_width,
                *tab,
                *color,
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
                color,
            } => text.paint_rich(
                &mut mesh,
                string,
                runs,
                *position,
                *size,
                *font,
                *wrap_width,
                *tab,
                *color,
                scale,
            ),
        }
    }
    mesh
}
