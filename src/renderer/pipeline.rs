//! UI shader and render pipeline creation.

use crate::Vertex;

pub fn create(
    device: &wgpu::Device,
    viewport_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    attachment_format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    create_with_fragment(
        device,
        viewport_layout,
        texture_layout,
        attachment_format,
        sample_count,
        "fs_main",
    )
}

pub fn create_with_fragment(
    device: &wgpu::Device,
    viewport_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    attachment_format: wgpu::TextureFormat,
    sample_count: u32,
    fragment: &str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("zaxis shader"),
        source: wgpu::ShaderSource::Wgsl(
            concat!(
                include_str!("../shaders/ui.wgsl"),
                "\n",
                include_str!("../shaders/scroll_hint.wgsl")
            )
            .into(),
        ),
    });
    let mut bind_group_layouts = vec![Some(viewport_layout), Some(texture_layout)];
    if fragment == "fs_backdrop" {
        bind_group_layouts.push(Some(texture_layout));
    }
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("zaxis pipeline layout"),
        bind_group_layouts: &bind_group_layouts,
        immediate_size: 0,
    });
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("zaxis pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &ATTRIBUTES,
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(fragment),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: attachment_format,
                // Backdrop effects replace the existing background, including alpha.
                blend: (fragment != "fs_backdrop")
                    .then_some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}
