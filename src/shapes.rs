//! Typed drawing primitives and CPU tessellation.

mod border;
mod color;
mod corner_radius;
mod gradient;
#[doc(hidden)]
pub mod mesh;
#[doc(hidden)]
pub mod outline;
mod rect;
mod shadow;
mod shape;
mod transform;

pub use border::Border;
pub use color::Color;
pub use corner_radius::CornerRadius;
pub use gradient::{Gradient, GradientDirection};
pub use rect::Rect;
pub use shadow::Shadow;
pub use shape::{RectShape, Shape};
pub use transform::Transform;

pub use mesh::Mesh;

// Preserve the original public paths for drawing protocol types.
pub use crate::protocol::{DrawCommand, DrawData, TextureId, Vertex};
