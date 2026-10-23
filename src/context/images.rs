use super::Paint;
use crate::{
    Context, ImageDecoder, ImageError, ImageHandle, ImageLimits, ImageMetrics, ImageSource,
    ImageState, ImageTiming, PixelFormat, Rect, TextureRegion, Vec2,
};
use std::{sync::Arc, time::Duration};

impl Context {
    /// Observe a source without starting work. A not-yet-requested source is Loading.
    pub fn image_state(&self, source: impl Into<ImageSource>) -> ImageState {
        self.images.lock().state(source.into())
    }
    /// Retain a context-local handle for explicit replacement/reload/free operations.
    /// Loading begins when displayed; this does not synchronously read a file.
    pub fn load_image(
        &mut self,
        source: impl Into<ImageSource>,
    ) -> Result<ImageHandle, ImageError> {
        let handle = self.images.lock().resolve(source.into())?;
        self.images.lock().pin(handle);
        Ok(handle)
    }
    /// Replace source data. Completion of an older generation is discarded.
    pub fn update_image(
        &mut self,
        handle: ImageHandle,
        source: impl Into<ImageSource>,
    ) -> Result<(), ImageError> {
        self.images.lock().reload(handle, Some(source.into()))?;
        self.request_repaint();
        Ok(())
    }
    /// Replace the pixels of `region` of an image made from raw pixels
    /// ([`ImageSource::rgba`], [`bgra`](ImageSource::bgra), [`rgba_opaque`](ImageSource::rgba_opaque),
    /// [`bgra_opaque`](ImageSource::bgra_opaque)), keeping its handle, texture and widgets. The
    /// graphics card receives the bytes of the region, not the frame.
    ///
    /// `region` is in physical pixels of the image. `pixels` holds its rows in the layout of
    /// the image, `stride` bytes apart (at least `region.width * 4`; a frame read back from
    /// the GPU has its rows padded to 256 bytes). Only the bytes of the rows are read, and
    /// they are copied before this returns; [`update_image_region_shared`](Self::update_image_region_shared)
    /// takes a buffer that is kept instead.
    ///
    /// Several updates before a frame apply in order, the last one wins, and the frame draws
    /// them all. [`update_image`](Self::update_image) with a new source discards them.
    /// A region of an image that has not been drawn yet is accepted as well: the image
    /// holds its pixels already. An empty or out-of-bounds region, short pixels, a stride
    /// below the row length, a released or foreign handle, an image decoded from a file
    /// or a failed one return an [`ImageError`] and change nothing.
    ///
    /// An image displayed smaller than half its size is normally downsampled; one that is
    /// updated in place is shown at its own size and filtered by the GPU instead.
    pub fn update_image_region(
        &mut self,
        handle: ImageHandle,
        region: impl Into<TextureRegion>,
        pixels: &[u8],
        stride: usize,
    ) -> Result<(), ImageError> {
        let region = region.into();
        self.images
            .lock()
            .check_region(handle, region, pixels.len(), 0, stride)?;
        let row = region.width as usize * PixelFormat::BYTES_PER_PIXEL;
        let mut tight = Vec::with_capacity(row * region.height as usize);
        for y in 0..region.height as usize {
            tight.extend_from_slice(&pixels[y * stride..][..row]);
        }
        self.images
            .lock()
            .update_region(handle, region, Arc::new(tight), 0, row)?;
        self.request_repaint();
        Ok(())
    }
    /// Like [`update_image_region`](Self::update_image_region) for a buffer that is shared
    /// instead of copied: the rows of the region start at `offset` in `pixels`, `stride`
    /// bytes apart. The buffer is held until the next frame applies it, so keep writing
    /// readback data into a new one, or one nobody else touches.
    pub fn update_image_region_shared(
        &mut self,
        handle: ImageHandle,
        region: impl Into<TextureRegion>,
        pixels: Arc<Vec<u8>>,
        offset: usize,
        stride: usize,
    ) -> Result<(), ImageError> {
        self.images
            .lock()
            .update_region(handle, region.into(), pixels, offset, stride)?;
        self.request_repaint();
        Ok(())
    }
    /// Explicit retry/re-read; files are never watched or read again automatically.
    pub fn reload_image(&mut self, handle: ImageHandle) -> Result<(), ImageError> {
        self.images.lock().reload(handle, None)?;
        self.request_repaint();
        Ok(())
    }
    pub fn invalidate_image(&mut self, source: impl Into<ImageSource>) -> Result<(), ImageError> {
        self.images.lock().invalidate(source.into())?;
        self.request_repaint();
        Ok(())
    }
    /// Invalidate this handle. Pending completions cannot recreate it.
    pub fn release_image(&mut self, handle: ImageHandle) {
        self.images.lock().release(handle);
        self.request_repaint();
    }
    /// Drop decoded/raster CPU residency; handles retain recoverable source data.
    pub fn clear_image_cache(&mut self) {
        self.images.lock().clear_decoded();
        self.request_repaint();
    }
    pub fn image_limits(&self) -> ImageLimits {
        self.images.lock().limits.clone()
    }
    pub fn set_image_limits(&mut self, limits: ImageLimits) {
        self.images.lock().limits = limits;
        self.clear_image_cache();
    }
    pub fn add_image_decoder(&mut self, decoder: Arc<dyn ImageDecoder>) {
        self.images.lock().add_decoder(decoder);
    }
    /// Custom hosts must install a callback which wakes their sleeping event loop.
    /// Called from workers: enqueue an event, never lock/call the Context here.
    /// The desktop runner installs its EventLoopProxy automatically.
    pub fn set_image_waker(&mut self, waker: impl Fn() + Send + Sync + 'static) {
        self.images.lock().set_waker(Some(Arc::new(waker)));
    }
    /// Decode images on the thread that draws, spending at most `frame_budget` per frame
    /// (one image always runs, so a single large one can exceed it), instead of on worker
    /// threads. This is the only mode on `wasm32`, where it is the default. `None` returns
    /// to worker threads on targets that have them.
    pub fn decode_images_inline(&mut self, frame_budget: Option<Duration>) {
        self.images.lock().set_inline_decoding(frame_budget);
    }
    pub fn clear_image_waker(&mut self) {
        self.images.lock().set_waker(None);
    }
    pub fn image_metrics(&self) -> ImageMetrics {
        self.images.lock().metrics()
    }
    /// Bounded diagnostic history; drain outside measured UI work in benchmarks.
    pub fn take_image_timings(&mut self) -> Vec<ImageTiming> {
        self.images.lock().take_timings()
    }

