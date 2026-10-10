//! UI shader and render pipeline creation.

use crate::Vertex;

/// What a pipeline renders into: the color format of the view, the sample count and, when the
/// pass has one, the format of its depth/stencil attachment. A pipeline is only valid in
/// passes with exactly these attachments.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    pub format: wgpu::TextureFormat,
    pub samples: u32,
    pub depth: Option<wgpu::TextureFormat>,
}

impl Target {
    /// A single-sample color target without depth, like a window's surface.
    pub const fn color(format: wgpu::TextureFormat) -> Self {
        Self {
            format,
            samples: 1,
            depth: None,
        }
    }

    /// The depth/stencil state of pipelines drawn into a pass that has a depth attachment:
    /// the interface is flat, so it neither tests nor writes depth or stencil and can be
    /// drawn into a pass whose attachment is read-only too.
    pub fn depth_stencil(&self) -> Option<wgpu::DepthStencilState> {
        self.depth.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState {
                read_mask: 0,
                write_mask: 0,
                ..Default::default()
            },
            bias: Default::default(),
        })
    }
}

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
    let target = Target {
        samples: sample_count,
        ..Target::color(attachment_format)
    };
    create_in(device, viewport_layout, texture_layout, &target, fragment)
}

/// The pipeline of fragment entry point `fragment` for passes described by `target`.
pub fn create_in(
    device: &wgpu::Device,
    viewport_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    target: &Target,
    fragment: &str,
) -> wgpu::RenderPipeline {
    let shader = shader(device);
    pipeline(
        device,
        &shader,
        viewport_layout,
        texture_layout,
        target,
        fragment,
    )
}

/// The module of every built-in fragment entry point.
fn shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("zaxis shader"),
        source: wgpu::ShaderSource::Wgsl(
            concat!(
                include_str!("../shaders/common.wgsl"),
                "\n",
                include_str!("../shaders/ui.wgsl"),
                "\n",
                include_str!("../shaders/scroll_hint.wgsl")
            )
            .into(),
        ),
    })
}

/// The three built-in pipelines of one [`Target`]: plain, linear-filtered image and scroll
/// hint. They share one shader module.
pub struct PipelineSet {
    pub target: Target,
    pub plain: wgpu::RenderPipeline,
    pub image: wgpu::RenderPipeline,
    pub scroll_hint: wgpu::RenderPipeline,
}

/// Which built-in pipeline of a [`PipelineSet`] draws a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Image,
    ScrollHint,
}

impl Kind {
    /// Scroll hint, linear filtered managed image, or plain.
    pub fn of(command: &crate::DrawCommand, data: &crate::DrawData) -> Self {
        if command.scroll_hint {
            Self::ScrollHint
        } else if data
            .texture_options
            .get(&command.texture)
            .is_some_and(|o| o.managed && o.filter == crate::TextureFilter::Linear)
        {
            Self::Image
        } else {
            Self::Plain
        }
    }
}

impl PipelineSet {
    pub fn get(&self, kind: Kind) -> &wgpu::RenderPipeline {
        match kind {
            Kind::Plain => &self.plain,
            Kind::Image => &self.image,
            Kind::ScrollHint => &self.scroll_hint,
        }
    }

    /// The pipeline a command draws with when it is not a material.
    pub fn for_command(
        &self,
        command: &crate::DrawCommand,
        data: &crate::DrawData,
    ) -> &wgpu::RenderPipeline {
        self.get(Kind::of(command, data))
    }

    pub fn new(
        device: &wgpu::Device,
        viewport_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        target: Target,
    ) -> Self {
        let shader = shader(device);
        let make = |fragment| {
            pipeline(
                device,
                &shader,
                viewport_layout,
                texture_layout,
                &target,
                fragment,
            )
        };
        Self {
            plain: make("fs_main"),
            image: make("fs_image_linear"),
            scroll_hint: make("fs_scroll_hint"),
            target,
        }
    }
}

fn pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    viewport_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    target: &Target,
    fragment: &str,
) -> wgpu::RenderPipeline {
    let bind_group_layouts = [Some(viewport_layout), Some(texture_layout)];
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
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &ATTRIBUTES,
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target.format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: target.depth_stencil(),
        multisample: wgpu::MultisampleState {
            count: target.samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}
