use crate::{
    Context, ImageDecoder, ImageError, ImageHandle, ImageLimits, ImageMetrics, ImageSource,
    ImageState, ImageTiming,
};
use std::sync::Arc;

impl Context {
    /// Observe a source without starting work. A not-yet-requested source is Loading.
    pub fn image_state(&self, source: impl Into<ImageSource>) -> ImageState {
        self.images.state(source.into())
    }
    /// Retain a context-local handle for explicit replacement/reload/free operations.
    /// Loading begins when displayed; this does not synchronously read a file.
    pub fn load_image(
        &mut self,
        source: impl Into<ImageSource>,
    ) -> Result<ImageHandle, ImageError> {
        let handle = self.images.resolve(source.into())?;
        self.images.pin(handle);
        Ok(handle)
    }
    /// Replace source data. Completion of an older generation is discarded.
    pub fn update_image(
        &mut self,
        handle: ImageHandle,
        source: impl Into<ImageSource>,
    ) -> Result<(), ImageError> {
        self.images.reload(handle, Some(source.into()))?;
        self.request_repaint();
        Ok(())
    }
    /// Explicit retry/re-read; files are never watched or read again automatically.
    pub fn reload_image(&mut self, handle: ImageHandle) -> Result<(), ImageError> {
        self.images.reload(handle, None)?;
        self.request_repaint();
        Ok(())
    }
    pub fn invalidate_image(&mut self, source: impl Into<ImageSource>) -> Result<(), ImageError> {
        self.images.invalidate(source.into())?;
        self.request_repaint();
        Ok(())
    }
    /// Invalidate this handle. Pending completions cannot recreate it.
    pub fn release_image(&mut self, handle: ImageHandle) {
        self.images.release(handle);
        self.request_repaint();
    }
    /// Drop decoded/raster CPU residency; handles retain recoverable source data.
    pub fn clear_image_cache(&mut self) {
        self.images.clear_decoded();
        self.request_repaint();
    }
    pub fn image_limits(&self) -> &ImageLimits {
        &self.images.limits
    }
    pub fn set_image_limits(&mut self, limits: ImageLimits) {
        self.images.limits = limits;
        self.clear_image_cache();
    }
    pub fn add_image_decoder(&mut self, decoder: Arc<dyn ImageDecoder>) {
        self.images.add_decoder(decoder);
    }
    /// Custom hosts must install a callback which wakes their sleeping event loop.
    /// Called from workers: enqueue an event, never lock/call the Context here.
    /// The desktop runner installs its EventLoopProxy automatically.
    pub fn set_image_waker(&mut self, waker: impl Fn() + Send + Sync + 'static) {
        self.images.set_waker(Some(Arc::new(waker)));
    }
    pub fn clear_image_waker(&mut self) {
        self.images.set_waker(None);
    }
    pub fn image_metrics(&self) -> ImageMetrics {
        self.images.metrics()
    }
    /// Bounded diagnostic history; drain outside measured UI work in benchmarks.
    pub fn take_image_timings(&mut self) -> Vec<ImageTiming> {
        self.images.take_timings()
    }
}
