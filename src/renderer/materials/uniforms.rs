//! The frame's uniform blocks of material draws, in one buffer bound with dynamic offsets.

use crate::{DrawData, MAX_UNIFORM_BYTES};
use std::collections::HashMap;

/// A buffer of aligned slots and the bind group over it, per window.
pub struct MaterialUniforms {
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
    capacity: u64,
    alignment: u64,
    /// Aligned slot of each distinct block of the uploaded draw data, by its start there.
    slots: HashMap<u32, u32>,
    uploaded: Option<(u64, u64)>,
    pub(in crate::renderer) bytes_written: u64,
}

impl MaterialUniforms {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let capacity = 4 * MAX_UNIFORM_BYTES as u64;
        let (buffer, group) = create(device, layout, capacity);
        Self {
            buffer,
            group,
            capacity,
            alignment: u64::from(device.limits().min_uniform_buffer_offset_alignment).max(16),
            slots: HashMap::new(),
            uploaded: None,
            bytes_written: 0,
        }
    }

    /// Bind group of group 3: set it with the dynamic offset of [`offset`](Self::offset).
    pub fn group(&self) -> &wgpu::BindGroup {
        &self.group
    }

    /// The dynamic offset of a command's block, after [`upload`](Self::upload).
    pub fn offset(&self, start: u32) -> Option<u32> {
        self.slots.get(&start).copied()
    }

    /// Copy the blocks of `data` into aligned slots when its revision is new. Each distinct
    /// block is written once however many commands bind it.
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        data: &DrawData,
    ) {
        let revision = (data.source, data.revision);
        if self.uploaded == Some(revision) {
            return;
        }
        self.slots.clear();
        let mut staging: Vec<u8> = Vec::new();
        for draw in data.commands.iter().filter_map(|c| c.material.as_ref()) {
            let range = draw.uniforms.start as usize..draw.uniforms.end as usize;
            if self.slots.contains_key(&draw.uniforms.start) {
                continue;
            }
            let offset = (staging.len() as u64).next_multiple_of(self.alignment);
            staging.resize(offset as usize, 0);
            staging.extend_from_slice(&data.material_uniforms[range]);
            self.slots.insert(draw.uniforms.start, offset as u32);
        }
        // A binding is MAX_UNIFORM_BYTES wide wherever its offset: leave room after the last.
        let needed = staging.len() as u64 + MAX_UNIFORM_BYTES as u64;
        if needed > self.capacity {
            self.capacity = needed.next_power_of_two();
            (self.buffer, self.group) = create(device, layout, self.capacity);
        }
        if !staging.is_empty() {
            queue.write_buffer(&self.buffer, 0, &staging);
            self.bytes_written += staging.len() as u64;
        }
        self.uploaded = Some(revision);
    }

    /// Forget what was uploaded, for a renderer whose buffer contents were lost.
    pub fn invalidate(&mut self) {
        self.uploaded = None;
    }
}

fn create(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    capacity: u64,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zaxis material uniforms"),
        size: capacity,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zaxis material uniforms"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: wgpu::BufferSize::new(MAX_UNIFORM_BYTES as u64),
            }),
        }],
    });
    (buffer, group)
}