    /// Request the images of one element at the size they are displayed at, visuals
    /// included. Returns the scale the element is tessellated at: an element with images
    /// is cached per displayed scale.
    pub(super) fn request_painted_images(&mut self, paint: &[Paint], clip: Rect) -> f32 {
        let mut images = false;
        for primitive in paint {
            if let Paint::Material {
                rect,
                image: Some(image),
                ..
            } = primitive
            {
                images = true;
                if !rect.intersect(clip).is_empty() {
                    self.images.lock().request(
                        image.handle,
                        rect.size() / image.crop.size().max(Vec2::splat(0.001))
                            * self.scale
                            * self.paint_state.image_visual_scale(),
                        image.texture,
                    );
                }
            }
            if let Paint::Image {
                rect,
                uv,
                handle,
                texture,
                ..
            } = primitive
            {
                images = true;
                if !rect.intersect(clip).is_empty() {
                    self.images.lock().request(
                        *handle,
                        rect.size() / uv.size().max(Vec2::splat(0.001))
                            * self.scale
                            * self.paint_state.image_visual_scale(),
                        *texture,
                    );
                }
            }
        }
        if images {
            self.scale * self.paint_state.image_visual_scale()
        } else {
            self.scale
        }
    }

    /// End of pass: images that settle later schedule their pass, finished loads repaint.
    pub(super) fn finish_images(&mut self) {
        let settle = self.images.lock().finish_frame();
        if let Some(deadline) = settle {
            self.request_repaint_after(deadline.saturating_duration_since(self.frame_time));
        }
        if self.images.lock().take_state_changed() {
            self.request_repaint();
        }
    }
}
