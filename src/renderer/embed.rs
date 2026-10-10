//! Drawing into a render pass or texture the host application owns: no window, no surface,
//! and a device the host created. See [`EmbeddedRenderer`].

mod error;
mod options;
mod pass;
mod target;
mod viewport;

pub use error::EmbedError;
pub use options::{EmbedAlpha, EmbedColorSpace, EmbedOptions};
pub use pass::RecordReport;
pub use viewport::{EmbedLoad, EmbedViewport, PhysicalRect};

use super::{
    gpu::{self, Gpu},
    pipeline::{PipelineSet, Target},
    Renderer, RendererStats,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, MutexGuard, PoisonError,
};

/// Draws [`DrawData`](crate::DrawData) into a render pass or texture owned by the host.
///
/// The host keeps its [`wgpu::Device`], queue, window and event loop and runs the
/// [`Context`](crate::Context) itself; this type only turns the frame it produced into GPU
/// work, with no surface and no `Window`. It never polls the device and never submits: it
/// writes buffers and textures through the queue and records commands into a pass or encoder
/// the host passes in, which the host submits with the rest of its frame.
///
/// There are two ways to draw, chosen per frame:
///
/// * [`prepare`](Self::prepare) once, then [`record`](Self::record) into a pass the host
///   began (with its own color format, MSAA and depth attachment). The pass cannot be read
///   back while it is open, so backdrop blur and materials that read the backdrop are not
///   available; see [`RecordReport`] for what happens to them.
/// * [`render_to`](Self::render_to) begins its own passes in the host's command encoder, so
///   blur works over what the host drew into the texture.
///
/// One value is one interface surface: its geometry buffers, viewport uniform and backdrop
/// targets are its own, so a frame prepared on it is valid until the next `prepare` or
/// `render_to` on the same value is submitted. Several viewports on one device use
/// [`create_sibling`](Self::create_sibling), which shares pipelines and the glyph atlas and
/// image store. A value is `Send` (native) and `Sync`, so it may be prepared on the thread that
/// builds the interface and recorded on the one that records commands, behind a lock.
pub struct EmbeddedRenderer {
    gpu: Gpu,
    options: EmbedOptions,
    /// Pipelines for the host's pass.
    pass_pipelines: Arc<PipelineSet>,
    /// Pipelines for `render_to`: the same color format, one sample, no depth.
    texture_pipelines: Option<Arc<PipelineSet>>,
    prepared: Option<pass::Prepared>,
    recorded: Counters,
}

#[derive(Default)]
struct Counters {
    frames: AtomicU64,
    draws: AtomicU64,
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl EmbeddedRenderer {
    /// A renderer for the device and queue of the host.
    ///
    /// Without the adapter the backdrop filter keeps its intermediate textures in
    /// `Rgba16Float` on desktop and in the target format in a browser; pass the adapter to
    /// [`new_with_adapter`](Self::new_with_adapter) to check what it renders to.
    ///
    /// The device needs at least the downlevel default limits (four bind groups, one
    /// dynamic uniform buffer binding). Pipelines are built here, for the target in
    /// `options`, and shaders compile now rather than in the first frame.
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        options: EmbedOptions,
    ) -> Result<Self, EmbedError> {
        Self::with_half_float(device, queue, !cfg!(target_arch = "wasm32"), options)
    }

    /// Like [`new`](Self::new), asking `adapter` which format the backdrop filter may use.
    pub fn new_with_adapter(
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter: &wgpu::Adapter,
        options: EmbedOptions,
    ) -> Result<Self, EmbedError> {
        Self::with_half_float(device, queue, gpu::half_float(adapter), options)
    }

    fn with_half_float(
        device: wgpu::Device,
        queue: wgpu::Queue,
        half_float: bool,
        options: EmbedOptions,
    ) -> Result<Self, EmbedError> {
        options.validate(device.features())?;
        let gpu = Gpu::new(device, queue, half_float, Arc::default());
        Self::from_gpu(gpu, options)
    }

    fn from_gpu(gpu: Gpu, options: EmbedOptions) -> Result<Self, EmbedError> {
        options.validate(gpu.device.features())?;
        let pass_pipelines = lock(&gpu.pipelines).get(pass_target(&options));
        Ok(Self {
            gpu,
            options,
            pass_pipelines,
            texture_pipelines: None,
            prepared: None,
            recorded: Counters::default(),
        })
    }

