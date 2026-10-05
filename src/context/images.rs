use super::Paint;
use crate::{
    Context, ImageDecoder, ImageError, ImageHandle, ImageLimits, ImageMetrics, ImageSource,
    ImageState, ImageTiming, Rect, Vec2,
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
