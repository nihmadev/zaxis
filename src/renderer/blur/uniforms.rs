//! Per-draw parameter blocks of one frame, packed into two buffers addressed by dynamic
//! offsets: the filter blocks of the passes and the effect blocks of the composites.

use super::{
    pipelines::{BlurPipelines, EFFECT_BYTES, FILTER_BYTES},
    BlurDevice,
};
use bytemuck::{Pod, Zeroable};

/// The block of `fs_gaussian` and `fs_resolve`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FilterBlock {
    pub step: [f32; 2],
    pub sigma: f32,
    pub radius: f32,
    pub inverse_scale: f32,
    pub pad: [f32; 3],
}

/// The block of `fs_composite`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct EffectBlock {
    /// Level texels per window pixel, and whether the level is reduced (the spline reads it).
    pub level: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<EffectBlock>() as u64 == EFFECT_BYTES);
const _: () = assert!(std::mem::size_of::<FilterBlock>() as u64 == FILTER_BYTES);

impl EffectBlock {
    /// The block of a composite over a backdrop reduced by `level` pyramid levels.
    pub fn new(level: u32) -> Self {
        Self {
            level: [1.0 / (1u32 << level) as f32, f32::from(level > 0), 0.0, 0.0],
        }
    }
}

/// Dynamic-offset buffer of fixed-size blocks, rebuilt every frame.
struct Ring {
    buffer: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    capacity: u64,
    block: u64,
    stride: u64,
    staged: Vec<u8>,
}

impl Ring {
    fn new(block: u64, align: u64) -> Self {
        Self {
            buffer: None,
            capacity: 0,
            block,
            stride: block.next_multiple_of(align),
            staged: Vec::new(),
        }
    }

    fn push(&mut self, bytes: &[u8]) -> u32 {
        let offset = self.staged.len() as u32;
        self.staged.extend_from_slice(bytes);
        self.staged
            .resize(offset as usize + self.stride as usize, 0);
        offset
    }

    fn upload(&mut self, dev: &BlurDevice, layout: &wgpu::BindGroupLayout, label: &str) {
        let needed = (self.staged.len() as u64).max(self.stride);
        if self.buffer.is_none() || needed > self.capacity {
            self.capacity = needed.next_power_of_two();
            let buffer = dev.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: self.capacity,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = dev.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(self.block),
                    }),
                }],
            });
            self.buffer = Some((buffer, group));
        }
        if let Some((buffer, _)) = &self.buffer {
            if !self.staged.is_empty() {
                dev.queue.write_buffer(buffer, 0, &self.staged);
            }
        }
        self.staged.clear();
    }
}

/// The blocks of one frame.
pub struct Uniforms {
    filter: Ring,
    effect: Ring,
}

impl Uniforms {
    pub fn new(dev: &BlurDevice) -> Self {
        let align = u64::from(dev.device.limits().min_uniform_buffer_offset_alignment);
        Self {
            filter: Ring::new(FILTER_BYTES, align),
            effect: Ring::new(EFFECT_BYTES, align),
        }
    }

    pub fn filter(&mut self, block: &FilterBlock) -> u32 {
        self.filter.push(bytemuck::bytes_of(block))
    }

    pub fn effect(&mut self, block: &EffectBlock) -> u32 {
        self.effect.push(bytemuck::bytes_of(block))
    }

    pub fn upload(&mut self, dev: &BlurDevice, pipelines: &BlurPipelines) {
        self.filter.upload(
            dev,
            &pipelines.filter_layout,
            "zaxis backdrop filter blocks",
        );
        self.effect.upload(
            dev,
            &pipelines.effect_layout,
            "zaxis backdrop effect blocks",
        );
    }

    pub fn filter_group(&self) -> &wgpu::BindGroup {
        &self
            .filter
            .buffer
            .as_ref()
            .expect("uploaded filter blocks")
            .1
    }

    pub fn effect_group(&self) -> &wgpu::BindGroup {
        &self
            .effect
            .buffer
            .as_ref()
            .expect("uploaded effect blocks")
            .1
    }

    pub fn bytes(&self) -> u64 {
        self.filter.capacity + self.effect.capacity
    }
}
