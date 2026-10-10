//! Pipelines and layouts of the backdrop filter, created once per device and shared by every
//! window on it.

use super::BlurDevice;
use crate::Vertex;

/// Bytes of a [`FilterBlock`](super::uniforms::FilterBlock) and an
/// [`EffectBlock`](super::uniforms::EffectBlock).
pub const FILTER_BYTES: u64 = 32;
pub const EFFECT_BYTES: u64 = 16;

pub struct BlurPipelines {
    pub filter_layout: wgpu::BindGroupLayout,
    pub effect_layout: wgpu::BindGroupLayout,
    /// One pyramid step, the Gaussian and the full-resolution reconstruction; all render to
    /// the intermediate format.
    pub down: wgpu::RenderPipeline,
    pub gaussian: wgpu::RenderPipeline,
    pub resolve: wgpu::RenderPipeline,
    /// The canvas to the surface.
    pub copy: wgpu::RenderPipeline,
    /// The composite of an effect command into the canvas.
    pub composite: wgpu::RenderPipeline,
}

fn dynamic_layout(device: &wgpu::Device, label: &str, size: u64) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(size),
            },
            count: None,
        }],
    })
}

impl BlurPipelines {
    pub fn new(dev: &BlurDevice) -> Self {
        let device = dev.device;
        let filter_layout = dynamic_layout(device, "zaxis backdrop filter block", FILTER_BYTES);
        let effect_layout = dynamic_layout(device, "zaxis backdrop effect block", EFFECT_BYTES);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("zaxis backdrop filter"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../../shaders/blur.wgsl"),
                    "\n",
                    include_str!("../../shaders/spline.wgsl")
                )
                .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zaxis backdrop filter layout"),
            bind_group_layouts: &[Some(dev.texture_layout), Some(&filter_layout)],
            immediate_size: 0,
        });
        let filter = |entry: &str, format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("zaxis backdrop filter pipeline"),
                layout: Some(&layout),
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
                        format,
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
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("zaxis backdrop composite"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../../shaders/common.wgsl"),
                    "\n",
                    include_str!("../../shaders/spline.wgsl"),
                    "\n",
                    include_str!("../../shaders/composite.wgsl")
                )
                .into(),
            ),
        });
        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zaxis backdrop composite layout"),
            bind_group_layouts: &[
                Some(dev.viewport_layout),
                Some(dev.texture_layout),
                Some(dev.texture_layout),
                Some(&effect_layout),
            ],
            immediate_size: 0,
        });
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];
        let composite = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("zaxis backdrop composite pipeline"),
            layout: Some(&composite_layout),
            vertex: wgpu::VertexState {
                module: &composite_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ATTRIBUTES,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &composite_shader,
                entry_point: Some("fs_composite"),
                compilation_options: Default::default(),
                // The effect replaces what is behind the shape, alpha included.
                targets: &[Some(wgpu::ColorTargetState {
                    format: dev.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            down: filter("fs_down", dev.intermediate),
            gaussian: filter("fs_gaussian", dev.intermediate),
            resolve: filter("fs_resolve", dev.intermediate),
            copy: filter("fs_copy", dev.format),
            composite,
            filter_layout,
            effect_layout,
        }
    }
}
