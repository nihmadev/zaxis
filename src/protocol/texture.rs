//! Texture identities and versioned image payloads.

use std::sync::Arc;

/// Texture identity. Zero is the built-in white texture for untextured geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub u64);

impl TextureId {
    pub const WHITE: Self = Self(0);
}

/// Sampling is local to a texture binding. Text defaults to linear filtering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextureFilter {
    #[default]
    Linear,
    Nearest,
}

/// Optional binding/lifetime policy. Omitted entries retain the original protocol
/// behavior: linear sampling, lifetime until a producer switch or renderer drop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextureOptions {
    pub filter: TextureFilter,
    /// Eligible for backend LRU eviction. Every frame still supplies full payloads.
    pub managed: bool,
}

/// A complete texture image shared with the rendering backend.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureImage {
    /// Identity referenced by draw commands. Reserve zero for the white texture.
    pub id: TextureId,
    /// Nonzero width and height in physical pixels.
    pub size: [u32; 2],
    /// Row-major, tightly packed sRGB RGBA8 pixels with straight alpha.
    /// The length must equal `size[0] * size[1] * 4`.
    pub pixels: Arc<Vec<u8>>,
    /// Increment whenever pixels change; unchanged revisions reuse GPU uploads.
    pub revision: u64,
}
