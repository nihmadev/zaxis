//! An independent recording of draw data into a host pass: its own buffers, textures, uniform
//! and draw loop, built from the library's public pieces the way the window renderer's
//! tests do. `EmbeddedRenderer` must match it pixel for pixel.

use super::harness::{Host, Spec};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use zaxis::{
    renderer::{
        pipeline::{Kind, PipelineSet, Target},
        textures::{self, GpuTexture},
        viewport,
    },
    DrawData, PhysicalRect, TextureFilter, TextureId,
};

pub struct Reference {
    set: PipelineSet,
    group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    textures: HashMap<TextureId, GpuTexture>,
    commands: Vec<Command>,
    region: PhysicalRect,
    target: [u32; 2],
}

struct Command {
    range: std::ops::Range<u32>,
    scissor: [u32; 4],
    texture: TextureId,
    kind: Kind,
}

impl Reference {
    /// Everything needed to draw `data` into `region` of a target of the size of `spec`,
    /// uploaded now: buffers, textures and the uniform.
    pub fn prepare(host: &Host, spec: &Spec, data: &DrawData, region: PhysicalRect) -> Self {
        let device = &host.device;
        let (viewport_layout, uniform, group) = viewport::create_bindings(device);
        let (texture_layout, sampler) = textures::create_bindings(device);
        let nearest = textures::nearest_sampler(device);
        host.queue.write_buffer(
            &uniform,
            0,
            bytemuck::cast_slice(&[
                data.logical_size.x,
                data.logical_size.y,
                data.scale_factor,
                0.0,
                region.x as f32,
                region.y as f32,
                0.0,
                0.0,
            ]),
        );
        let mut textures: HashMap<_, _> = [(
            TextureId::WHITE,
            textures::create_texture(
                device,
                &host.queue,
                &texture_layout,
                &sampler,
                [1, 1],
                &[255; 4],
                0,
            ),
        )]
        .into();
        for image in &data.textures {
            let nearest_filter = data
                .texture_options
                .get(&image.id)
                .is_some_and(|o| o.filter == TextureFilter::Nearest);
            textures.insert(
                image.id,
                textures::create_texture(
                    device,
                    &host.queue,
                    &texture_layout,
                    if nearest_filter { &nearest } else { &sampler },
                    image.size,
                    &image.pixels,
                    image.revision,
                ),
            );
        }
        let init = |contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents,
                usage,
            })
        };
        let physical = PhysicalSize::new(region.width, region.height);
        let commands = data
            .commands
            .iter()
            .filter(|c| !c.indices.is_empty() && c.blur.is_none())
            .filter_map(|c| {
                let [x, y, w, h] = viewport::scissor(c.clip_rect, data.scale_factor, physical)?;
                Some(Command {
                    range: c.indices.clone(),
                    scissor: [region.x + x, region.y + y, w, h],
                    texture: c.texture,
                    kind: Kind::of(c, data),
                })
            })
            .collect();
        let target = Target {
            format: spec.view_format(),
            samples: spec.samples,
            depth: spec.depth.then_some(super::harness::DEPTH),
        };
        Self {
            set: PipelineSet::new(device, &viewport_layout, &texture_layout, target),
            group,
            vertices: init(
                bytemuck::cast_slice(&data.vertices),
                wgpu::BufferUsages::VERTEX,
            ),
            indices: init(
                bytemuck::cast_slice(&data.indices),
                wgpu::BufferUsages::INDEX,
            ),
            textures,
            commands,
            region,
            target: spec.size,
        }
    }

    pub fn record(&self, pass: &mut wgpu::RenderPass<'_>) {
        let r = self.region;
        pass.set_viewport(
            r.x as f32,
            r.y as f32,
            r.width as f32,
            r.height as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        for command in &self.commands {
            let [x, y, w, h] = command.scissor;
            pass.set_scissor_rect(x, y, w, h);
            pass.set_pipeline(self.set.get(command.kind));
            pass.set_bind_group(1, &self.textures[&command.texture].bind_group, &[]);
            pass.draw_indexed(command.range.clone(), 0, 0..1);
        }
        pass.set_viewport(
            0.0,
            0.0,
            self.target[0] as f32,
            self.target[1] as f32,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(0, 0, self.target[0], self.target[1]);
    }
}
