//! Built-in local image loading. All expensive work runs on bounded worker threads.
mod cache;
#[doc(hidden)]
pub mod decode;
mod file;
#[doc(hidden)]
pub mod resize;
#[doc(hidden)]
pub mod source;
mod svg_limits;
mod worker;

pub(crate) use cache::ImageCache;
pub use cache::{ImageLimits, ImageMetrics, ImageStage, ImageState, ImageTiming};
pub use decode::{DecodedImage, ImageDecoder, ImageError};
pub use source::{ImageHandle, ImageSource};

use crate::TextureId;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// Allocator shared by the text atlas and image cache of one set of shared resources.
#[derive(Clone)]
pub(crate) struct TextureIds(Arc<AtomicU64>);
impl Default for TextureIds {
    fn default() -> Self {
        Self(Arc::new(AtomicU64::new(1)))
    }
}
impl TextureIds {
    pub(crate) fn next(&self) -> TextureId {
        TextureId(self.0.fetch_add(1, Ordering::Relaxed))
    }
}

/// The image cache of one set of [`SharedResources`](crate::SharedResources): every window
/// of an application decodes, rasterizes and keeps each image once.
#[derive(Clone)]
pub(crate) struct SharedImages(Arc<std::sync::Mutex<ImageCache>>);
impl SharedImages {
    pub(crate) fn new(ids: TextureIds) -> Self {
        let owner = crate::protocol::next_source();
        Self(Arc::new(std::sync::Mutex::new(ImageCache::new(owner, ids))))
    }
    pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, ImageCache> {
        self.0.lock().expect("image cache mutex")
    }
}
