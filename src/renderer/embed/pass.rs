//! Recording into a render pass the host began.

use super::{lock, target::check_region, EmbedError, EmbedViewport, EmbeddedRenderer};
use crate::{
    renderer::{
        draw::{Area, Degraded},
        materials::{Draw, MaterialFrame},
        pipeline::Kind,
        viewport,
    },
    DrawData, TextureId,
};
use std::{ops::Range, sync::atomic::Ordering, sync::Arc};

/// One command that reaches the pass, resolved at `prepare`.
struct Item {
    /// Index in the draw data, for the material of the command.
    command: usize,
    indices: Range<u32>,
    /// Absolute scissor rectangle in the target, `[x, y, width, height]`.
    scissor: [u32; 4],
    texture: TextureId,
    kind: Kind,
}

/// A frame ready to record: what `record` needs from the draw data once it is gone.
pub(super) struct Prepared {
    viewport: EmbedViewport,
    materials: MaterialFrame,
    items: Vec<Item>,
    degraded: Degraded,
}

/// What `record` drew and what it could not draw as the interface asked.
///
/// A pass cannot read its own pixels, so effects that need what is behind them degrade:
/// a command that blurs the backdrop is not drawn at all (its mesh is only the shape to
/// composite into), and a material that reads the backdrop is drawn against a transparent
/// one. Both are listed by command index in the draw data so the host can show a
/// translucent fill of its own, or switch to [`render_to`](EmbeddedRenderer::render_to).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordReport {
    /// Draw calls recorded.
    pub draw_calls: u32,
    /// Backdrop-blur commands that were not drawn.
    pub skipped_backdrop: Vec<usize>,
    /// Backdrop-reading materials drawn without a backdrop.
    pub degraded_materials: Vec<usize>,
}

impl EmbeddedRenderer {
    /// Upload what `data` changed (geometry and atlas pages only when their revision is
    /// new, textures when their payload is) and resolve the frame for [`record`](Self::record).
    /// Call it outside the pass, at any point before the host submits the frame; it does
    /// not wait for the GPU.
    ///
    /// `data` must have been built for the region: its logical size times its scale factor is
    /// the region in physical pixels (within a pixel). The scale factor is the DPI, kept
    /// apart from the size, so a fractional scale only changes how clips round.
    pub fn prepare(&mut self, data: &DrawData, viewport: EmbedViewport) -> Result<(), EmbedError> {
        let reuse = self.prepared.take();
        self.check_alive()?;
        check_region(
            viewport.region,
            viewport.target,
            data,
            self.gpu.device.limits(),
        )?;
        viewport::validate(data)?;
        let target = self.pass_pipelines.target;
        let region = viewport.region;
        let gpu = &mut self.gpu;
        let store = Arc::clone(&gpu.store);
        let mut store = lock(&store);
        store.clock += 1;
        gpu.prepare_textures(&mut store, data)?;
        gpu.prepare_geometry(data, &store)?;
        gpu.prepare_viewport(data, [region.x, region.y]);
        let materials = gpu.prepare_materials(data, &target)?;
        gpu.note_no_effects();
        let area = Area {
            origin: [region.x, region.y],
            size: region.size(),
        };
        let mut items = reuse.map(|p| p.items).unwrap_or_default();
        items.clear();
        let mut degraded = Degraded::default();
        for (index, command) in data.commands.iter().enumerate() {
            if command.indices.is_empty() {
                continue;
            }
            let Some(scissor) = area.scissor(command.clip_rect, data.scale_factor) else {
                continue;
            };
            if command.blur.is_some() {
                if command.material.is_none() {
                    degraded.backdrop.push(index);
                    continue;
                }
                degraded.materials.push(index);
            }
            items.push(Item {
                command: index,
                indices: command.indices.clone(),
                scissor,
                texture: command.texture,
                kind: Kind::of(command, data),
            });
        }
        drop(store);
        self.prepared = Some(Prepared {
            viewport,
            materials,
            items,
            degraded,
        });
        Ok(())
    }

    /// Record the prepared frame into `pass`, which the host began with the color format,
    /// sample count and depth/stencil format of this renderer's [`EmbedOptions`].
    ///
    /// Draws only inside the region, leaves the attachments' contents outside it alone, and
    /// neither tests nor writes depth or stencil. Sets the viewport and scissor rectangle to
    /// the region and, when done, back to the whole target; a host that had set its own must
    /// set them again. It binds vertex and index buffers, pipelines and bind groups 0 to 3;
    /// the host re-binds what it needs afterwards. Never blocks, polls or submits.
    ///
    /// The pass cannot be inspected, so a pass whose attachments differ from the options is
    /// reported by wgpu's validation, not here. Backdrop effects cannot be drawn into a pass;
    /// see [`RecordReport`].
    pub fn record(&self, pass: &mut wgpu::RenderPass<'_>) -> Result<RecordReport, EmbedError> {
        self.check_alive()?;
        let Some(frame) = &self.prepared else {
            return Err(EmbedError::NotPrepared);
        };
        let mut report = RecordReport {
            draw_calls: 0,
            skipped_backdrop: frame.degraded.backdrop.clone(),
            degraded_materials: frame.degraded.materials.clone(),
        };
        if frame.items.is_empty() {
            return Ok(report);
        }
        let region = frame.viewport.region;
        let [width, height] = frame.viewport.target;
        let store = lock(&self.gpu.store);
        pass.set_viewport(
            region.x as f32,
            region.y as f32,
            region.width as f32,
            region.height as f32,
            0.0,
            1.0,
        );
        pass.set_pipeline(&self.pass_pipelines.plain);
        pass.set_bind_group(0, &self.gpu.viewport_group, &[]);
        pass.set_vertex_buffer(0, self.gpu.vertices.slice(..));
        pass.set_index_buffer(self.gpu.indices.slice(..), wgpu::IndexFormat::Uint32);
        for item in &frame.items {
            let Some(texture) = store.textures.get(&item.texture) else {
                continue;
            };
            let [x, y, w, h] = item.scissor;
            pass.set_scissor_rect(x, y, w, h);
            match frame.materials.draw(item.command) {
                Draw::Skip => continue,
                Draw::Material { pipeline, offset } => {
                    pass.set_pipeline(pipeline);
                    frame.materials.bind(pass, *offset, None);
                }
                Draw::Plain => pass.set_pipeline(self.pass_pipelines.get(item.kind)),
            }
            pass.set_bind_group(1, &texture.bind_group, &[]);
            pass.draw_indexed(item.indices.clone(), 0, 0..1);
            report.draw_calls += 1;
        }
        pass.set_viewport(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
        pass.set_scissor_rect(0, 0, width, height);
        self.recorded.frames.fetch_add(1, Ordering::Relaxed);
        self.recorded
            .draws
            .fetch_add(u64::from(report.draw_calls), Ordering::Relaxed);
        Ok(report)
    }
}
