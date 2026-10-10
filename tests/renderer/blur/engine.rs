//! Drives the backdrop engine without a window: the canvas under test is drawn as an image,
//! the effect command follows, and the engine's output is read back.

use super::gpu::{Gpu, FORMAT};
use std::{cell::RefCell, sync::Arc};
use wgpu::util::DeviceExt;
use zaxis::renderer::{
    blur::{self, BlurDevice, BlurPipelines, BlurRenderer, Frame, FrameStats},
    pipeline, textures,
};
use zaxis::{
    shapes::Mesh, vec2, Border, Color, CornerRadius, DrawCommand, DrawData, Rect, Shape, TextureId,
};

pub struct Engine {
    pipelines: Arc<BlurPipelines>,
    plain: wgpu::RenderPipeline,
    nearest: wgpu::Sampler,
    intermediate: wgpu::TextureFormat,
    renderer: RefCell<Option<BlurRenderer>>,
    revision: std::cell::Cell<u64>,
}

/// One frame to run: a canvas and a list of effect rectangles drawn over it, in order.
pub struct Shot<'a> {
    pub size: [u32; 2],
    pub scale: f32,
    pub canvas: &'a [[u8; 4]],
    pub effects: Vec<(Rect, f32, f32)>,
    pub clear: Color,
}

impl Engine {
    pub fn new(gpu: &Gpu) -> Self {
        let intermediate = blur::intermediate_format(&gpu.adapter, FORMAT);
        let dev = Self::device(gpu, intermediate);
        Self {
            pipelines: Arc::new(BlurPipelines::new(&dev)),
            plain: pipeline::create(
                &gpu.device,
                &gpu.viewport_layout,
                &gpu.texture_layout,
                FORMAT,
                1,
            ),
            nearest: textures::nearest_sampler(&gpu.device),
            intermediate,
            renderer: RefCell::new(None),
            revision: std::cell::Cell::new(0),
        }
    }

    fn device(gpu: &Gpu, intermediate: wgpu::TextureFormat) -> BlurDevice<'_> {
        BlurDevice {
            device: &gpu.device,
            queue: &gpu.queue,
            format: FORMAT,
            intermediate,
            texture_layout: &gpu.texture_layout,
            sampler: &gpu.sampler,
            viewport_layout: &gpu.viewport_layout,
            backdrop_layout: None,
        }
    }

    pub fn run(&self, gpu: &Gpu, shot: &Shot) -> (Vec<[u8; 4]>, FrameStats) {
        let device = &gpu.device;
        let [w, h] = shot.size;
        let scale = shot.scale;
        let mut mesh = Mesh::default();
        // The canvas as one image at 1:1 physical pixels, then the effect meshes.
        mesh.quad(
            Rect::from_min_size(vec2(0.0, 0.0), vec2(w as f32 / scale, h as f32 / scale)),
            Rect::from_min_size(vec2(0.0, 0.0), vec2(1.0, 1.0)),
            [1.0; 4],
            TextureId::WHITE,
        );
        let mut data = DrawData::new(vec2(w as f32 / scale, h as f32 / scale), scale);
        let full = Rect::from_min_size(vec2(0.0, 0.0), data.logical_size);
        let mut commands = vec![DrawCommand {
            indices: 0..6,
            clip_rect: full,
            texture: TextureId(7),
            ..Default::default()
        }];
        for (rect, rounding, sigma) in &shot.effects {
            let start = mesh.indices.len() as u32;
            mesh.shape(
                &Shape::Rect {
                    rect: *rect,
                    rounding: CornerRadius::all(*rounding),
                    fill: Color::WHITE,
                    border: Border::NONE,
                },
                scale,
            );
            commands.push(DrawCommand {
                indices: start..mesh.indices.len() as u32,
                clip_rect: full,
                blur: Some(*sigma),
                ..Default::default()
            });
        }
        data.vertices = mesh.vertices.clone();
        data.indices = mesh.indices.clone();
        data.commands = commands;
        // Each shot carries new canvas contents, which the draw data announces by revision.
        self.revision.set(self.revision.get() + 1);
        data.textures.push(zaxis::TextureImage {
            id: TextureId(7),
            size: shot.size,
            pixels: Arc::new(shot.canvas.as_flattened().to_vec()),
            revision: self.revision.get(),
        });
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
        let image = textures::create_texture(
            device,
            &gpu.queue,
            &gpu.texture_layout,
            &self.nearest,
            [w, h],
            shot.canvas.as_flattened(),
            0,
        );
        gpu.queue.write_buffer(
            &gpu.uniform,
            0,
            bytemuck::cast_slice(&[w as f32 / scale, h as f32 / scale, scale, 0.0]),
        );
        let output = gpu.texture(shot.size, FORMAT);
        let view = output.create_view(&Default::default());
        let dev = Self::device(gpu, self.intermediate);
        let mut slot = self.renderer.borrow_mut();
        if slot.as_ref().is_some_and(|r| r.size() != shot.size) {
            *slot = None;
        }
        let renderer = slot
            .get_or_insert_with(|| BlurRenderer::new(&dev, Arc::clone(&self.pipelines), shot.size));
        let mut encoder = device.create_command_encoder(&Default::default());
        let frame = Frame {
            data: &data,
            size: shot.size,
            clear: shot.clear,
            output: &view,
            vertices: &vertices,
            indices: &indices,
            viewport_group: &gpu.viewport_group,
        };
        let mut draw =
            |pass: &mut wgpu::RenderPass<'_>, index: usize, _: Option<&wgpu::BindGroup>| {
                pass.set_pipeline(&self.plain);
                pass.set_bind_group(1, &image.bind_group, &[]);
                pass.draw_indexed(data.commands[index].indices.clone(), 0, 0..1);
            };
        let stats = renderer.render(&dev, &mut encoder, &frame, &mut draw);
        gpu.queue.submit([encoder.finish()]);
        (gpu.read(&output), stats)
    }
}

/// The engine as a [`BlurFn`](super::metrics::BlurFn): one plain blur of `rect`.
pub fn blur_fn(engine: Engine) -> Box<super::metrics::BlurFn<'static>> {
    Box::new(move |gpu, size, canvas, sigma, scale, rect, rounding| {
        let shot = Shot {
            size,
            scale,
            canvas,
            effects: vec![(rect, rounding, sigma)],
            clear: Color::TRANSPARENT,
        };
        engine.run(gpu, &shot).0
    })
}
