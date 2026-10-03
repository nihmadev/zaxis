//! Typed drawing primitives and CPU tessellation.

mod border;
mod color;
mod corner_radius;
mod gradient;
mod mesh;
mod outline;
mod rect;
mod shadow;
mod shape;
mod transform;

#[cfg(test)]
mod tests;

pub use border::Border;
pub use color::Color;
pub use corner_radius::CornerRadius;
pub use gradient::{Gradient, GradientDirection};
pub use rect::Rect;
pub use shadow::Shadow;
pub use shape::{RectShape, Shape};
pub use transform::Transform;

pub(crate) use mesh::Mesh;

// Preserve the original public paths for drawing protocol types.
pub use crate::protocol::{DrawCommand, DrawData, TextureId, Vertex};
