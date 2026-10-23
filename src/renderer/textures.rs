//! Versioned uploads, partial updates, shared allocations, independent samplers, managed LRU.
use super::{diagnostics::RendererStage, gpu::Gpu, Renderer};
use crate::{PixelFormat, TextureFilter, TextureId};
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

mod evict;
mod prepare;
mod update;
mod upload;

/// How the bytes of a payload are stored on this device: the host's layout, and whether the
/// device cannot sample it, so that the written rows are swapped to RGBA on the way in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Texels {
    pub format: PixelFormat,
    pub convert: bool,
}

impl Texels {
    pub const RGBA: Self = Self {
        format: PixelFormat::Rgba8Srgb,
        convert: false,
    };

    /// The texture format the bytes are uploaded to without any change.
    pub fn gpu_format(self) -> wgpu::TextureFormat {
        if self.format.is_bgr() && !self.convert {
            wgpu::TextureFormat::Bgra8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8UnormSrgb
        }
    }
}

pub struct GpuTexture {
    pub(super) texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
    pub(super) size: [u32; 2],
    pub(super) texels: Texels,
    pub(super) revision: u64,
    pub(super) filter: TextureFilter,
    pub(super) managed: bool,
    pub(super) last_used: u64,
    pub(super) allocation: u64,
    pub(super) pixels: Weak<Vec<u8>>,
    /// Registered by the host; never evicted, replaced or written by the renderer.
    pub(super) external: bool,
}
/// GPU textures shared by a renderer and its siblings.
pub(super) struct TextureStore {
    pub(super) textures: HashMap<TextureId, GpuTexture>,
    /// Producer of the last frame; a change discards textures unless `shared`.
    pub(super) source: Option<u64>,
    /// Sibling renderers present contexts that allocate IDs from one allocator, so a
    /// change of producer must keep the textures.
    pub(super) shared: bool,
    /// Advances once per presented frame; orders least-recently-used eviction.
    pub(super) clock: u64,
    /// Last allocation number handed to a created texture; unique across siblings.
    pub(super) allocations: u64,
    /// Last number of an external registration.
    pub(super) next_external: u64,
}

impl TextureStore {
    pub(super) fn new(white: GpuTexture) -> Self {
        Self {
            textures: HashMap::from([(TextureId::WHITE, white)]),
            source: None,
            shared: false,
            clock: 0,
            allocations: 0,
            next_external: 0,
        }
    }
}

pub fn create_bindings(device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::Sampler) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("zaxis texture layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zaxis linear sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    (layout, sampler)
}
pub fn nearest_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zaxis nearest sampler"),
        ..Default::default()
    })
}

impl Gpu {
    /// The sampler of a binding with `filter`.
    pub(super) fn sampler_for(&self, filter: TextureFilter) -> &wgpu::Sampler {
        if filter == TextureFilter::Nearest {
            &self.nearest_sampler
        } else {
            &self.sampler
        }
    }

    /// A new bind group of `texture` with the sampler of `filter`.
    pub(super) fn bind(
        &mut self,
        texture: &wgpu::Texture,
        filter: TextureFilter,
    ) -> wgpu::BindGroup {
        self.stats.texture_bind_group_creations += 1;
        let sampler = if filter == TextureFilter::Nearest {
            &self.nearest_sampler
        } else {
            &self.sampler
        };
        binding(
            &self.device,
            &self.texture_layout,
            sampler,
            texture,
            &mut self.diagnostics,
        )
    }

    /// How a payload of `format` is stored on this device.
    pub(super) fn texels(&self, format: PixelFormat) -> Texels {
        Texels {
            format,
            convert: format.is_bgr() && !self.bgra_native,
        }
    }
}

impl Renderer {
    /// Drop every managed image texture, including those used by sibling renderers.
    pub fn clear_image_textures(&mut self) {
        self.gpu.clear_image_textures();
    }
    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.gpu.device.limits().max_texture_dimension_2d
    }
    /// Convert BGRA payloads to RGBA on upload even though the device could sample them, as
    /// a device without BGRA textures (WebGL2) needs. For tests of that path.
    #[doc(hidden)]
    pub fn set_bgra_conversion(&mut self, convert: bool) {
        self.gpu.bgra_native = !convert;
    }
}
pub(super) fn binding(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    texture: &wgpu::Texture,
    diagnostics: &mut super::diagnostics::Diagnostics,
) -> wgpu::BindGroup {
    let start = diagnostics.start();
    let view = texture.create_view(&Default::default());
    diagnostics.end(RendererStage::CreateViewCpu, start);
    let start = diagnostics.start();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zaxis texture binding"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    diagnostics.end(RendererStage::CreateBindGroupCpu, start);
    group
}
pub fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    size: [u32; 2],
    pixels: &[u8],
    revision: u64,
) -> GpuTexture {
    upload::create_texture_profiled(
        device,
        queue,
        layout,
        sampler,
        size,
        pixels,
        revision,
        Texels::RGBA,
        &mut Default::default(),
    )
}
/// Whether two payload allocations are the same buffer.
fn same_buffer(a: &Weak<Vec<u8>>, b: &Arc<Vec<u8>>) -> bool {
    a.upgrade().is_some_and(|p| Arc::ptr_eq(&p, b))
}
