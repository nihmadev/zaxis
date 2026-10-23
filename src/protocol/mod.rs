//! Public drawing protocol shared by UI producers and rendering backends.
//!
//! This module has no dependency on wgpu or winit. Backends consume logical
//! vertices, global u32 indices, ordered commands, and complete texture images.
//! Geometry is cached by `(source, revision)`; textures by producer, ID, size,
//! pixel format and image revision. A texture that changed in a few places may also
//! come with a [`TextureUpdate`] naming them, so a backend can skip the full upload. Convert logical clip rectangles using `scale_factor`.
//!
//! Create data without a UI context, or read it through [`crate::Context::draw_data`]:
//!
//! ```
//! use std::sync::Arc;
//! use zaxis::{vec2, Rect};
//! use zaxis::protocol::{DrawCommand, DrawData, TextureId, TextureImage, Vertex};
//!
//! let mut frame = DrawData::new(vec2(800.0, 600.0), 1.5);
//! let texture = TextureId(1);
//! frame.textures.push(TextureImage {
//!     id: texture,
//!     size: [1, 1],
//!     pixels: Arc::new(vec![255; 4]),
//!     revision: 1,
//! });
//! frame.vertices = [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]]
//!     .map(|position| Vertex { position, uv: [0.5; 2], color: [1.0; 4] })
//!     .to_vec();
//! frame.indices = vec![0, 1, 2];
//! frame.commands.push(DrawCommand {
//!     indices: 0..3,
//!     clip_rect: Rect::from_min_size(vec2(0.0, 0.0), frame.logical_size),
//!     texture,
//!     ..Default::default()
//! });
//! frame.revision += 1;
//!
//! // Independent producers must never share a geometry cache identity.
//! let other = DrawData::new(frame.logical_size, frame.scale_factor);
//! assert_ne!(frame.source, other.source);
//! // Existing imports still refer to the same public types.
//! let _: &zaxis::DrawData = &frame;
//! let _: &zaxis::shapes::DrawData = &frame;
//! ```

mod draw_command;
mod draw_data;
mod external;
mod format;
mod material;
mod texture;
mod update;
mod vertex;

pub use draw_command::{DrawCommand, MAX_BACKDROP_SIGMA};
pub(crate) use draw_data::next_source;
pub use draw_data::{DrawData, GeometryUpdate};
pub use external::{ExternalAlpha, ExternalImage};
pub use format::PixelFormat;
pub use material::{MaterialDraw, MaterialId, MaterialSource};
pub use texture::{TextureFilter, TextureId, TextureImage, TextureOptions};
pub use update::{TextureRegion, TextureUpdate};
pub use vertex::Vertex;
