//! User materials in the renderer: which pipeline and which uniform slot each command draws
//! with. Everything a material needs arrives in the draw data, so a renderer created after a
//! device loss rebuilds its pipelines from the next frame alone.

mod pipelines;
mod uniforms;

pub use pipelines::{MaterialPipelines, MAX_PIPELINES};
pub use uniforms::MaterialUniforms;

use super::{gpu::Gpu, pipeline::Target, RenderError, Renderer};
use crate::{DrawData, MAX_UNIFORM_BYTES};

/// How one command of a frame is drawn.
pub(super) enum Draw {
    /// With the built-in pipelines.
    Plain,
    /// A material whose pipeline failed to build: nothing is drawn.
    Skip,
    Material {
        pipeline: wgpu::RenderPipeline,
        offset: u32,
    },
}

/// The materials of one frame, by command index. Empty when no command has one.
pub(super) struct MaterialFrame {
    draws: Vec<Draw>,
    no_backdrop: Option<wgpu::BindGroup>,
    uniforms: Option<wgpu::BindGroup>,
}

impl MaterialFrame {
    pub fn none() -> Self {
        Self {
            draws: Vec::new(),
            no_backdrop: None,
            uniforms: None,
        }
    }

    /// Bind groups 2 and 3 of a material draw after its pipeline is set. `backdrop` is the
    /// group with the sharp and blurred backdrop of a draw that reads it.
    pub fn bind(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        offset: u32,
        backdrop: Option<&wgpu::BindGroup>,
    ) {
        pass.set_bind_group(2, backdrop.or(self.no_backdrop.as_ref()), &[]);
        pass.set_bind_group(3, self.uniforms.as_ref(), &[offset]);
    }

    pub fn draw(&self, command: usize) -> &Draw {
        self.draws.get(command).unwrap_or(&Draw::Plain)
    }
}

impl Gpu {
    /// Check the materials of `data`, upload their uniform blocks and find the pipelines for
    /// passes into `target`.
    pub(super) fn prepare_materials(
        &mut self,
        data: &DrawData,
        target: &Target,
    ) -> Result<MaterialFrame, RenderError> {
        if data.commands.iter().all(|c| c.material.is_none()) {
            return Ok(MaterialFrame::none());
        }
        validate(data)?;
        let mut pipelines = self.materials.lock().expect("material pipelines mutex");
        pipelines.begin_frame();
        self.material_uniforms
            .upload(&self.device, &self.queue, &pipelines.params_layout, data);
        let mut draws = Vec::with_capacity(data.commands.len());
        for command in &data.commands {
            let Some(draw) = &command.material else {
                draws.push(Draw::Plain);
                continue;
            };
            let source = data
                .materials
                .iter()
                .find(|source| source.id == draw.id)
                .expect("validated material source");
            let offset = self.material_uniforms.offset(draw.uniforms.start);
            let pipeline = pipelines.pipeline_in(source, target);
            draws.push(match (pipeline, offset) {
                (Some(pipeline), Some(offset)) => Draw::Material { pipeline, offset },
                _ => Draw::Skip,
            });
            self.stats.material_draws += 1;
        }
        pipelines.trim();
        let (builds, failures) = pipelines.counters();
        self.stats.material_pipeline_builds = builds;
        self.stats.material_pipeline_failures = failures;
        self.stats.material_uniform_bytes = self.material_uniforms.bytes_written;
        self.material_errors.extend(pipelines.take_errors());
        Ok(MaterialFrame {
            draws,
            no_backdrop: Some(pipelines.no_backdrop.clone()),
            uniforms: Some(self.material_uniforms.group().clone()),
        })
    }

    /// Pipelines of materials held now, failed builds included.
    pub(super) fn material_pipeline_count(&self) -> usize {
        self.materials
            .lock()
            .expect("material pipelines mutex")
            .len()
    }
}

impl Renderer {
    /// Pipelines of materials held now, failed builds included.
    pub fn material_pipeline_count(&self) -> usize {
        self.gpu.material_pipeline_count()
    }

    /// Why material pipelines failed to build since the last call. Empty on a healthy
    /// device: sources are validated before they reach the renderer.
    pub fn take_material_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.gpu.material_errors)
    }
}

/// Reject draw data whose materials cannot be drawn safely.
fn validate(data: &DrawData) -> Result<(), RenderError> {
    for command in &data.commands {
        let Some(draw) = &command.material else {
            continue;
        };
        if command.scroll_hint {
            return Err(RenderError::InvalidDrawData(
                "scroll hint and material cannot share a command",
            ));
        }
        if !data.materials.iter().any(|source| source.id == draw.id) {
            return Err(RenderError::InvalidDrawData(
                "a command uses a material that has no source in the draw data",
            ));
        }
        let range = draw.uniforms.start as usize..draw.uniforms.end as usize;
        let len = range.end.saturating_sub(range.start);
        if range.start > range.end
            || range.end > data.material_uniforms.len()
            || range.start % 16 != 0
            || len % 16 != 0
            || !(crate::FRAME_BYTES..=MAX_UNIFORM_BYTES).contains(&len)
        {
            return Err(RenderError::InvalidDrawData(
                "material uniforms are outside the buffer, misaligned or of an unsupported size",
            ));
        }
    }
    Ok(())
}
