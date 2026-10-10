//! Window/GPU initialization and renderer assembly.

use super::{
    adapter,
    gpu::{self, Gpu},
    pipeline::Target,
    PresentationMode, RenderError, Renderer,
};
use std::sync::{Arc, Mutex};
use winit::{dpi::PhysicalSize, window::Window};

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Result<Self, RenderError> {
        Self::new_with_presentation_mode(window, PresentationMode::default()).await
    }

    pub async fn new_with_presentation_mode(
        window: Arc<Window>,
        presentation_mode: PresentationMode,
    ) -> Result<Self, RenderError> {
        Self::new_with_backends(window, presentation_mode, None).await
    }

    /// Like [`new_with_presentation_mode`](Self::new_with_presentation_mode), trying only
    /// `backends`. `None` keeps the platform's default order: DX12 first on Windows,
    /// `WGPU_BACKEND` honored, WebGPU before WebGL2 in a browser.
    pub async fn new_with_backends(
        window: Arc<Window>,
        presentation_mode: PresentationMode,
        backends: Option<wgpu::Backends>,
    ) -> Result<Self, RenderError> {
        let (instance, surface, adapter) = adapter::select(&window, backends).await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("zaxis device"),
                required_features: adapter.features()
                    & (wgpu::Features::TIMESTAMP_QUERY
                        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
                required_limits: adapter::baseline_limits(&adapter)
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                ..Default::default()
            })
            .await
            .map_err(RenderError::RequestDevice)?;
        let device_lost = Arc::new(Mutex::new(None));
        let callback = Arc::clone(&device_lost);
        device.set_device_lost_callback(move |reason, message| {
            if reason != wgpu::DeviceLostReason::Destroyed {
                if let Ok(mut error) = callback.lock() {
                    *error = Some(message);
                }
            }
        });
        let size = window.inner_size();
        let (config, attachment_format) =
            surface_config(&surface, &adapter, size, presentation_mode)?;
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let gpu = Gpu::new(device, queue, gpu::half_float(&adapter), device_lost);
        let pipelines = gpu
            .pipelines
            .lock()
            .expect("pipeline cache mutex")
            .get(Target::color(attachment_format));
        let mut renderer = Self {
            gpu,
            surface,
            window,
            instance,
            adapter,
            config,
            presentation_mode,
            attachment_format,
            physical_size: size,
            pipelines,
        };
        renderer.resize(size)?;
        if let Some(error) = validation.pop().await {
            return Err(RenderError::Validation(error.to_string()));
        }
        Ok(renderer)
    }

    /// A renderer for another window on the same GPU device. Pipelines, samplers, the
    /// device-loss state and the texture store (glyph atlas pages, images) are shared;
    /// the surface, viewport uniform, geometry buffers and blur targets belong to the new
    /// window and are freed when the sibling is dropped.
    ///
    /// Every draw data presented through a family of siblings must allocate its
    /// [`TextureId`](crate::TextureId)s from one source, as contexts created from the same
    /// [`SharedResources`](crate::SharedResources) do. The adapter chosen for the first
    /// window must support the new surface; otherwise this returns an error and the other
    /// renderers are unaffected. Cheap and synchronous: no device or pipeline is created.
    pub fn create_sibling(
        &self,
        window: Arc<Window>,
        presentation_mode: PresentationMode,
    ) -> Result<Self, RenderError> {
        let surface = self
            .instance
            .create_surface(Arc::clone(&window))
            .map_err(RenderError::CreateSurface)?;
        if !self.adapter.is_surface_supported(&surface) {
            return Err(RenderError::UnsupportedSurface);
        }
        let size = window.inner_size();
        let (config, attachment_format) =
            surface_config(&surface, &self.adapter, size, presentation_mode)?;
        if attachment_format != self.attachment_format {
            return Err(RenderError::UnsupportedSurface);
        }
        let mut renderer = Self {
            gpu: self.gpu.sibling(),
            surface,
            window,
            instance: self.instance.clone(),
            adapter: self.adapter.clone(),
            config,
            presentation_mode,
            attachment_format,
            physical_size: size,
            pipelines: Arc::clone(&self.pipelines),
        };
        renderer.resize(size)?;
        Ok(renderer)
    }

    /// The native window this renderer presents to.
    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }
}

/// Surface configuration plus the sRGB attachment format, chosen the same way for every window.
fn surface_config(
    surface: &wgpu::Surface<'static>,
    adapter: &wgpu::Adapter,
    size: PhysicalSize<u32>,
    presentation_mode: PresentationMode,
) -> Result<(wgpu::SurfaceConfiguration, wgpu::TextureFormat), RenderError> {
    let mut config = surface
        .get_default_config(adapter, size.width.max(1), size.height.max(1))
        .ok_or(RenderError::UnsupportedSurface)?;
    let capabilities = surface.get_capabilities(adapter);
    config.format = capabilities
        .formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| {
            capabilities.formats.iter().copied().find(|format| {
                matches!(
                    format,
                    wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
                )
            })
        })
        .ok_or(RenderError::UnsupportedSurface)?;
    // Use an sRGB view even when the surface only advertises the unorm base
    // format. Encoding after linear blending preserves translucent edges.
    let attachment_format = config.format.add_srgb_suffix();
    if attachment_format != config.format {
        config.view_formats.push(attachment_format);
    }
    presentation_mode.apply(&mut config);
    config.alpha_mode = capabilities
        .alpha_modes
        .first()
        .copied()
        .ok_or(RenderError::UnsupportedSurface)?;
    Ok((config, attachment_format))
}
