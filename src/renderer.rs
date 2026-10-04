//! wgpu backend with persistent buffers, texture versioning, and surface recovery.

mod blur;
#[cfg(test)]
mod combo_box_tests;
mod diagnostics;
mod error;
mod frame;
mod geometry;
#[cfg(test)]
mod image_tests;
mod init;
#[cfg(test)]
mod number_tests;
mod pipeline;
#[cfg(test)]
mod probe_tests;
#[cfg(test)]
mod scroll_hint_tests;
mod surface;
#[cfg(test)]
mod tests;
mod textures;
mod viewport;

use std::sync::{Arc, Mutex, OnceLock};
use textures::TextureStore;
use winit::{dpi::PhysicalSize, window::Window};

pub use diagnostics::{ImageUploadMeasurement, RendererStage, RendererTiming};
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
}

/// A renderer bound to one winit window. Owning an `Arc<Window>` makes surface
/// lifetime management safe without raw handles or unsafe code.
///
/// Further windows get their own renderer through [`Renderer::create_sibling`]. Siblings
/// share the device, queue, pipelines and the GPU texture store (glyph atlas pages and
/// images); each keeps its own surface, uniform, geometry buffers and blur targets.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    window: Arc<Window>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    presentation_mode: PresentationMode,
    attachment_format: wgpu::TextureFormat,
    physical_size: PhysicalSize<u32>,
    pipeline: wgpu::RenderPipeline,
    image_pipeline: wgpu::RenderPipeline,
    scroll_hint_pipeline: wgpu::RenderPipeline,
    backdrop_pipeline: wgpu::RenderPipeline,
    blur: Option<blur::BlurRenderer>,
    blur_pipelines: Arc<OnceLock<Arc<blur::BlurPipelines>>>,
    uniform: wgpu::Buffer,
    viewport_layout: wgpu::BindGroupLayout,
    viewport_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    nearest_sampler: wgpu::Sampler,
    store: Arc<Mutex<TextureStore>>,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    vertex_capacity: u64,
    index_capacity: u64,
    uploaded: Option<(u64, u64)>,
    uploaded_sizes: [usize; 2],
    viewport_value: [f32; 4],
    device_lost: Arc<Mutex<Option<String>>>,
    stats: RendererStats,
    diagnostics: diagnostics::Diagnostics,
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
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn stats(&self) -> RendererStats {
        self.stats
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
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(timeout),
            })
            .map_err(|error| {
                RenderError::Validation(format!("waiting for GPU completion: {error}"))
            })?;
        if let Some(message) = self
            .device_lost
            .lock()
            .expect("device callback mutex")
            .as_ref()
        {
            return Err(RenderError::DeviceLost(message.clone()));
        }
        Ok(())
    }
}
