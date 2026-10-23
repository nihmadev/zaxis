//! Host textures registered with an embedded renderer.
use super::EmbeddedRenderer;
use crate::{ExternalTexture, ExternalTextureError, ExternalTextureOptions};

impl EmbeddedRenderer {
    /// Register a texture of the host's device to be shown by `Image`s without a copy: a video
    /// frame, a render target, a canvas the host draws. The host's writes must be submitted
    /// before the frame that samples it; this renderer inserts no barriers. Siblings share the
    /// registration.
    pub fn register_external_texture(
        &mut self,
        texture: &wgpu::Texture,
        options: ExternalTextureOptions,
    ) -> Result<ExternalTexture, ExternalTextureError> {
        self.gpu.register_external(texture, options)
    }
}
