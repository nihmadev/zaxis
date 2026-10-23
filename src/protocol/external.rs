//! Textures the host owns and draws into itself.

use super::{TextureFilter, TextureId};

/// How the alpha of a host texture is read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ExternalAlpha {
    /// The fourth channel is straight alpha, like every decoded image.
    #[default]
    Straight,
    /// The fourth channel is ignored: every texel is opaque.
    Opaque,
}

/// A texture of the host's device registered with a renderer
/// ([`Renderer::register_external_texture`](crate::Renderer::register_external_texture)),
/// described without the wgpu object so that draw data stays renderer-agnostic. The renderer
/// looks the texture up by `id` and checks `size` against what it registered; the pixels are
/// never part of the draw data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalImage {
    pub id: TextureId,
    /// Physical pixels.
    pub size: [u32; 2],
    /// Fixed at registration: one registration is one sampler binding.
    pub filter: TextureFilter,
    pub alpha: ExternalAlpha,
}

impl ExternalImage {
    /// Texture IDs at or above this value belong to external registrations; those below are
    /// allocated by contexts.
    pub const FIRST_ID: u64 = 1 << 62;
}
