//! Frame acquisition, surface recovery, draw submission, and presentation.

use super::{
    backdrop::BlurTarget,
    draw::Area,
    viewport::{self, scissor},
    RenderError, RenderStatus, Renderer,
};
use crate::{Color, DrawData};
use std::sync::Arc;

impl Renderer {
    /// Present draw commands. Geometry and atlas pages upload only when their
    /// revisions change. Call on requested redraws, including OS exposure events.
    pub fn render(&mut self, data: &DrawData, clear: Color) -> Result<RenderStatus, RenderError> {
        let clear = self.surface_clear(clear);
        if let Some(message) = self.gpu.lost() {
            return Err(RenderError::DeviceLost(message));
        }
        if self.physical_size.width == 0 || self.physical_size.height == 0 {
            return Ok(RenderStatus::Dormant);
        }
        let (frame, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout => return Ok(RenderStatus::Retry),
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderStatus::Dormant),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.resize(self.window.inner_size())?;
                return Ok(RenderStatus::Retry);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(Arc::clone(&self.window))
                    .map_err(RenderError::CreateSurface)?;
                if !self
                    .surface
                    .get_capabilities(&self.adapter)
                    .formats
                    .contains(&self.config.format)
                {
                    return Err(RenderError::UnsupportedSurface);
                }
                self.resize(self.window.inner_size())?;
                return Ok(RenderStatus::Retry);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderError::Validation(
                    "acquiring the surface texture".to_owned(),
                ))
            }
        };
        viewport::validate(data)?;
        let target = self.pipelines.target;
        let gpu = &mut self.gpu;
        let store = Arc::clone(&gpu.store);
        let mut store = store.lock().expect("texture store mutex");
        store.clock += 1;
        gpu.prepare_textures(&mut store, data)?;
        gpu.prepare_geometry(data, &store)?;
        gpu.prepare_viewport(data, [0, 0]);
        let materials = gpu.prepare_materials(data, &target)?;
        let encoding = gpu.diagnostics.start();
        gpu.diagnostics.render_pending = false;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.attachment_format),
            ..Default::default()
        });
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("zaxis frame"),
            });
        let size = [self.physical_size.width, self.physical_size.height];
        let blur = data.commands.iter().any(|c| c.blur.is_some());
        if blur {
            let target = BlurTarget {
                output: &view,
                format: self.attachment_format,
                size,
                pipelines: &self.pipelines,
                compose: Default::default(),
            };
            gpu.render_blur(&mut encoder, data, clear, target, &store, &materials);
        } else {
            gpu.note_no_effects();
            let clear = clear.linear();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("zaxis draw pass"),
                timestamp_writes: gpu.diagnostics.timestamps.as_ref().map(|q| {
                    wgpu::RenderPassTimestampWrites {
                        query_set: &q.query,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    }
                }),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(clear[0]),
                            g: f64::from(clear[1]),
                            b: f64::from(clear[2]),
                            a: f64::from(clear[3]),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            let area = Area::whole(self.physical_size);
            gpu.record_commands(
                &mut pass,
                data,
                &store,
                &materials,
                &self.pipelines,
                area,
                None,
            );
        }
        if !blur {
            if let Some(q) = &gpu.diagnostics.timestamps {
                q.encode_readback(&mut encoder);
                gpu.diagnostics.render_pending = true;
            }
        }
        drop(store);
        let commands = encoder.finish();
        gpu.diagnostics
            .end(super::diagnostics::RendererStage::EncodingCpu, encoding);
        let submit = gpu.diagnostics.start();
        gpu.queue.submit([commands]);
        gpu.diagnostics
            .end(super::diagnostics::RendererStage::SubmitCpu, submit);
        gpu.stats.draw_calls += data
            .commands
            .iter()
            .filter(|c| {
                !c.indices.is_empty()
                    && scissor(c.clip_rect, data.scale_factor, self.physical_size).is_some()
            })
            .count() as u64;
        self.window.pre_present_notify();
        gpu.queue.present(frame);
        gpu.stats.presented_frames += 1;
        if suboptimal {
            self.resize(self.window.inner_size())?;
        }
        Ok(RenderStatus::Presented)
    }
}
