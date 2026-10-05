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
    Handle(ImageHandle),
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
