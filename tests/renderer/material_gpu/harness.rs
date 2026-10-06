//! A device, a render target and readback for material draws, shared by the pixel tests.

use std::collections::HashMap;
use zaxis::renderer::{
    materials::{MaterialPipelines, MaterialUniforms},
    pipeline, textures, viewport,
};
use zaxis::{vec2, Context, DrawData, Material, ParamKind, Rect, TextureFilter, TextureId, Window};

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const SIZE: [u32; 2] = [512, 256];

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub viewport_layout: wgpu::BindGroupLayout,
    pub uniform: wgpu::Buffer,
    pub viewport_group: wgpu::BindGroup,
    pub texture_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub pipelines: MaterialPipelines,
    pub uniforms: MaterialUniforms,
    pub adapter: wgpu::Adapter,
}

pub fn gpu(name: &str) -> Option<Gpu> {
    if std::env::var_os("ZAXIS_SKIP_GPU_TESTS").is_some() {
        eprintln!("SKIPPED {name}: ZAXIS_SKIP_GPU_TESTS");
        return None;
    }
    let adapter =
        pollster::block_on(wgpu::Instance::default().request_adapter(&Default::default()))
            .inspect_err(|_| eprintln!("SKIPPED {name}: no adapter"))
            .ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .inspect_err(|_| eprintln!("SKIPPED {name}: no device"))
        .ok()?;
    let (viewport_layout, uniform, viewport_group) = viewport::create_bindings(&device);
    let (texture_layout, sampler) = textures::create_bindings(&device);
    let pipelines = MaterialPipelines::new(&device, &queue, &viewport_layout, &texture_layout);
    let uniforms = MaterialUniforms::new(&device, &pipelines.params_layout);
    Some(Gpu {
        device,
        queue,
        viewport_layout,
        uniform,
        viewport_group,
        texture_layout,
        sampler,
        pipelines,
        uniforms,
        adapter,
    })
}

/// Which draws of `data` to render.
#[derive(Clone, Copy, PartialEq)]
pub enum Only {
    Materials,
    Plain,
}

impl Gpu {
    /// Render the chosen commands of `data` at `scale` and read the target back as RGBA.
    pub fn render(
        &mut self,
        data: &DrawData,
        scale: f32,
        samples: u32,
        only: Only,
        clear: [f64; 4],
    ) -> Vec<[u8; 4]> {
        use wgpu::util::DeviceExt;
        let device = &self.device;
        let [width, height] = SIZE;
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
        let make = |count| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: count,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage,
                view_formats: &[],
            })
        };
        let target = make(1);
        let multisampled = (samples > 1).then(|| make(samples));
        let view = target.create_view(&Default::default());
        let msaa_view = multisampled
            .as_ref()
            .map(|t| t.create_view(&Default::default()));
        self.queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::cast_slice(&[width as f32 / scale, height as f32 / scale, scale, 0.0]),
        );
        let mut bindings = HashMap::new();
        bindings.insert(
            TextureId::WHITE,
            textures::create_texture(
                device,
                &self.queue,
                &self.texture_layout,
                &self.sampler,
                [1, 1],
                &[255; 4],
                0,
            ),
        );
        let nearest = textures::nearest_sampler(device);
        for image in &data.textures {
            let filter = data
                .texture_options
                .get(&image.id)
                .map(|o| o.filter)
                .unwrap_or_default();
            let sampler = if filter == TextureFilter::Nearest {
                &nearest
            } else {
                &self.sampler
            };
            bindings.insert(
                image.id,
                textures::create_texture(
                    device,
                    &self.queue,
                    &self.texture_layout,
                    sampler,
                    image.size,
                    &image.pixels,
                    image.revision,
                ),
            );
        }
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let plain = pipeline::create(
            device,
            &self.viewport_layout,
            &self.texture_layout,
            FORMAT,
            samples,
        );
        self.pipelines.begin_frame();
        self.uniforms
            .upload(device, &self.queue, &self.pipelines.params_layout, data);
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa_view.as_ref().unwrap_or(&view),
                    depth_slice: None,
                    resolve_target: msaa_view.as_ref().map(|_| &view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0],
                            g: clear[1],
                            b: clear[2],
                            a: clear[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_bind_group(0, &self.viewport_group, &[]);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            for command in &data.commands {
                let is_material = command.material.is_some();
                if is_material != (only == Only::Materials) || command.indices.is_empty() {
                    continue;
                }
                let Some([x, y, w, h]) = viewport::scissor(
                    command.clip_rect,
                    scale,
                    winit::dpi::PhysicalSize::new(width, height),
                ) else {
                    continue;
                };
                pass.set_scissor_rect(x, y, w, h);
                pass.set_bind_group(1, &bindings[&command.texture].bind_group, &[]);
                if let Some(draw) = &command.material {
                    let source = data.materials.iter().find(|s| s.id == draw.id).unwrap();
                    let pipeline = self
                        .pipelines
                        .pipeline(source, FORMAT, samples)
                        .unwrap_or_else(|| panic!("{:?}", self.pipelines.take_errors()));
                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(2, &self.pipelines.no_backdrop, &[]);
                    let offset = self.uniforms.offset(draw.uniforms.start).unwrap();
                    pass.set_bind_group(3, self.uniforms.group(), &[offset]);
                } else {
                    pass.set_pipeline(&plain);
                }
                pass.draw_indexed(command.indices.clone(), 0, 0..1);
            }
        }
        self.pipelines.trim();
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(width * height * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
            },
            extent,
        );
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        let bytes = readback.slice(..).get_mapped_range().unwrap();
        bytes.as_chunks::<4>().0.to_vec()
    }
}

pub fn context(scale: f32) -> Context {
    let mut c = Context::new();
    c.set_viewport(
        winit::dpi::PhysicalSize::new(SIZE[0], SIZE[1]),
        f64::from(scale),
    );
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}

/// One pass with `build` in a window covering the viewport.
pub fn scene(c: &mut Context, scale: f32, build: impl FnOnce(&mut zaxis::Ui<'_>)) -> DrawData {
    let logical = vec2(SIZE[0] as f32, SIZE[1] as f32) / scale;
    c.run(|c| {
        Window::new("m")
            .default_position(vec2(0.0, 0.0))
            .default_size(logical)
            .show(c, build);
    });
    copy(c.draw_data())
}

/// A copy of the frame, to render while the context goes on.
pub fn copy(data: &DrawData) -> DrawData {
    DrawData {
        vertices: data.vertices.clone(),
        indices: data.indices.clone(),
        commands: data.commands.clone(),
        logical_size: data.logical_size,
        scale_factor: data.scale_factor,
        revision: data.revision,
        source: data.source,
        textures: data.textures.clone(),
        texture_options: data.texture_options.clone(),
        materials: data.materials.clone(),
        material_uniforms: data.material_uniforms.clone(),
        ..DrawData::new(data.logical_size, data.scale_factor)
    }
}

pub fn at(pixels: &[[u8; 4]], x: u32, y: u32) -> [u8; 4] {
    pixels[(y * SIZE[0] + x) as usize]
}

pub fn near(a: [u8; 4], b: [u8; 4], tolerance: u8) -> bool {
    a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= tolerance)
}

pub const CELL: Rect = Rect {
    min: vec2(24.0, 64.0),
    max: vec2(104.0, 112.0),
};

pub fn flat(source: &str) -> Material {
    Material::new(
        "flat",
        format!("fn material(in: MaterialInput, p: Params) -> vec4<f32> {{ {source} }}"),
    )
    .param("level", ParamKind::F32)
}
