//! Indexed draw ranges and their bindings.

use super::TextureId;
use crate::Rect;
use std::ops::Range;

/// One indexed draw into the shared frame buffers. Indices are global u32 offsets.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawCommand {
    pub indices: Range<u32>,
    pub clip_rect: Rect,
    pub texture: TextureId,
    /// Backdrop Gaussian sigma in logical pixels. `None` draws a normal texture.
    pub blur: Option<f32>,
    /// Draw the procedural scroll-edge fade; UV.y runs from transparent to the edge.
    pub scroll_hint: bool,
}
