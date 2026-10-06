//! Frame acquisition, surface recovery, draw submission, and presentation.

use super::{
    materials::Draw,
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
        if let Some(message) = self
            .device_lost
            .lock()
            .expect("device callback mutex")
            .as_ref()
        {
            return Err(RenderError::DeviceLost(message.clone()));
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
        let store = Arc::clone(&self.store);
        let mut store = store.lock().expect("texture store mutex");
        store.clock += 1;
        self.prepare_textures(&mut store, data)?;
        self.prepare_geometry(data, &store)?;
        self.prepare_viewport(data);
        let materials = self.prepare_materials(data)?;
        let encoding = self.diagnostics.start();
        self.diagnostics.render_pending = false;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.attachment_format),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("zaxis frame"),
            });
        if data.commands.iter().any(|c| c.blur.is_some()) {
            self.render_blur(&mut encoder, data, clear, &view, &store, &materials);
        } else {
            let clear = clear.linear();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("zaxis draw pass"),
                timestamp_writes: self.diagnostics.timestamps.as_ref().map(|q| {
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
            if !data.indices.is_empty() {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.viewport_group, &[]);
                pass.set_vertex_buffer(0, self.vertices.slice(..));
                pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
                for (index, command) in data.commands.iter().enumerate() {
                    if command.indices.is_empty() {
                        continue;
                    }
                    let Some([x, y, width, height]) =
                        scissor(command.clip_rect, data.scale_factor, self.physical_size)
                    else {
                        continue;
                    };
                    pass.set_scissor_rect(x, y, width, height);
                    match materials.draw(index) {
                        Draw::Skip => continue,
                        Draw::Material { pipeline, offset } => {
                            pass.set_pipeline(pipeline);
                            materials.bind(&mut pass, *offset, None);
                        }
                        Draw::Plain => pass.set_pipeline(self.pipeline_for(command, data)),
                    }
                    pass.set_bind_group(1, &store.textures[&command.texture].bind_group, &[]);
                    pass.draw_indexed(command.indices.clone(), 0, 0..1);
                }
            }
        }
        if !data.commands.iter().any(|c| c.blur.is_some()) {
            if let Some(q) = &self.diagnostics.timestamps {
                q.encode_readback(&mut encoder);
                self.diagnostics.render_pending = true;
            }
        }
        drop(store);
        let commands = encoder.finish();
        self.diagnostics
            .end(super::diagnostics::RendererStage::EncodingCpu, encoding);
        let submit = self.diagnostics.start();
        self.queue.submit([commands]);
        self.diagnostics
            .end(super::diagnostics::RendererStage::SubmitCpu, submit);
        self.stats.draw_calls += data
            .commands
            .iter()
            .filter(|c| {
                !c.indices.is_empty()
                    && scissor(c.clip_rect, data.scale_factor, self.physical_size).is_some()
            })
            .count() as u64;
        self.window.pre_present_notify();
        self.queue.present(frame);
        self.stats.presented_frames += 1;
        if suboptimal {
            self.resize(self.window.inner_size())?;
        }
        Ok(RenderStatus::Presented)
    }
}
