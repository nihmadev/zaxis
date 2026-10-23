//! Byte layouts of the pixels in a [`TextureImage`](super::TextureImage).

/// How the four bytes of a texel are laid out. All formats are 8 bits per channel, sRGB
/// encoded, four bytes per pixel, row-major and tightly packed in a payload.
///
/// A backend uploads the bytes as they are: a BGRA payload reaches a `Bgra8UnormSrgb`
/// texture without swapping channels on the CPU, and the `…x…` formats are drawn as fully
/// opaque whatever their fourth byte holds. Where the device cannot sample a layout (WebGL2
/// has no BGRA textures) the backend converts the written rows, and only those, itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// Red, green, blue, straight alpha. The only layout decoded files produce.
    #[default]
    Rgba8Srgb,
    /// Blue, green, red, straight alpha: what browsers, compositors and most capture APIs
    /// hand out.
    Bgra8Srgb,
    /// Red, green, blue and an unused byte: alpha is taken as one.
    Rgbx8Srgb,
    /// Blue, green, red and an unused byte: alpha is taken as one.
    Bgrx8Srgb,
}

impl PixelFormat {
    /// Bytes per pixel; the same for every format.
    pub const BYTES_PER_PIXEL: usize = 4;

    /// Blue comes before red in memory.
    pub const fn is_bgr(self) -> bool {
        matches!(self, Self::Bgra8Srgb | Self::Bgrx8Srgb)
    }

    /// The fourth byte is ignored and every texel is opaque.
    pub const fn is_opaque(self) -> bool {
        matches!(self, Self::Rgbx8Srgb | Self::Bgrx8Srgb)
    }

    /// Bytes of a tightly packed `size` image, or `None` when the count overflows.
    pub fn byte_len(self, [width, height]: [u32; 2]) -> Option<usize> {
        usize::try_from(u64::from(width) * u64::from(height) * Self::BYTES_PER_PIXEL as u64).ok()
    }
}
