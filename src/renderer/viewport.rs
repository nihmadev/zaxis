//! Logical viewport uniforms and DPI-aware physical clipping.

use super::{gpu::Gpu, RenderError};
use crate::{DrawData, Rect};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

/// Bytes of the viewport uniform: logical size, scale, padding, region origin, reserved.
pub const UNIFORM_BYTES: u64 = 32;

pub fn create_bindings(
    device: &wgpu::Device,
) -> (wgpu::BindGroupLayout, wgpu::Buffer, wgpu::BindGroup) {
    let layout = create_layout(device);
    let (uniform, viewport_group) = create_uniform(device, &layout);
    (layout, uniform, viewport_group)
}

pub(super) fn create_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("zaxis viewport layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(UNIFORM_BYTES),
            },
            count: None,
        }],
    })
}

/// The per-window viewport uniform; the layout is shared by every window.
pub(super) fn create_uniform(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("zaxis viewport"),
        contents: bytemuck::cast_slice(&[1.0_f32, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let viewport_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zaxis viewport bind group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }],
    });
    (uniform, viewport_group)
}

pub(super) fn validate(data: &DrawData) -> Result<(), RenderError> {
    if data
        .commands
        .iter()
        .any(|command| command.scroll_hint && command.blur.is_some())
    {
        return Err(RenderError::InvalidDrawData(
            "scroll hint and backdrop blur cannot share a command",
        ));
    }
    if data.commands.iter().any(|command| {
        command
            .blur
            .is_some_and(|r| !r.is_finite() || r <= 0.0 || r > 64.0)
    }) {
        return Err(RenderError::InvalidDrawData(
            "blur sigma must be finite and in (0, 64]",
        ));
    }
    if data.commands.iter().filter(|c| c.blur.is_some()).count() > super::blur::plan::MAX_EFFECTS {
        return Err(RenderError::InvalidDrawData(
            "too many backdrop effects in one frame",
        ));
    }
    if !data.logical_size.is_finite()
        || data.logical_size.min_element() <= 0.0
        || !data.scale_factor.is_finite()
        || data.scale_factor <= 0.0
    {
        return Err(RenderError::InvalidDrawData(
            "viewport and scale must be finite and positive",
        ));
    }
    Ok(())
}

impl Gpu {
    /// Write the uniform for `data` drawn into the area of the target that starts at
    /// `origin` (physical pixels); skipped when it already holds these values.
    pub(super) fn prepare_viewport(&mut self, data: &DrawData, origin: [u32; 2]) {
        let viewport = [
            data.logical_size.x,
            data.logical_size.y,
            data.scale_factor,
            0.0,
            origin[0] as f32,
            origin[1] as f32,
            0.0,
            0.0,
        ];
        if self.viewport_value != viewport {
            self.queue
                .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&viewport));
            self.viewport_value = viewport;
        }
    }
}

pub fn scissor(rect: Rect, scale: f32, viewport: PhysicalSize<u32>) -> Option<[u32; 4]> {
    // Keep every rasterized pixel inside the logical clip. Outward rounding
    // leaks content across adjacent panels at fractional DPI/drag positions.
    if rect.is_empty() {
        return None;
    }
    let x0 = (rect.min.x * scale)
        .ceil()
        .clamp(0.0, viewport.width as f32) as u32;
    let y0 = (rect.min.y * scale)
        .ceil()
        .clamp(0.0, viewport.height as f32) as u32;
    let x1 = (rect.max.x * scale)
        .floor()
        .clamp(0.0, viewport.width as f32) as u32;
    let y1 = (rect.max.y * scale)
        .floor()
        .clamp(0.0, viewport.height as f32) as u32;
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)])
}
