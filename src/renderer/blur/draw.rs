//! The drawing half of a frame: texture and block preparation, segments of commands into the
//! canvas, the composite of effects and the final copy.

use super::plan::Plan;
use super::{
    params::level_size, uniforms::*, Blocks, BlurDevice, BlurRenderer, DrawPlain, Frame, Offsets,
    Segment, Set, MAX_SETS,
};
use crate::{renderer::viewport::scissor, Color};
use std::ops::Range;
use winit::dpi::PhysicalSize;

impl BlurRenderer {
    /// Create the textures every set needs for the jobs mapped to it.
    pub(super) fn prepare_sets(
        &mut self,
        dev: &BlurDevice,
        plan: &Plan,
        set_of: &[usize],
        materials: &[bool],
    ) {
        let used = set_of.iter().copied().max().map_or(0, |m| m + 1);
        self.sets
            .resize_with(self.sets.len().max(used), Set::default);
        for (index, set) in self.sets.iter_mut().enumerate().take(used) {
            let jobs = || {
                plan.jobs
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| set_of[*j] == index)
            };
            let depth = jobs().map(|(_, j)| j.filter.level).max().unwrap_or(0);
            let mut last = vec![false; super::params::MAX_LEVEL as usize + 1];
            for (_, job) in jobs() {
                last[job.filter.level as usize] = true;
            }
            let resolved = jobs().any(|(j, _)| materials[j]);
            set.ensure(dev, self.size, depth, &last, resolved);
        }
        debug_assert!(self.sets.len() <= MAX_SETS);
    }

    /// Write the parameter blocks of every job and upload them.
    pub(super) fn write_blocks(&mut self, dev: &BlurDevice, plan: &Plan) -> Blocks {
        let unused = self.uniforms.filter(&bytemuck::Zeroable::zeroed());
        let mut filters = Vec::with_capacity(plan.jobs.len());
        let mut effects = Vec::with_capacity(plan.jobs.len());
        for job in &plan.jobs {
            let [w, h] = level_size(self.size, job.filter.level);
            let block = |step: [f32; 2], inverse_scale: f32| FilterBlock {
                step,
                sigma: job.filter.sigma,
                radius: job.filter.radius as f32,
                inverse_scale,
                pad: [0.0; 3],
            };
            filters.push(Offsets {
                horizontal: self.uniforms.filter(&block([1.0 / w as f32, 0.0], 1.0)),
                vertical: self.uniforms.filter(&block([0.0, 1.0 / h as f32], 1.0)),
                resolve: self
                    .uniforms
                    .filter(&block([0.0; 2], 1.0 / (1u32 << job.filter.level) as f32)),
            });
            effects.push(self.uniforms.effect(&EffectBlock::new(job.filter.level)));
        }
        self.uniforms.upload(dev, &self.pipelines);
        Blocks {
            filters,
            effects,
            unused,
        }
    }

    /// Draw commands `range` into the canvas.
    pub(super) fn segment(
        &self,
        segment: &mut Segment,
        encoder: &mut wgpu::CommandEncoder,
        draw: &mut DrawPlain,
        range: Range<usize>,
        clear: Option<Color>,
    ) {
        let frame = segment.frame;
        let data = frame.data;
        let load = match clear {
            Some(color) => {
                let c = color.linear();
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: f64::from(c[0]),
                    g: f64::from(c[1]),
                    b: f64::from(c[2]),
                    a: f64::from(c[3]),
                })
            }
            None => wgpu::LoadOp::Load,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zaxis backdrop segment"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.canvas.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        if data.indices.is_empty() {
            return;
        }
        pass.set_bind_group(0, frame.viewport_group, &[]);
        pass.set_vertex_buffer(0, frame.vertices.slice(..));
        pass.set_index_buffer(frame.indices.slice(..), wgpu::IndexFormat::Uint32);
        let physical = PhysicalSize::new(frame.size[0], frame.size[1]);
        for index in range {
            let command = &data.commands[index];
            if command.indices.is_empty() {
                continue;
            }
            let Some([x, y, w, h]) = scissor(command.clip_rect, data.scale_factor, physical) else {
                continue;
            };
            if command.blur.is_none() {
                pass.set_scissor_rect(x, y, w, h);
                draw(&mut pass, index, None);
                continue;
            }
            let Some(job) = segment.plan.job_of(index) else {
                continue;
            };
            let set = &self.sets[segment.set_of[job]];
            pass.set_scissor_rect(x, y, w, h);
            if command.material.is_some() {
                draw(&mut pass, index, set.material_group.as_ref());
                continue;
            }
            let level = segment.plan.jobs[job].filter.level as usize;
            let result = set.result[level].as_ref().expect("filtered level");
            pass.set_pipeline(&self.pipelines.composite);
            pass.set_bind_group(1, &result.group, &[]);
            pass.set_bind_group(2, &set.snapshot.as_ref().expect("snapshot").group, &[]);
            pass.set_bind_group(
                3,
                self.uniforms.effect_group(),
                &[segment.blocks.effects[job]],
            );
            pass.draw_indexed(command.indices.clone(), 0, 0..1);
        }
    }

    /// Fill the canvas with the `size` texels of `base` that start at `origin`.
    pub(super) fn copy_base(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        base: &wgpu::Texture,
        origin: [u32; 2],
    ) {
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                origin: wgpu::Origin3d {
                    x: origin[0],
                    y: origin[1],
                    z: 0,
                },
                ..base.as_image_copy()
            },
            self.canvas.texture.as_image_copy(),
            wgpu::Extent3d {
                width: self.size[0],
                height: self.size[1],
                depth_or_array_layers: 1,
            },
        );
    }

    /// Copy the canvas into the output, or into `region` of it, keeping the rest.
    pub(super) fn finish(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        unused: u32,
        region: Option<[u32; 4]>,
        clear: Option<wgpu::Color>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zaxis backdrop present"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame.output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: match (clear, region) {
                        (Some(color), _) => wgpu::LoadOp::Clear(color),
                        (None, Some(_)) => wgpu::LoadOp::Load,
                        (None, None) => wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        if let Some([x, y, w, h]) = region {
            pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0.0, 1.0);
            pass.set_scissor_rect(x, y, w, h);
        }
        pass.set_pipeline(&self.pipelines.copy);
        pass.set_bind_group(0, &self.canvas.group, &[]);
        pass.set_bind_group(1, self.uniforms.filter_group(), &[unused]);
        pass.draw(0..3, 0..1);
    }
}
