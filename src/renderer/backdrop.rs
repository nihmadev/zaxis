//! The renderer's side of backdrop effects: the filter engine in `blur`, fed with this
//! renderer's device, buffers, pipelines and materials.

use super::{
    blur::{BlurDevice, BlurRenderer, Compose, Frame},
    gpu::Gpu,
    materials::{Draw, MaterialFrame},
    pipeline::PipelineSet,
    textures::TextureStore,
};
use crate::{Color, DrawData};

/// Frames without effects after which the targets are released.
const RELEASE_AFTER_FRAMES: u32 = 120;

/// Where and over what a frame with backdrop effects is drawn.
pub(super) struct BlurTarget<'a> {
    /// Single-sample view the frame ends up in.
    pub output: &'a wgpu::TextureView,
    /// Format of that view, which is also the canvas format.
    pub format: wgpu::TextureFormat,
    /// Physical size of the canvas: the window, or the region of an embedded target.
    pub size: [u32; 2],
    pub pipelines: &'a PipelineSet,
    pub compose: Compose<'a>,
}

impl Gpu {
    pub(super) fn render_blur(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        data: &DrawData,
        clear: Color,
        target: BlurTarget<'_>,
        store: &TextureStore,
        materials: &MaterialFrame,
    ) {
        self.blur_idle_frames = 0;
        let backdrop_layout = self
            .materials
            .lock()
            .expect("material pipelines mutex")
            .backdrop_layout
            .clone();
        let BlurTarget {
            output,
            format,
            size,
            pipelines: builtin,
            compose,
        } = target;
        let dev = BlurDevice {
            device: &self.device,
            queue: &self.queue,
            format,
            intermediate: self.intermediate(format),
            texture_layout: &self.texture_layout,
            sampler: &self.sampler,
            viewport_layout: &self.viewport_layout,
            backdrop_layout: Some(&backdrop_layout),
        };
        let pipelines = self
            .blur_pipelines
            .lock()
            .expect("backdrop pipelines mutex")
            .get(&dev);
        let mut blur = match self.blur.take() {
            Some(blur) if blur.size() == size => blur,
            _ => BlurRenderer::new(&dev, pipelines, size),
        };
        let frame = Frame {
            data,
            size,
            clear,
            output,
            vertices: &self.vertices,
            indices: &self.indices,
            viewport_group: &self.viewport_group,
        };
        let mut draw =
            |pass: &mut wgpu::RenderPass<'_>, index: usize, backdrop: Option<&wgpu::BindGroup>| {
                let command = &data.commands[index];
                match materials.draw(index) {
                    Draw::Skip => return,
                    Draw::Material { pipeline, offset } => {
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(1, &store.textures[&command.texture].bind_group, &[]);
                        materials.bind(pass, *offset, backdrop);
                    }
                    Draw::Plain => {
                        pass.set_pipeline(builtin.for_command(command, data));
                        pass.set_bind_group(1, &store.textures[&command.texture].bind_group, &[]);
                    }
                }
                pass.draw_indexed(command.indices.clone(), 0, 0..1);
            };
        let stats = blur.render_over(&dev, encoder, &frame, &mut draw, compose);
        self.stats.blur_effects += stats.effects;
        self.stats.blur_batches += stats.batches;
        self.stats.blur_batches_reused += stats.reused;
        self.stats.blur_passes += stats.passes + stats.copies;
        self.stats.blur_pixels += stats.pixels;
        self.stats.blur_target_bytes = stats.bytes;
        self.blur = Some(blur);
    }

    /// Release the filter's textures after a stretch of frames without effects.
    pub(super) fn note_no_effects(&mut self) {
        if self.blur.is_some() {
            self.blur_idle_frames += 1;
            if self.blur_idle_frames >= RELEASE_AFTER_FRAMES {
                self.blur = None;
                self.stats.blur_target_bytes = 0;
            }
        }
    }
}
