//! Window/GPU initialization and renderer assembly.

use super::{
    geometry::create_buffer,
    pipeline,
    textures::{self, create_texture},
    viewport, PresentationMode, RenderError, Renderer, RendererStats,
};
use crate::TextureId;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use winit::window::Window;

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
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or(RenderError::UnsupportedSurface)?;
        let capabilities = surface.get_capabilities(&adapter);
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
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let (viewport_layout, uniform, viewport_group) = viewport::create_bindings(&device);
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
            uniform,
            viewport_group,
            texture_layout,
            sampler,
            nearest_sampler,
            textures: HashMap::from([(TextureId::WHITE, white)]),
            vertices,
            indices,
            vertex_capacity: 256,
            index_capacity: 256,
            uploaded: None,
            uploaded_sizes: [0; 2],
            texture_source: None,
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
