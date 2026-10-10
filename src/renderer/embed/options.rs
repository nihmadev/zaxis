//! What the host's render pass looks like.

use super::EmbedError;

/// How the host composites the result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EmbedAlpha {
    /// The target is opaque or composited as if it were: a clear color is used as it is.
    #[default]
    Opaque,
    /// The target is composited with premultiplied alpha (a transparent overlay): a clear
    /// color is premultiplied by its alpha first.
    Premultiplied,
}

/// How the target stores color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmbedColorSpace {
    /// An sRGB view: the hardware encodes what the shaders write after blending.
    Srgb,
    /// A float or wide target that holds linear values.
    Linear,
}

/// The attachments of the pass an [`EmbeddedRenderer`](super::EmbeddedRenderer) draws into.
///
/// `format` is the format of the target *texture*. The view in the pass must have
/// [`attachment_format`](Self::attachment_format): the same for sRGB and float formats, the
/// sRGB sibling for `Rgba8Unorm` and `Bgra8Unorm`, which the host lists in the texture's
/// `view_formats`. The shaders write linear color and rely on the sRGB view to encode it
/// after blending, as the window renderer does; a plain unorm view would be too dark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EmbedOptions {
    pub format: wgpu::TextureFormat,
    /// Samples of the pass's color attachment: 1, 2, 4, 8 or 16. Applies to
    /// [`record`](super::EmbeddedRenderer::record); `render_to` draws without MSAA.
    pub sample_count: u32,
    /// Format of the pass's depth/stencil attachment, if it has one. The interface neither
    /// tests nor writes depth or stencil.
    pub depth_format: Option<wgpu::TextureFormat>,
    pub alpha: EmbedAlpha,
}

impl EmbedOptions {
    /// Single-sample, no depth, opaque.
    pub const fn new(format: wgpu::TextureFormat) -> Self {
        Self {
            format,
            sample_count: 1,
            depth_format: None,
            alpha: EmbedAlpha::Opaque,
        }
    }

    pub const fn sample_count(mut self, sample_count: u32) -> Self {
        self.sample_count = sample_count;
        self
    }

    pub const fn depth_format(mut self, depth_format: wgpu::TextureFormat) -> Self {
        self.depth_format = Some(depth_format);
        self
    }

    pub const fn alpha(mut self, alpha: EmbedAlpha) -> Self {
        self.alpha = alpha;
        self
    }

    /// The format of the view the pass must render to.
    pub fn attachment_format(&self) -> wgpu::TextureFormat {
        self.format.add_srgb_suffix()
    }

    pub fn color_space(&self) -> EmbedColorSpace {
        if self.attachment_format().is_srgb() {
            EmbedColorSpace::Srgb
        } else {
            EmbedColorSpace::Linear
        }
    }

    /// Reject what no pass can use, before any pipeline is built for it.
    pub(super) fn validate(&self, features: wgpu::Features) -> Result<(), EmbedError> {
        use wgpu::TextureFormat as F;
        let blendable = match self.attachment_format() {
            F::Rgba8UnormSrgb | F::Bgra8UnormSrgb | F::Rgba16Float | F::Rgb10a2Unorm => true,
            F::Rgba32Float => features.contains(wgpu::Features::FLOAT32_BLENDABLE),
            _ => false,
        };
        if !blendable {
            return Err(EmbedError::InvalidOptions(
                "format must be Rgba8Unorm[Srgb], Bgra8Unorm[Srgb], Rgba16Float, Rgb10a2Unorm \
                 or Rgba32Float with FLOAT32_BLENDABLE",
            ));
        }
        if !matches!(self.sample_count, 1 | 2 | 4 | 8 | 16) {
            return Err(EmbedError::InvalidOptions(
                "sample count must be 1, 2, 4, 8 or 16",
            ));
        }
        if self
            .depth_format
            .is_some_and(|f| !f.is_depth_stencil_format())
        {
            return Err(EmbedError::InvalidOptions(
                "depth format must be a depth or stencil format",
            ));
        }
        Ok(())
    }
}
