//! Built-in local image loading. All expensive work runs on bounded worker threads.
mod cache;
mod decode;
mod resize;
mod source;
mod svg_limits;
#[cfg(test)]
mod tests;
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

/// Per-context allocator shared by the text atlas and image cache.
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
