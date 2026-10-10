//! The host's multisampled color and depth attachments, recreated on resize.

use crate::gpu::Gpu;

pub const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Targets {
    pub size: [u32; 2],
    pub samples: u32,
    pub msaa: Option<wgpu::TextureView>,
    pub depth: wgpu::TextureView,
}

impl Targets {
    /// `view_format` is the format the scene renders to (the sRGB view of the target).
    pub fn new(gpu: &Gpu, size: [u32; 2], samples: u32, view_format: wgpu::TextureFormat) -> Self {
        let make = |format, usage| {
            gpu.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("host attachment"),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        Self {
            size,
            samples,
            msaa: (samples > 1).then(|| make(view_format, wgpu::TextureUsages::RENDER_ATTACHMENT)),
            depth: make(DEPTH, wgpu::TextureUsages::RENDER_ATTACHMENT),
        }
    }
}
