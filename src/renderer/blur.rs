//! Ordered backdrop effects. The window is drawn into an offscreen canvas in segments; at
//! each effect the area behind it is filtered (pyramid + Gaussian, see `params`) and the
//! effect's mesh is composited into the canvas (the blurred level replaces what is behind the shape). Independent effects
//! share one set of filter passes (`plan`), unchanged backdrops are not filtered again
//! (`cache`), and every texture exists only while frames have effects.
//!
//! The engine needs a device and a few bind group layouts, not a window, so tests drive it
//! headless (`BlurDevice`).

pub mod area;
mod cache;
pub mod params;
mod passes;
mod pipelines;
pub mod plan;
mod targets;
mod uniforms;

use crate::{Color, DrawData};
use passes::{Group, Offsets};
pub use pipelines::BlurPipelines;
use plan::Plan;
use std::sync::Arc;
use targets::{Set, Tex};
use uniforms::Uniforms;

/// Independent sets of filter textures a window keeps. Groups beyond this share the last
/// one and are recomputed every frame.
pub const MAX_SETS: usize = 3;

/// What the engine needs from a device.
#[derive(Clone, Copy)]
pub struct BlurDevice<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    /// Format of the canvas (the surface's attachment format).
    pub format: wgpu::TextureFormat,
    /// Format of the pyramid; see [`intermediate_format`].
    pub intermediate: wgpu::TextureFormat,
    pub texture_layout: &'a wgpu::BindGroupLayout,
    pub sampler: &'a wgpu::Sampler,
    pub viewport_layout: &'a wgpu::BindGroupLayout,
    /// Layout of the material backdrop group (sharp, blurred, sampler), if materials exist.
    pub backdrop_layout: Option<&'a wgpu::BindGroupLayout>,
}

/// Half-float where the adapter renders to and filters it, else the canvas format: pyramid
/// levels keep their precision, so large blurs of smooth gradients do not band.
pub fn intermediate_format(
    adapter: &wgpu::Adapter,
    canvas: wgpu::TextureFormat,
) -> wgpu::TextureFormat {
    let format = wgpu::TextureFormat::Rgba16Float;
    let features = adapter.get_texture_format_features(format);
    let usages = wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_SRC
        | wgpu::TextureUsages::COPY_DST;
    if features.allowed_usages.contains(usages)
        && features
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE)
    {
        format
    } else {
        canvas
    }
}

/// Work done by the filter in one frame (`bytes` is the standing size of its textures).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    pub effects: u64,
    pub batches: u64,
    pub reused: u64,
    pub passes: u64,
    pub copies: u64,
    pub pixels: u64,
    pub bytes: u64,
}

/// One frame to draw.
pub struct Frame<'a> {
    pub data: &'a DrawData,
    /// Physical size of the canvas and the output.
    pub size: [u32; 2],
    pub clear: Color,
    pub output: &'a wgpu::TextureView,
    pub vertices: &'a wgpu::Buffer,
    pub indices: &'a wgpu::Buffer,
    pub viewport_group: &'a wgpu::BindGroup,
}

/// Where a frame goes and what it starts from, when the output is not a window of its own.
#[derive(Clone, Copy, Default)]
pub struct Compose<'a> {
    /// Content to draw over, copied into the canvas from `base_origin` instead of clearing it.
    /// Effects then see it as their backdrop, so nothing is reused between frames.
    pub base: Option<&'a wgpu::Texture>,
    /// Texel of `base` the canvas starts at.
    pub base_origin: [u32; 2],
    /// Part of the output the canvas is drawn into, `[x, y, width, height]` in output pixels;
    /// `None` is the whole output, cleared first. With a region the rest is kept unless
    /// `clear_output` is set.
    pub region: Option<[u32; 4]>,
    /// Clear the whole output to this color before the canvas goes into the region.
    pub clear_output: Option<wgpu::Color>,
}

/// Draws command `index` that is not an effect composite, with the pass's scissor already
/// set. The group is the backdrop of a material that reads it.
pub type DrawPlain<'f> = dyn FnMut(&mut wgpu::RenderPass<'_>, usize, Option<&wgpu::BindGroup>) + 'f;

/// Textures and parameters of one window; freed on resize, on idle and when it closes.
pub struct BlurRenderer {
    pipelines: Arc<BlurPipelines>,
    canvas: Tex,
    size: [u32; 2],
    sets: Vec<Set>,
    uniforms: Uniforms,
}

