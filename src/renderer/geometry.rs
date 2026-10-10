//! Mesh validation, persistent buffers, and revision caching.

use super::{gpu::Gpu, RenderError};
use crate::{DrawData, Vertex};
use std::ops::Range;

impl Gpu {
    pub(super) fn prepare_geometry(
        &mut self,
        data: &DrawData,
        store: &super::textures::TextureStore,
    ) -> Result<(), RenderError> {
        let revision = (data.source, data.revision);
        if self.uploaded != Some(revision) {
            let update = data.geometry_update.as_ref().filter(|update| {
                update.to_revision == data.revision
                    && self.uploaded == Some((data.source, update.from_revision))
                    && data.vertices.len() >= self.uploaded_sizes[0]
                    && data.indices.len() >= self.uploaded_sizes[1]
            });
            if let Some(update) = update {
                if !valid_ranges(&update.vertices, data.vertices.len())
                    || !valid_ranges(&update.indices, data.indices.len())
                {
                    return Err(RenderError::InvalidDrawData(
                        "dirty geometry range is outside the buffer",
                    ));
                }
            }
            let vertex_ranges = update.map(|u| u.vertices.as_slice());
            let index_ranges = update.map(|u| u.indices.as_slice());
            if data.vertices.len() > u32::MAX as usize
                || data.indices.len() > u32::MAX as usize
                || !valid_indices(data, index_ranges)
            {
                return Err(RenderError::InvalidDrawData(
                    "mesh indices are outside the vertex buffer",
                ));
            }
            if !valid_vertices(data, vertex_ranges) {
                return Err(RenderError::InvalidDrawData(
                    "mesh contains non-finite vertices",
                ));
            }
            for command in &data.commands {
                if command.indices.start > command.indices.end
                    || command.indices.end as usize > data.indices.len()
                    || !command.clip_rect.min.is_finite()
                    || !command.clip_rect.max.is_finite()
                    || !store.textures.contains_key(&command.texture)
                {
                    return Err(RenderError::InvalidDrawData(
                        "draw range, clip rectangle, or texture ID is invalid",
                    ));
                }
            }
            let vertices = bytemuck::cast_slice::<Vertex, u8>(&data.vertices);
            let indices = bytemuck::cast_slice::<u32, u8>(&data.indices);
            let max_buffer = self.device.limits().max_buffer_size;
            if vertices.len() as u64 > max_buffer || indices.len() as u64 > max_buffer {
                return Err(RenderError::InvalidDrawData(
                    "mesh exceeds the GPU buffer limit",
                ));
            }
            let grow_vertices = vertices.len() as u64 > self.vertex_capacity;
            let grow_indices = indices.len() as u64 > self.index_capacity;
            if grow_vertices {
                self.vertex_capacity = (vertices.len() as u64).next_power_of_two().min(max_buffer);
                self.vertices = create_buffer(
                    &self.device,
                    self.vertex_capacity,
                    wgpu::BufferUsages::VERTEX,
                    "zaxis vertices",
                );
            }
            if grow_indices {
                self.index_capacity = (indices.len() as u64).next_power_of_two().min(max_buffer);
                self.indices = create_buffer(
                    &self.device,
                    self.index_capacity,
                    wgpu::BufferUsages::INDEX,
                    "zaxis indices",
                );
            }
            self.stats.geometry_upload_bytes += write_ranges(
                &self.queue,
                &self.vertices,
                vertices,
                std::mem::size_of::<Vertex>(),
                vertex_ranges.filter(|_| !grow_vertices),
            );
            self.stats.geometry_upload_bytes += write_ranges(
                &self.queue,
                &self.indices,
                indices,
                std::mem::size_of::<u32>(),
                index_ranges.filter(|_| !grow_indices),
            );
            self.uploaded = Some(revision);
            self.uploaded_sizes = [data.vertices.len(), data.indices.len()];
            self.stats.geometry_uploads += 1;
        }
        Ok(())
    }
}

fn valid_ranges(ranges: &[Range<usize>], len: usize) -> bool {
    ranges.iter().all(|r| r.start <= r.end && r.end <= len)
}

fn valid_indices(data: &DrawData, ranges: Option<&[Range<usize>]>) -> bool {
    let valid = |slice: &[u32]| slice.iter().all(|i| (*i as usize) < data.vertices.len());
    ranges.map_or_else(
        || valid(&data.indices),
        |ranges| ranges.iter().all(|r| valid(&data.indices[r.clone()])),
    )
}

fn valid_vertices(data: &DrawData, ranges: Option<&[Range<usize>]>) -> bool {
    let valid = |slice: &[Vertex]| {
        slice.iter().all(|v| {
            v.position
                .iter()
                .chain(&v.uv)
                .chain(&v.color)
                .all(|x| x.is_finite())
        })
    };
    ranges.map_or_else(
        || valid(&data.vertices),
        |ranges| ranges.iter().all(|r| valid(&data.vertices[r.clone()])),
    )
}

fn write_ranges(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    bytes: &[u8],
    stride: usize,
    ranges: Option<&[Range<usize>]>,
) -> u64 {
    // Dense updates are cheaper as one transfer than hundreds of small staging
    // allocations. Unchanged entries were already validated at the parent revision.
    let ranges = ranges.filter(|ranges| {
        ranges
            .iter()
            .fold(0usize, |count, range| count.saturating_add(range.len()))
            <= bytes.len() / stride / 2
    });
    if let Some(ranges) = ranges {
        let mut written = 0;
        for range in ranges {
            let start = range.start * stride;
            let end = range.end * stride;
            if start != end {
                queue.write_buffer(buffer, start as u64, &bytes[start..end]);
                written += (end - start) as u64;
            }
        }
        written
    } else if !bytes.is_empty() {
        queue.write_buffer(buffer, 0, bytes);
        bytes.len() as u64
    } else {
        0
    }
}

pub(super) fn create_buffer(
    device: &wgpu::Device,
    size: u64,
    usage: wgpu::BufferUsages,
    label: &str,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
