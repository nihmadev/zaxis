//! Opt-in serialized diagnostics. CPU API durations are never GPU allocation times.
use super::{RenderError, Renderer};
use crate::time::Instant;
use crate::TextureImage;
use std::{collections::VecDeque, sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RendererStage {
    TextureLookup,
    CreateTextureCpu,
    CreateViewCpu,
    CreateBindGroupCpu,
    WriteTextureCpu,
    StagingCpu,
    EncodingCpu,
    SubmitCpu,
}
#[derive(Clone, Copy, Debug)]
pub struct RendererTiming {
    pub stage: RendererStage,
    pub duration: Duration,
}
/// Diagnostic explicit buffer-to-texture copy, separate from production write_texture.
#[derive(Clone, Debug)]
pub struct ImageUploadMeasurement {
    pub cpu_stages: Vec<RendererTiming>,
    pub gpu_transfer: Option<Duration>,
    /// Exact hardware allocation duration is unavailable in wgpu.
    pub gpu_allocation: Option<Duration>,
    /// Wall time: creation, staging, encode/submit, wait and timestamp readback.
    pub completion_inclusive: Duration,
    pub upload_bytes: u64,
}
#[derive(Default)]
pub(super) struct Diagnostics {
    enabled: bool,
    timings: VecDeque<RendererTiming>,
    pub(super) timestamps: Option<Timestamps>,
    pub(super) render_pending: bool,
}
impl Diagnostics {
    pub fn start(&self) -> Option<Instant> {
        self.enabled.then(Instant::now)
    }
    pub fn end(&mut self, stage: RendererStage, start: Option<Instant>) {
        if let Some(start) = start {
            if self.timings.len() == 1024 {
                self.timings.pop_front();
            }
            self.timings.push_back(RendererTiming {
                stage,
                duration: start.elapsed(),
            });
        }
    }
}
pub(super) struct Timestamps {
    pub query: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
}
impl Timestamps {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            query: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("zaxis diagnostic timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 256,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
        }
    }
    pub fn encode_readback(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.resolve_query_set(&self.query, 0..2, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, 16);
    }
    fn read(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Duration, RenderError> {
        let (tx, rx) = mpsc::channel();
        self.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .map_err(|e| RenderError::Validation(e.to_string()))?;
        rx.recv_timeout(Duration::from_secs(10))
            .map_err(|e| RenderError::Validation(e.to_string()))?
            .map_err(|e| RenderError::Validation(e.to_string()))?;
        let data = self
            .readback
            .slice(..)
            .get_mapped_range()
            .map_err(|e| RenderError::Validation(e.to_string()))?;
        let start = u64::from_ne_bytes(data[..8].try_into().unwrap());
        let end = u64::from_ne_bytes(data[8..16].try_into().unwrap());
        let seconds = end.wrapping_sub(start) as f64 * queue.get_timestamp_period() as f64 / 1e9;
        drop(data);
        self.readback.unmap();
        Ok(Duration::from_secs_f64(seconds))
    }
}
impl Renderer {
    /// Enables CPU stage instrumentation and supported render-pass timestamp queries.
    /// Consume read_render_gpu_time after each present before submitting another frame.
    pub fn set_image_diagnostics(&mut self, enabled: bool) {
        self.gpu.diagnostics.enabled = enabled;
        self.gpu.diagnostics.timestamps = (enabled
            && self
                .gpu
                .device
                .features()
                .contains(wgpu::Features::TIMESTAMP_QUERY))
        .then(|| Timestamps::new(&self.gpu.device));
        self.gpu.diagnostics.render_pending = false;
    }
    pub fn take_renderer_timings(&mut self) -> Vec<RendererTiming> {
        self.gpu.diagnostics.timings.drain(..).collect()
    }
    pub fn timestamp_queries_supported(&self) -> bool {
        self.gpu
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
    }
    /// Actual GPU render pass duration, excluding uploads, surface acquisition and
    /// presentation. None when unsupported/disabled or for the multi-pass blur path.
    /// Includes a completion/readback wait; diagnostic serialized use only.
    pub fn read_render_gpu_time(&mut self) -> Result<Option<Duration>, RenderError> {
        if !self.gpu.diagnostics.render_pending {
            return Ok(None);
        }
        self.gpu.diagnostics.render_pending = false;
        self.gpu
            .diagnostics
            .timestamps
            .as_ref()
            .map(|q| q.read(&self.gpu.device, &self.gpu.queue))
            .transpose()
    }
    pub fn diagnostic_image_upload(
        &self,
        image: &TextureImage,
    ) -> Result<ImageUploadMeasurement, RenderError> {
        if image.size.contains(&0)
            || image
                .size
                .iter()
                .any(|s| *s > self.max_texture_dimension_2d())
            || image.pixels.len() as u64 != u64::from(image.size[0]) * u64::from(image.size[1]) * 4
            || image.pixels.len() > 256 << 20
        {
            return Err(RenderError::Texture(
                "unsupported diagnostic dimensions/byte count".into(),
            ));
        }
        self.wait_idle(Duration::from_secs(10))?;
        let total = Instant::now();
        let mut profile = Diagnostics {
            enabled: true,
            ..Default::default()
        };
        let start = profile.start();
        let extent = wgpu::Extent3d {
            width: image.size[0],
            height: image.size[1],
            depth_or_array_layers: 1,
        };
        let texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("diagnostic explicit-copy texture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        profile.end(RendererStage::CreateTextureCpu, start);
        let _binding = super::textures::binding(
            &self.gpu.device,
            &self.gpu.texture_layout,
            &self.gpu.sampler,
            &texture,
            &mut profile,
        );
        let start = profile.start();
        let row = (image.size[0] * 4).div_ceil(256) * 256;
        let mut padded = vec![0; row as usize * image.size[1] as usize];
        for (dst, src) in padded
            .chunks_exact_mut(row as usize)
            .zip(image.pixels.chunks_exact(image.size[0] as usize * 4))
        {
            dst[..src.len()].copy_from_slice(src);
        }
        let staging = self
            .gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("diagnostic staging"),
                contents: &padded,
                usage: wgpu::BufferUsages::COPY_SRC,
            });
        profile.end(RendererStage::StagingCpu, start);
        let timestamp = self
            .gpu
            .device
            .features()
            .contains(
                wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
            )
            .then(|| Timestamps::new(&self.gpu.device));
        let start = profile.start();
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        if let Some(q) = &timestamp {
            encoder.write_timestamp(&q.query, 0);
        }
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(image.size[1]),
                },
            },
            texture.as_image_copy(),
            extent,
        );
        if let Some(q) = &timestamp {
            encoder.write_timestamp(&q.query, 1);
            q.encode_readback(&mut encoder);
        }
        let command = encoder.finish();
        profile.end(RendererStage::EncodingCpu, start);
        let start = profile.start();
        self.gpu.queue.submit([command]);
        profile.end(RendererStage::SubmitCpu, start);
        self.wait_idle(Duration::from_secs(10))?;
        let gpu_transfer = timestamp
            .map(|q| q.read(&self.gpu.device, &self.gpu.queue))
            .transpose()?;
        Ok(ImageUploadMeasurement {
            cpu_stages: profile.timings.into_iter().collect(),
            gpu_transfer,
            gpu_allocation: None,
            completion_inclusive: total.elapsed(),
            upload_bytes: image.pixels.len() as u64,
        })
    }
}
