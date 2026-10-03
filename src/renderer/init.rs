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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(Arc::clone(&window)),
        ));
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(RenderError::CreateSurface)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(RenderError::RequestAdapter)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("zaxis device"),
                required_features: adapter.features()
                    & (wgpu::Features::TIMESTAMP_QUERY
                        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
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
        let sample_count = if adapter
            .get_texture_format_features(attachment_format)
            .flags
            .sample_count_supported(4)
        {
            4
        } else {
            1
        };
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let (viewport_layout, uniform, viewport_group) = viewport::create_bindings(&device);
        let (texture_layout, sampler) = textures::create_bindings(&device);
        let nearest_sampler = textures::nearest_sampler(&device);
        let image_pipeline = pipeline::create_with_fragment(
            &device,
            &viewport_layout,
            &texture_layout,
            attachment_format,
            sample_count,
            "fs_image_linear",
        );
        let pipeline = pipeline::create(
            &device,
            &viewport_layout,
            &texture_layout,
            attachment_format,
            sample_count,
        );
        let vertices = create_buffer(&device, 256, wgpu::BufferUsages::VERTEX, "zaxis vertices");
        let backdrop_pipeline = pipeline::create_with_fragment(
            &device,
            &viewport_layout,
            &texture_layout,
            attachment_format,
            sample_count,
            "fs_backdrop",
        );
        let scroll_hint_pipeline = pipeline::create_with_fragment(
            &device,
            &viewport_layout,
            &texture_layout,
            attachment_format,
            sample_count,
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
            sample_count,
            msaa_view: None,
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
