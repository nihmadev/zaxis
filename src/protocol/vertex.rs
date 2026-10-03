//! GPU-compatible vertex layout.

use bytemuck::{Pod, Zeroable};

/// GPU vertex: position in logical pixels, normalized UV, and linear straight-alpha color.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}