    /// Another renderer on the same device for a second panel or viewport, possibly with
    /// other `options`. Pipelines, the glyph atlas and the image store are shared; buffers and
    /// backdrop targets are not. Every draw data presented through a family must allocate
    /// its [`TextureId`](crate::TextureId)s from one source, as contexts created from the same
    /// [`SharedResources`](crate::SharedResources) do.
    pub fn create_sibling(&self, options: EmbedOptions) -> Result<Self, EmbedError> {
        Self::from_gpu(self.gpu.sibling(), options)
    }

    /// The options in force.
    pub fn options(&self) -> &EmbedOptions {
        &self.options
    }

    /// Change the host's target format, sample count or depth format, for example after a
    /// resize that recreated the swapchain with another format. Buffers, the atlas and the
    /// pipelines of other targets are kept; only what depends on the old target is dropped
    /// (the frame prepared for it, backdrop textures, pipelines nobody else uses are
    /// released lazily). Call `prepare` again before `record`.
    pub fn set_options(&mut self, options: EmbedOptions) -> Result<(), EmbedError> {
        options.validate(self.gpu.device.features())?;
        if options == self.options {
            return Ok(());
        }
        self.prepared = None;
        if options.attachment_format() != self.options.attachment_format() {
            self.gpu.blur = None;
            self.gpu.stats.blur_target_bytes = 0;
            self.texture_pipelines = None;
        }
        self.pass_pipelines = lock(&self.gpu.pipelines).get(pass_target(&options));
        self.options = options;
        Ok(())
    }

    /// Record that the host's device was lost: report it from the host's
    /// `set_device_lost_callback` (this crate never replaces the host's callback). Every
    /// renderer on the same device then fails with [`EmbedError::DeviceLost`] instead of
    /// touching the dead device; make a new renderer on the new device.
    pub fn notify_device_lost(&self, reason: impl Into<String>) {
        *lock(&self.gpu.device_lost) = Some(reason.into());
    }

    /// Whether [`notify_device_lost`](Self::notify_device_lost) was called for this device.
    pub fn is_device_lost(&self) -> bool {
        self.gpu.lost().is_some()
    }

    /// Free the backdrop filter's textures now; they come back with the next frame that
    /// has a backdrop effect.
    pub fn release_targets(&mut self) {
        self.gpu.blur = None;
        self.gpu.stats.blur_target_bytes = 0;
    }

    /// Counters of this renderer (uploads, draws, backdrop work), with frames recorded or
    /// rendered as `presented_frames`.
    pub fn stats(&self) -> RendererStats {
        let mut stats = self.gpu.stats;
        stats.presented_frames += self.recorded.frames.load(Ordering::Relaxed);
        stats.draw_calls += self.recorded.draws.load(Ordering::Relaxed);
        stats
    }

    /// Drop every managed image texture, including those other renderers of the device use.
    pub fn clear_image_textures(&mut self) {
        self.gpu.clear_image_textures();
    }

    /// Pipelines of materials held now, failed builds included.
    pub fn material_pipeline_count(&self) -> usize {
        self.gpu.material_pipeline_count()
    }

    /// Why material pipelines failed to build since the last call.
    pub fn take_material_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.gpu.material_errors)
    }

    /// Built-in pipeline targets cached on the device, shared by its renderers.
    #[doc(hidden)]
    pub fn shared_pipeline_targets(&self) -> usize {
        lock(&self.gpu.pipelines).len()
    }

    fn check_alive(&self) -> Result<(), EmbedError> {
        match self.gpu.lost() {
            Some(message) => Err(EmbedError::DeviceLost(message)),
            None => Ok(()),
        }
    }
}

impl Renderer {
    /// An [`EmbeddedRenderer`] on this renderer's device that shares its pipelines, glyph atlas
    /// and image store: one atlas serves the window and the panels drawn into the host's
    /// own targets. The same rule as for [`create_sibling`](Self::create_sibling) applies to
    /// [`TextureId`](crate::TextureId)s.
    pub fn create_embedded(&self, options: EmbedOptions) -> Result<EmbeddedRenderer, EmbedError> {
        EmbeddedRenderer::from_gpu(self.gpu.sibling(), options)
    }
}

fn pass_target(options: &EmbedOptions) -> Target {
    Target {
        format: options.attachment_format(),
        samples: options.sample_count,
        depth: options.depth_format,
    }
}
