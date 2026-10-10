//! Indexed draw ranges and their bindings.

use super::{MaterialDraw, TextureId};
use crate::Rect;
use std::ops::Range;

/// Largest backdrop blur sigma a command may ask for, in logical pixels.
pub const MAX_BACKDROP_SIGMA: f32 = 64.0;

/// One indexed draw into the shared frame buffers. Indices are global u32 offsets.
///
/// Build commands by hand with `..Default::default()` so fields added later keep their
/// neutral value.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawCommand {
    pub indices: Range<u32>,
    pub clip_rect: Rect,
    pub texture: TextureId,
    /// Backdrop Gaussian sigma in logical pixels. `None` draws a normal texture.
    pub blur: Option<f32>,
    /// Draw the procedural scroll-edge fade; UV.y runs from transparent to the edge.
    pub scroll_hint: bool,
    /// A user material replacing the fragment color stage. `None` draws normally. With a
    /// material and a `blur` the material also reads the sharp and the blurred backdrop.
    /// Not combinable with `scroll_hint`.
    pub material: Option<MaterialDraw>,
}

impl Default for DrawCommand {
    /// An empty range with an empty clip, the white texture and no effect.
    fn default() -> Self {
        Self {
            indices: 0..0,
            clip_rect: Rect::default(),
            texture: TextureId::WHITE,
            blur: None,
            scroll_hint: false,
            material: None,
        }
    }
}
