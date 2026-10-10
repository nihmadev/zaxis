//! The backdrop blur as it was before the pyramid: a box prefilter, a Gaussian truncated at
//! three sigma and a copy of the area per effect. A faithful headless copy of the old
//! `render_blur`, kept as the baseline every new measure is compared with.

use super::gpu::{Gpu, FORMAT};
use wgpu::util::DeviceExt;
use zaxis::{shapes::Mesh, Border, Color, CornerRadius, Rect, Shape, Vec2, Vertex};

struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    group: wgpu::BindGroup,
}

fn target(gpu: &Gpu, [w, h]: [u32; 2], divisor: u32) -> Target {
    let texture = gpu.texture([w.div_ceil(divisor), h.div_ceil(divisor)], FORMAT);
    let view = texture.create_view(&Default::default());
    let group = gpu.group(&view);
    Target {
        texture,
        view,
        group,
    }
}

fn pass(
    gpu: &Gpu,
    encoder: &mut wgpu::CommandEncoder,
    output: &wgpu::TextureView,
    input: &wgpu::BindGroup,
    parameters: &wgpu::BindGroup,
    pipeline: &wgpu::RenderPipeline,
    region: [u32; 4],
) {
    let _ = gpu;
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("legacy blur pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: output,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    pass.set_scissor_rect(region[0], region[1], region[2], region[3]);
    pass.set_bind_group(0, input, &[]);
    pass.set_bind_group(1, parameters, &[]);
    pass.draw(0..3, 0..1);
}

/// The pipelines of the old chain, built once per device.
pub struct Legacy {
    parameter_layout: wgpu::BindGroupLayout,
    down: wgpu::RenderPipeline,
    gaussian: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
}

impl Legacy {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let parameter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
            label: Some("legacy blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("legacy_filter.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&gpu.texture_layout), Some(&parameter_layout)],
            immediate_size: 0,
        });
        let filter = |entry| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
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
                        format: FORMAT,
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
        let (down, gaussian) = (filter("fs_down"), filter("fs_blur"));
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../../../src/shaders/common.wgsl"),
                    "\n",
                    include_str!("legacy_composite.wgsl")
                )
                .into(),
            ),
        });
        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                Some(&gpu.viewport_layout),
                Some(&gpu.texture_layout),
                Some(&gpu.texture_layout),
            ],
            immediate_size: 0,
        });
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];
        let composite = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
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
                entry_point: Some("fs_backdrop"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
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
            parameter_layout,
            down,
            gaussian,
            composite,
        }
    }

    /// Blur `canvas` behind `rect` (logical pixels, rounded by `rounding`); return the canvas.
    pub fn blur(
        &self,
        gpu: &Gpu,
        size: [u32; 2],
        canvas: &[[u8; 4]],
        sigma: f32,
        scale: f32,
        rect: Rect,
        rounding: f32,
    ) -> Vec<[u8; 4]> {
        let device = &gpu.device;
        let (down, gaussian, composite) = (&self.down, &self.gaussian, &self.composite);
        let parameter_layout = &self.parameter_layout;
        let physical = sigma * scale;
        let downsample = if physical >= 8.0 {
            4
        } else if physical >= 4.0 {
            2
        } else {
            1
        };
        let canvas_texture = gpu.canvas(size, canvas);
        let canvas_view = canvas_texture.create_view(&Default::default());
        let backdrop = target(gpu, size, 1);
        let low = target(gpu, size, downsample);
        let scratch = target(gpu, size, downsample);
        let blurred = target(gpu, size, downsample);
        let samples = (3.0 * physical / downsample as f32).ceil().clamp(1.0, 32.0);
        let parameters = [
            [physical / size[0] as f32, 0.0, samples, 0.0],
            [0.0, physical / size[1] as f32, samples, 0.0],
        ]
        .map(|step| {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&step),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: parameter_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            })
        });
        // The composite of a rounded rectangle, replacing the canvas by the blurred copy.
        let mut mesh = Mesh::default();
        mesh.shape(
            &Shape::Rect {
                rect,
                rounding: CornerRadius::all(rounding),
                fill: Color::WHITE,
                border: Border::NONE,
            },
            scale,
        );
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        gpu.queue.write_buffer(
            &gpu.uniform,
            0,
            bytemuck::cast_slice(&[size[0] as f32 / scale, size[1] as f32 / scale, scale, 0.0]),
        );
        // The region logic of the old `render_blur`.
        let physical_size = winit::dpi::PhysicalSize::new(size[0], size[1]);
        let bounds = rect;
        let region = zaxis::renderer::viewport::scissor(bounds, scale, physical_size).unwrap();
        let filter_region = |[x, y, width, height]: [u32; 4]| {
            let d = downsample;
            let right = (x + width)
                .div_ceil(d)
                .saturating_add(1)
                .min(blurred.texture.width());
            let bottom = (y + height)
                .div_ceil(d)
                .saturating_add(1)
                .min(blurred.texture.height());
            let left = (x / d).saturating_sub(1);
            let top = (y / d).saturating_sub(1);
            [left, top, right - left, bottom - top]
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        let origin = wgpu::Origin3d {
            x: region[0],
            y: region[1],
            z: 0,
        };
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                origin,
                ..canvas_texture.as_image_copy()
            },
            wgpu::TexelCopyTextureInfo {
                origin,
                ..backdrop.texture.as_image_copy()
            },
            wgpu::Extent3d {
                width: region[2],
                height: region[3],
                depth_or_array_layers: 1,
            },
        );
        let padding = Vec2::splat(sigma * 3.0 + 2.0 * downsample as f32 / scale);
        let padded = Rect::from_min_max(bounds.min - padding, bounds.max + padding);
        let horizontal = zaxis::renderer::viewport::scissor(padded, scale, physical_size)
            .map(filter_region)
            .unwrap();
        let canvas_group = gpu.group(&canvas_view);
        let source = if downsample > 1 {
            pass(
                gpu,
                &mut encoder,
                &low.view,
                &canvas_group,
                &parameters[0],
                &down,
                horizontal,
            );
            &low.group
        } else {
            &canvas_group
        };
        pass(
            gpu,
            &mut encoder,
            &scratch.view,
            source,
            &parameters[0],
            &gaussian,
            horizontal,
        );
        pass(
            gpu,
            &mut encoder,
            &blurred.view,
            &scratch.group,
            &parameters[1],
            &gaussian,
            filter_region(region),
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("legacy composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &canvas_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_scissor_rect(region[0], region[1], region[2], region[3]);
            pass.set_pipeline(&composite);
            pass.set_bind_group(0, &gpu.viewport_group, &[]);
            pass.set_bind_group(1, &blurred.group, &[]);
            pass.set_bind_group(2, &backdrop.group, &[]);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }
        gpu.queue.submit([encoder.finish()]);
        gpu.read(&canvas_texture)
    }
}