impl BlurRenderer {
    pub fn new(dev: &BlurDevice, pipelines: Arc<BlurPipelines>, size: [u32; 2]) -> Self {
        Self {
            canvas: Tex::new(dev, size, dev.format),
            uniforms: Uniforms::new(dev),
            pipelines,
            size,
            sets: Vec::new(),
        }
    }

    /// Physical size of the canvas this renderer was made for.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Estimated bytes of the textures and parameter buffers held.
    pub fn bytes(&self) -> u64 {
        self.canvas.bytes + self.sets.iter().map(Set::bytes).sum::<u64>() + self.uniforms.bytes()
    }

    /// Draw `frame`: its commands in order into the canvas, effects filtered and composited,
    /// then the canvas into the output.
    pub fn render(
        &mut self,
        dev: &BlurDevice,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        draw: &mut DrawPlain,
    ) -> FrameStats {
        self.render_over(dev, encoder, frame, draw, Compose::default())
    }

    /// Like [`render`](Self::render), starting from the content of `compose` and writing
    /// only its region of the output.
    pub fn render_over(
        &mut self,
        dev: &BlurDevice,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        draw: &mut DrawPlain,
        compose: Compose,
    ) -> FrameStats {
        let data = frame.data;
        let plan = plan::plan(data, self.size);
        let signatures = cache::signatures(data, &plan, self.size, frame.clear);
        let mut stats = FrameStats {
            effects: plan.jobs.len() as u64,
            batches: plan.batches.len() as u64,
            ..Default::default()
        };
        let materials: Vec<bool> = plan
            .jobs
            .iter()
            .map(|job| data.commands[job.command].material.is_some())
            .collect();
        let mut set_of = vec![0; plan.jobs.len()];
        for (batch, range) in plan.batches.iter().enumerate() {
            set_of[range.clone()].fill(batch.min(MAX_SETS - 1));
        }
        self.prepare_sets(dev, &plan, &set_of, &materials);
        let blocks = self.write_blocks(dev, &plan);
        let first = plan.jobs.first().map_or(data.commands.len(), |j| j.command);
        let mut segment = Segment {
            frame,
            plan: &plan,
            blocks: &blocks,
            set_of: &set_of,
        };
        let clear = match compose.base {
            Some(base) => {
                self.copy_base(encoder, base, compose.base_origin);
                None
            }
            None => Some(frame.clear),
        };
        self.segment(&mut segment, encoder, draw, 0..first, clear);
        for (batch, range) in plan.batches.iter().enumerate() {
            let set = batch.min(MAX_SETS - 1);
            let reusable =
                compose.base.is_none() && (batch < MAX_SETS - 1 || plan.batches.len() <= MAX_SETS);
            let signature = cache::combine(&signatures[range.clone()]);
            if reusable && self.sets[set].signature == Some(signature) {
                stats.reused += 1;
            } else {
                let group = Group {
                    jobs: &plan.jobs[range.clone()],
                    offsets: &blocks.filters[range.clone()],
                    unused: blocks.unused,
                    materials: &materials[range.clone()],
                };
                passes::filter_group(
                    encoder,
                    &self.pipelines,
                    &self.uniforms,
                    &self.canvas,
                    &self.sets[set],
                    &group,
                    &mut stats,
                );
                self.sets[set].signature = reusable.then_some(signature);
            }
            for job in range.clone() {
                let from = plan.jobs[job].command;
                let to = plan
                    .jobs
                    .get(job + 1)
                    .map_or(data.commands.len(), |j| j.command);
                self.segment(&mut segment, encoder, draw, from..to, None);
            }
        }
        self.finish(
            encoder,
            frame,
            blocks.unused,
            compose.region,
            compose.clear_output,
        );
        stats.bytes = self.bytes();
        stats
    }
}

/// Everything a segment of commands needs to know about the frame.
pub(crate) struct Segment<'a> {
    pub frame: &'a Frame<'a>,
    pub plan: &'a Plan,
    pub blocks: &'a Blocks,
    pub set_of: &'a [usize],
}

/// The parameter blocks of a frame, as dynamic offsets.
pub(crate) struct Blocks {
    pub filters: Vec<Offsets>,
    pub effects: Vec<u32>,
    pub unused: u32,
}

mod draw;
