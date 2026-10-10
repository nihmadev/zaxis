//! Recording the commands of a frame into a render pass, for every kind of renderer.

use super::{
    gpu::Gpu,
    materials::{Draw, MaterialFrame},
    pipeline::PipelineSet,
    textures::TextureStore,
    viewport::scissor,
};
use crate::DrawData;
use winit::dpi::PhysicalSize;

/// The part of the render target the interface is drawn into, in physical pixels. The
/// scissor of a command is cut from the logical clip inside it and moved to `origin`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Area {
    pub origin: [u32; 2],
    pub size: [u32; 2],
}

impl Area {
    /// The whole target of `size`.
    pub fn whole(size: PhysicalSize<u32>) -> Self {
        Self {
            origin: [0, 0],
            size: [size.width, size.height],
        }
    }

    /// Scissor rectangle `[x, y, width, height]` of a clip, in target pixels.
    pub fn scissor(&self, clip: crate::Rect, scale: f32) -> Option<[u32; 4]> {
        let local = PhysicalSize::new(self.size[0], self.size[1]);
        scissor(clip, scale, local)
            .map(|[x, y, w, h]| [self.origin[0] + x, self.origin[1] + y, w, h])
    }
}

/// What a pass without access to its own pixels could not draw as asked.
#[derive(Debug, Default)]
pub(super) struct Degraded {
    /// Commands that blur the backdrop: not drawn.
    pub backdrop: Vec<usize>,
    /// Materials that read the backdrop: drawn with a transparent one.
    pub materials: Vec<usize>,
}

impl Gpu {
    /// Record the commands of `data` into `pass` with the built-in pipelines of `set` and
    /// the materials of `materials`, inside `area`. Returns the number of commands that
    /// reach it. With `degraded`, commands that need the pixels behind them are noted in it
    /// instead of drawn as backdrop effects.
    pub(super) fn record_commands(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        data: &DrawData,
        store: &TextureStore,
        materials: &MaterialFrame,
        set: &PipelineSet,
        area: Area,
        mut degraded: Option<&mut Degraded>,
    ) -> u64 {
        if data.indices.is_empty() {
            return 0;
        }
        pass.set_pipeline(&set.plain);
        pass.set_bind_group(0, &self.viewport_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        let mut draws = 0;
        for (index, command) in data.commands.iter().enumerate() {
            if command.indices.is_empty() {
                continue;
            }
            let Some([x, y, width, height]) = area.scissor(command.clip_rect, data.scale_factor)
            else {
                continue;
            };
            draws += 1;
            if let Some(degraded) = degraded.as_deref_mut() {
                if command.blur.is_some() {
                    if command.material.is_none() {
                        degraded.backdrop.push(index);
                        continue;
                    }
                    degraded.materials.push(index);
                }
            }
            pass.set_scissor_rect(x, y, width, height);
            match materials.draw(index) {
                Draw::Skip => continue,
                Draw::Material { pipeline, offset } => {
                    pass.set_pipeline(pipeline);
                    materials.bind(pass, *offset, None);
                }
                Draw::Plain => pass.set_pipeline(set.for_command(command, data)),
            }
            pass.set_bind_group(1, &store.textures[&command.texture].bind_group, &[]);
            pass.draw_indexed(command.indices.clone(), 0, 0..1);
        }
        draws
    }
}
