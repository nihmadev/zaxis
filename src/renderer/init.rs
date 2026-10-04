//! Window/GPU initialization and renderer assembly.

use super::{
    geometry::create_buffer,
    pipeline,
    textures::{self, create_texture, TextureStore},
    viewport, PresentationMode, RenderError, Renderer, RendererStats,
};
use std::sync::{Arc, Mutex, OnceLock};
use winit::{dpi::PhysicalSize, window::Window};

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Result<Self, RenderError> {
        Self::new_with_presentation_mode(window, PresentationMode::default()).await
    }

    pub async fn new_with_presentation_mode(
        window: Arc<Window>,
        presentation_mode: PresentationMode,
    ) -> Result<Self, RenderError> {
        let (instance, surface, adapter) = select_adapter(&window).await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("zaxis device"),
                required_features: adapter.features()
                    & (wgpu::Features::TIMESTAMP_QUERY
                        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
                required_limits: wgpu::Limits::downlevel_defaults()
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
        let viewport_layout = viewport::create_layout(&device);
        let (uniform, viewport_group) = viewport::create_uniform(&device, &viewport_layout);
        let (texture_layout, sampler) = textures::create_bindings(&device);
        let nearest_sampler = textures::nearest_sampler(&device);
        let image_pipeline = pipeline::create_with_fragment(
            &device,
            &viewport_layout,
            &texture_layout,
            attachment_format,
            1,
            "fs_image_linear",
        );
        let pipeline = pipeline::create(
            &device,
            &viewport_layout,
            &texture_layout,
            attachment_format,
            1,
        );
        let vertices = create_buffer(&device, 256, wgpu::BufferUsages::VERTEX, "zaxis vertices");
        let backdrop_pipeline = pipeline::create_with_fragment(
            &device,
            &viewport_layout,
            &texture_layout,
            attachment_format,
            1,
            "fs_backdrop",
        );
        let scroll_hint_pipeline = pipeline::create_with_fragment(
            &device,
            &viewport_layout,
            &texture_layout,
            attachment_format,
            1,
            "fs_scroll_hint",
        );
        let indices = create_buffer(&device, 256, wgpu::BufferUsages::INDEX, "zaxis indices");
        let white = create_texture(
            &device,
            &queue,
            &texture_layout,
            &sampler,
            [1, 1],
            &[255; 4],
            0,
        );
        let mut renderer = Self {
            surface,
            window,
            instance,
            adapter,
            device,
            queue,
            config,
            presentation_mode,
            attachment_format,
            physical_size: size,
            pipeline,
            image_pipeline,
            backdrop_pipeline,
            scroll_hint_pipeline,
            blur: None,
            blur_pipelines: Arc::new(OnceLock::new()),
            uniform,
            viewport_layout,
            viewport_group,
            texture_layout,
            sampler,
            nearest_sampler,
            store: Arc::new(Mutex::new(TextureStore::new(white))),
            vertices,
            indices,
            vertex_capacity: 256,
            index_capacity: 256,
            uploaded: None,
            uploaded_sizes: [0; 2],
            viewport_value: [1.0, 1.0, 0.0, 0.0],
            device_lost,
            stats: RendererStats::default(),
            diagnostics: Default::default(),
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
        self.store.lock().expect("texture store mutex").shared = true;
        let (uniform, viewport_group) =
            viewport::create_uniform(&self.device, &self.viewport_layout);
        let buffer = |usage, label| create_buffer(&self.device, 256, usage, label);
        let mut renderer = Self {
            surface,
            window,
            instance: self.instance.clone(),
            adapter: self.adapter.clone(),
            device: self.device.clone(),
            queue: self.queue.clone(),
            config,
            presentation_mode,
            attachment_format,
            physical_size: size,
            pipeline: self.pipeline.clone(),
            image_pipeline: self.image_pipeline.clone(),
            backdrop_pipeline: self.backdrop_pipeline.clone(),
            scroll_hint_pipeline: self.scroll_hint_pipeline.clone(),
            blur: None,
            blur_pipelines: Arc::clone(&self.blur_pipelines),
            uniform,
            viewport_layout: self.viewport_layout.clone(),
            viewport_group,
            texture_layout: self.texture_layout.clone(),
            sampler: self.sampler.clone(),
            nearest_sampler: self.nearest_sampler.clone(),
            store: Arc::clone(&self.store),
            vertices: buffer(wgpu::BufferUsages::VERTEX, "zaxis vertices"),
            indices: buffer(wgpu::BufferUsages::INDEX, "zaxis indices"),
            vertex_capacity: 256,
            index_capacity: 256,
            uploaded: None,
            uploaded_sizes: [0; 2],
            viewport_value: [1.0, 1.0, 0.0, 0.0],
            device_lost: Arc::clone(&self.device_lost),
            stats: RendererStats::default(),
            diagnostics: Default::default(),
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

/// DX12 needs far less driver memory than Vulkan on Windows, so it is tried
/// first. `WGPU_BACKEND` overrides the order; the default set is the fallback.
async fn select_adapter(
    window: &Arc<Window>,
) -> Result<(wgpu::Instance, wgpu::Surface<'static>, wgpu::Adapter), RenderError> {
    let explicit = std::env::var_os("WGPU_BACKEND").is_some();
    let mut attempts = vec![wgpu::Backends::from_env().unwrap_or_default()];
    if cfg!(windows) && !explicit {
        attempts.insert(0, wgpu::Backends::DX12);
    }
    let mut failure = None;
    for backends in attempts {
        let mut descriptor = wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(
            Arc::clone(window),
        ));
        descriptor.backends = backends;
        let instance = wgpu::Instance::new(descriptor);
        let surface = match instance.create_surface(Arc::clone(window)) {
            Ok(surface) => surface,
            Err(error) => {
                failure = Some(RenderError::CreateSurface(error));
                continue;
            }
        };
        match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
        {
            Ok(adapter) => return Ok((instance, surface, adapter)),
            Err(error) => failure = Some(RenderError::RequestAdapter(error)),
        }
    }
    Err(failure.expect("at least one backend set is tried"))
}
