//! The host's own 3D scene: a lit cube with depth testing and multisampling.

use crate::{gpu::Gpu, model::Model, targets::DEPTH};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

const SHADER: &str = "
struct Uniforms { mvp: mat4x4<f32>, model: mat4x4<f32> }
@group(0) @binding(0) var<uniform> u: Uniforms;
struct Out { @builtin(position) position: vec4<f32>, @location(0) color: vec3<f32> }
const COLORS = array<vec3<f32>, 6>(
    vec3(0.9, 0.3, 0.25), vec3(0.95, 0.7, 0.2), vec3(0.3, 0.8, 0.4),
    vec3(0.25, 0.65, 0.9), vec3(0.6, 0.4, 0.9), vec3(0.85, 0.85, 0.9));
const CORNER = array<u32, 6>(0u, 1u, 2u, 2u, 1u, 3u);
@vertex fn vs(@builtin(vertex_index) vertex: u32) -> Out {
    let face = vertex / 6u;
    let corner = CORNER[vertex % 6u];
    let axis = face / 2u;
    let sign = f32(face % 2u) * 2.0 - 1.0;
    let a = f32(corner & 1u) * 2.0 - 1.0;
    let b = f32(corner >> 1u) * 2.0 - 1.0;
    var p = vec3<f32>(0.0);
    p[axis] = sign;
    p[(axis + 1u) % 3u] = a;
    p[(axis + 2u) % 3u] = b;
    var n = vec3<f32>(0.0);
    n[axis] = sign;
    let light = normalize(vec3<f32>(0.4, 0.8, 0.5));
    let lit = 0.35 + 0.65 * max(dot(normalize((u.model * vec4<f32>(n, 0.0)).xyz), light), 0.0);
    return Out(u.mvp * vec4<f32>(p, 1.0), COLORS[face] * lit);
}
@fragment fn fs(in: Out) -> @location(0) vec4<f32> { return vec4<f32>(in.color, 1.0); }
";

pub struct Scene {
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
}

impl Scene {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat, samples: u32) -> Self {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cube"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cube"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube uniforms"),
            contents: &[0; 128],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cube"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Self {
            pipeline,
            uniform,
            group,
        }
    }

    /// Upload the transform for `model` on a target of `size`.
    pub fn update(&self, gpu: &Gpu, model: &Model, size: [u32; 2]) {
        let aspect = size[0] as f32 / size[1].max(1) as f32;
        let projection = glam::camera::rh::proj::directx::perspective(0.9, aspect, 0.1, 50.0);
        let view =
            glam::camera::rh::view::look_at_mat4(Vec3::new(2.6, 2.2, 4.4), Vec3::ZERO, Vec3::Y);
        let world = Mat4::from_rotation_y(model.angle)
            * Mat4::from_rotation_x(model.angle * 0.37)
            * Mat4::from_scale(Vec3::splat(model.size));
        let mut bytes = [0.0f32; 32];
        bytes[..16].copy_from_slice(&(projection * view * world).to_cols_array());
        bytes[16..].copy_from_slice(&world.to_cols_array());
        gpu.queue
            .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&bytes));
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..36, 0..1);
    }
}
