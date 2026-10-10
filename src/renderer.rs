//! wgpu backend with persistent buffers, texture versioning, and surface recovery.

mod adapter;
mod backdrop;
#[doc(hidden)]
pub mod blur;
mod diagnostics;
mod draw;
mod embed;
mod error;
mod frame;
mod geometry;
mod gpu;
mod init;
#[doc(hidden)]
pub mod materials;
#[doc(hidden)]
pub mod pipeline;
mod pipeline_cache;
mod surface;
#[doc(hidden)]
pub mod textures;
#[doc(hidden)]
pub mod viewport;

use gpu::Gpu;
use std::sync::Arc;
use winit::{dpi::PhysicalSize, window::Window};

pub use diagnostics::{ImageUploadMeasurement, RendererStage, RendererTiming};
pub use embed::{
    EmbedAlpha, EmbedColorSpace, EmbedError, EmbedLoad, EmbedOptions, EmbedViewport,
    EmbeddedRenderer, PhysicalRect, RecordReport,
};
pub use error::RenderError;

/// Choose between synchronized presentation and the lowest available latency.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationMode {
    /// Wait for monitor refreshes and queue frames without tearing.
    #[default]
    Vsync,
    /// Present immediately when supported; may tear. Falls back to mailbox,
    /// then synchronized presentation on platforms that require it.
    Immediate,
}

impl PresentationMode {
    pub(super) fn apply(self, config: &mut wgpu::SurfaceConfiguration) {
        (config.present_mode, config.desired_maximum_frame_latency) = match self {
            Self::Vsync => (wgpu::PresentMode::Fifo, 2),
            Self::Immediate => (wgpu::PresentMode::AutoNoVsync, 1),
        };
    }
}

/// Presentation result. Retry transient failures later; wait for events while dormant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderStatus {
    Presented,
    Retry,
    Dormant,
}

/// GPU activity counters. Exposure redraws increase presents without uploading meshes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RendererStats {
    pub presented_frames: u64,
    pub geometry_uploads: u64,
    pub texture_uploads: u64,
    pub geometry_upload_bytes: u64,
    pub texture_creations: u64,
    pub texture_reuploads: u64,
    pub texture_upload_bytes: u64,
    pub texture_evictions: u64,
    pub draw_calls: u64,
    /// Estimate from dimensions and RGBA8 format, not measured VRAM.
    pub image_resident_bytes_estimate: u64,
    /// Draws with a user material, summed over presented frames.
    pub material_draws: u64,
    /// Material pipelines built and builds the device rejected, over the device's lifetime.
    pub material_pipeline_builds: u64,
    pub material_pipeline_failures: u64,
    /// Bytes of material uniform blocks written to the GPU by this renderer.
    pub material_uniform_bytes: u64,
    /// Backdrop effect commands drawn, summed over presented frames.
    pub blur_effects: u64,
    /// Groups of effects whose filter passes were shared, summed over frames.
    pub blur_batches: u64,
    /// Groups whose backdrop was unchanged and whose filtered result was reused.
    pub blur_batches_reused: u64,
    /// Render passes and canvas copies of the backdrop filter (not the scene itself).
    pub blur_passes: u64,
    /// Texels written by those passes and copies.
    pub blur_pixels: u64,
    /// Estimate of the bytes of the filter's textures and buffers now: zero when freed.
    pub blur_target_bytes: u64,
}

/// A renderer bound to one winit window. Owning an `Arc<Window>` makes surface
/// lifetime management safe without raw handles or unsafe code.
///
/// Further windows get their own renderer through [`Renderer::create_sibling`]. Siblings
/// share the device, queue, pipelines and the GPU texture store (glyph atlas pages and
/// images); each keeps its own surface, uniform, geometry buffers and blur targets.
///
/// Everything that belongs to the device rather than the window (buffers, textures,
/// pipelines, counters) lives in a part the window-less
/// [`EmbeddedRenderer`] uses as well; this type adds the surface, its configuration and
/// recovery.
pub struct Renderer {
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    window: Arc<Window>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    config: wgpu::SurfaceConfiguration,
    presentation_mode: PresentationMode,
    attachment_format: wgpu::TextureFormat,
    physical_size: PhysicalSize<u32>,
    pipelines: Arc<pipeline::PipelineSet>,
}

impl Renderer {
    pub fn presentation_mode(&self) -> PresentationMode {
        self.presentation_mode
    }

    /// Change presentation without rebuilding the pipeline or invalidating caches.
    /// The host should request a redraw after changing the mode.
    pub fn set_presentation_mode(&mut self, mode: PresentationMode) {
        if self.presentation_mode == mode {
            return;
        }
        self.presentation_mode = mode;
        mode.apply(&mut self.config);
        if self.physical_size.width > 0 && self.physical_size.height > 0 {
            self.surface.configure(&self.gpu.device, &self.config);
        }
    }

    pub fn stats(&self) -> RendererStats {
        self.gpu.stats
    }
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    /// Presentation modes advertised by this surface. Immediate presentation may
    /// fall back to Mailbox or Fifo; the requested mode alone does not prove support.
    pub fn supported_present_modes(&self) -> Vec<wgpu::PresentMode> {
        self.surface.get_capabilities(&self.adapter).present_modes
    }

    /// Wait for previously submitted GPU work, for diagnostics and benchmarks.
    /// This serializes CPU/GPU execution and should not be used in a normal frame loop.
    /// Completion does not mean the compositor has displayed the frame.
    pub fn wait_idle(&self, timeout: std::time::Duration) -> Result<(), RenderError> {
        self.gpu
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(timeout),
            })
            .map_err(|error| {
                RenderError::Validation(format!("waiting for GPU completion: {error}"))
            })?;
        if let Some(message) = self.gpu.lost() {
            return Err(RenderError::DeviceLost(message));
        }
        Ok(())
    }
}
