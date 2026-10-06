//! Pipelines and layout of the Gaussian passes, created once per device.

use super::BlurPipelines;
use crate::renderer::Renderer;

impl BlurPipelines {
    pub(super) fn new(renderer: &Renderer) -> Self {
        let device = &renderer.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zaxis blur parameters"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("zaxis Gaussian blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/blur.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zaxis blur pipeline layout"),
            bind_group_layouts: &[Some(&renderer.texture_layout), Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("zaxis blur fullscreen pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: renderer.attachment_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            layout,
            gaussian: pipeline("fs_blur"),
            down: pipeline("fs_down"),
            copy: pipeline("fs_copy"),
        }
    }
}
