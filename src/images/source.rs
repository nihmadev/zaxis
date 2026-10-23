use crate::{ExternalImage, PixelFormat};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Owned source description. Cloning is cheap and never hashes pixel/encoded contents.
/// Static slices use address+length identity; shared data uses its Arc allocation.
/// Keep a shared source around between frames. Files are cached by lexical absolute path.
#[derive(Clone, Debug)]
pub struct ImageSource(pub(crate) Source, pub(crate) u64);

#[derive(Clone, Debug)]
pub enum Source {
    Path(Arc<PathBuf>),
    Static(&'static [u8]),
    Encoded(Arc<Vec<u8>>),
    Rgba([u32; 2], Arc<Vec<u8>>),
    /// Raw pixels in a layout other than straight-alpha RGBA, or any raw pixels meant to be
    /// updated in place.
    Pixels(Pixels),
    /// A texture the host registered with a renderer: no pixels pass through the cache.
    External(ExternalImage),
    Handle(ImageHandle),
}

/// A raw pixel source seen from outside: layout, size, bytes, and whether it is live.
pub type RawPixels<'a> = (PixelFormat, [u32; 2], &'a Arc<Vec<u8>>, bool);

/// Tightly packed 8-bit pixels the host produced, uploaded without conversion.
#[derive(Clone, Debug)]
pub struct Pixels {
    pub format: PixelFormat,
    pub size: [u32; 2],
    pub data: Arc<Vec<u8>>,
    /// Shown at its own resolution, never downsampled: updates change parts of it.
    pub live: bool,
}

impl Source {
    /// Raw pixels: their layout, size, bytes and whether they are shown at their own size.
    pub fn pixels(&self) -> Option<RawPixels<'_>> {
        match self {
            Self::Rgba(size, data) => Some((PixelFormat::Rgba8Srgb, *size, data, false)),
            Self::Pixels(p) => Some((p.format, p.size, &p.data, p.live)),
            _ => None,
        }
    }

    /// Bytes the source keeps in memory.
    pub fn held_bytes(&self) -> usize {
        match self {
            Self::Encoded(p) | Self::Rgba(_, p) => p.len(),
            Self::Pixels(p) => p.data.len(),
            _ => 0,
        }
    }

    /// The buffer of a shared source, for telling copies of it apart.
    pub fn shared(&self) -> Option<&Arc<Vec<u8>>> {
        match self {
            Self::Encoded(p) | Self::Rgba(_, p) => Some(p),
            Self::Pixels(p) => Some(&p.data),
            _ => None,
        }
    }
}

/// Context-local live resource. Replacing its source preserves texture and widget identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageHandle {
    pub(crate) owner: u64,
    pub(crate) id: u64,
}

impl ImageSource {
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Self(Source::Path(Arc::new(path.into())), 0)
    }
    pub fn bytes(bytes: &'static [u8]) -> Self {
        Self(Source::Static(bytes), 0)
    }
    pub fn encoded(bytes: impl Into<Arc<Vec<u8>>>) -> Self {
        Self(Source::Encoded(bytes.into()), 0)
    }
    /// Explicit version of the same source identity. Different versions cannot reuse old pixels.
    /// Prefer Context::update_image for frequently changing data with a stable handle.
    pub fn with_revision(mut self, revision: u64) -> Self {
        self.1 = revision;
        self
    }
    /// Tightly packed sRGB RGBA8 with straight alpha. Validation happens in the worker.
    pub fn rgba(size: [u32; 2], pixels: impl Into<Arc<Vec<u8>>>) -> Self {
        Self(Source::Rgba(size, pixels.into()), 0)
    }
    /// Tightly packed sRGB BGRA8 with straight alpha, the layout of browser and capture
    /// frames. The bytes reach a BGRA texture as they are, with no swapping on the CPU.
    pub fn bgra(size: [u32; 2], pixels: impl Into<Arc<Vec<u8>>>) -> Self {
        Self::pixels(PixelFormat::Bgra8Srgb, size, pixels)
    }
    /// Tightly packed RGBA8 whose fourth byte is ignored: every pixel is opaque.
    pub fn rgba_opaque(size: [u32; 2], pixels: impl Into<Arc<Vec<u8>>>) -> Self {
        Self::pixels(PixelFormat::Rgbx8Srgb, size, pixels)
    }
    /// Tightly packed BGRA8 whose fourth byte is ignored: every pixel is opaque.
    pub fn bgra_opaque(size: [u32; 2], pixels: impl Into<Arc<Vec<u8>>>) -> Self {
        Self::pixels(PixelFormat::Bgrx8Srgb, size, pixels)
    }
    /// Tightly packed 8-bit pixels in `format`. Any layout but [`PixelFormat::Rgba8Srgb`] is
    /// shown at its own resolution, never downsampled, so that
    /// [`Context::update_image_region`](crate::Context::update_image_region) can change
    /// parts of it.
    pub fn pixels(format: PixelFormat, size: [u32; 2], pixels: impl Into<Arc<Vec<u8>>>) -> Self {
        let data = pixels.into();
        if format == PixelFormat::Rgba8Srgb {
            return Self(Source::Rgba(size, data), 0);
        }
        Self(
            Source::Pixels(Pixels {
                format,
                size,
                data,
                live: true,
            }),
            0,
        )
    }
    /// A texture the host owns, registered with a renderer
    /// ([`Renderer::register_external_texture`](crate::Renderer::register_external_texture)) and
    /// sampled without a copy. The image is ready at once, its size is the texture's, and
    /// `Image` filter, tint, opacity, fit and rounding apply as for any image; the filter of
    /// the registration is the one used. Replace it with [`Context::update_image`](crate::Context::update_image)
    /// after registering again.
    pub fn external(image: ExternalImage) -> Self {
        Self(Source::External(image), 0)
    }
    /// Show raw pixels at their own resolution and keep them for in-place updates, instead
    /// of downsampling them when displayed small. Has no effect on other sources and on
    /// layouts other than RGBA8, which are always live.
    pub fn live(mut self) -> Self {
        if let Source::Rgba(size, data) = self.0 {
            self.0 = Source::Pixels(Pixels {
                format: PixelFormat::Rgba8Srgb,
                size,
                data,
                live: true,
            });
        }
        self
    }
}
impl From<ExternalImage> for ImageSource {
    fn from(image: ExternalImage) -> Self {
        Self::external(image)
    }
}
impl From<&str> for ImageSource {
    fn from(s: &str) -> Self {
        Self::path(s)
    }
}
impl From<String> for ImageSource {
    fn from(s: String) -> Self {
        Self::path(s)
    }
}
impl From<&Path> for ImageSource {
    fn from(s: &Path) -> Self {
        Self::path(s)
    }
}
impl From<PathBuf> for ImageSource {
    fn from(s: PathBuf) -> Self {
        Self::path(s)
    }
}
impl From<&ImageSource> for ImageSource {
    fn from(s: &ImageSource) -> Self {
        s.clone()
    }
}
impl From<ImageHandle> for ImageSource {
    fn from(s: ImageHandle) -> Self {
        Self(Source::Handle(s), 0)
    }
}
impl From<&'static [u8]> for ImageSource {
    fn from(s: &'static [u8]) -> Self {
        Self::bytes(s)
    }
}
impl<const N: usize> From<&'static [u8; N]> for ImageSource {
    fn from(s: &'static [u8; N]) -> Self {
        Self::bytes(s)
    }
}
#[cfg(feature = "bundled-icons")]
impl From<&'static z_icons::Icon> for ImageSource {
    fn from(icon: &'static z_icons::Icon) -> Self {
        Self::bytes(icon.svg())
    }
}
