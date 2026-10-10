//! Textures of the backdrop filter. A [`Set`] holds everything one group of effects needs
//! and is created only when a frame has effects; levels are created on first use.

use super::{params::MAX_LEVEL, BlurDevice};

/// A texture the filter renders into or samples, with its bind group.
pub struct Tex {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub group: wgpu::BindGroup,
    pub bytes: u64,
}

impl Tex {
    pub fn new(dev: &BlurDevice, [width, height]: [u32; 2], format: wgpu::TextureFormat) -> Self {
        let texture = dev.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zaxis backdrop texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let group = dev.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zaxis backdrop texture group"),
            layout: dev.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(dev.sampler),
                },
            ],
        });
        let bytes = u64::from(width)
            * u64::from(height)
            * u64::from(format.block_copy_size(None).unwrap_or(8));
        Self {
            texture,
            view,
            group,
            bytes,
        }
    }
}

/// The textures of one group of effects: a snapshot of the canvas (the sharp backdrop and
/// level 0 of the pyramid), the pyramid, per last level the horizontally filtered rows and
/// the result, and for materials the result at full resolution.
#[derive(Default)]
pub struct Set {
    pub snapshot: Option<Tex>,
    pub pyramid: Vec<Option<Tex>>,
    pub scratch: Vec<Option<Tex>>,
    pub result: Vec<Option<Tex>>,
    pub resolved: Option<Tex>,
    pub material_group: Option<wgpu::BindGroup>,
    /// Signature of the effects whose result the textures hold.
    pub signature: Option<u64>,
}

impl Set {
    pub fn bytes(&self) -> u64 {
        let all = self
            .pyramid
            .iter()
            .chain(&self.scratch)
            .chain(&self.result)
            .chain([&self.snapshot, &self.resolved]);
        all.flatten().map(|t| t.bytes).sum()
    }

    /// The texture holding level `level` of the pyramid.
    pub fn level(&self, level: u32) -> &Tex {
        if level == 0 {
            self.snapshot.as_ref().expect("snapshot")
        } else {
            self.pyramid[level as usize]
                .as_ref()
                .expect("pyramid level")
        }
    }

    /// Make the textures for `levels` (pyramid depth) and the filtered levels in `last`
    /// exist at `size`. Returns whether the material group must be rebuilt.
    pub fn ensure(
        &mut self,
        dev: &BlurDevice,
        size: [u32; 2],
        depth: u32,
        last: &[bool],
        resolved: bool,
    ) {
        let n = MAX_LEVEL as usize + 1;
        for list in [&mut self.pyramid, &mut self.scratch, &mut self.result] {
            list.resize_with(n, || None);
        }
        if self.snapshot.is_none() {
            self.snapshot = Some(Tex::new(dev, size, dev.format));
            self.material_group = None;
            self.signature = None;
        }
        for k in 1..=depth {
            let at = super::params::level_size(size, k);
            self.pyramid[k as usize].get_or_insert_with(|| Tex::new(dev, at, dev.intermediate));
        }
        for (k, used) in last.iter().enumerate() {
            if *used {
                let at = super::params::level_size(size, k as u32);
                self.scratch[k].get_or_insert_with(|| Tex::new(dev, at, dev.intermediate));
                self.result[k].get_or_insert_with(|| Tex::new(dev, at, dev.intermediate));
            }
        }
        if resolved && self.resolved.is_none() {
            self.resolved = Some(Tex::new(dev, size, dev.intermediate));
            self.material_group = None;
        }
        if self.material_group.is_none() {
            if let (Some(layout), Some(resolved)) = (dev.backdrop_layout, &self.resolved) {
                let snapshot = self.snapshot.as_ref().expect("snapshot");
                self.material_group =
                    Some(dev.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("zaxis material backdrop"),
                        layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&snapshot.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(&resolved.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Sampler(dev.sampler),
                            },
                        ],
                    }));
            }
        }
    }
}
