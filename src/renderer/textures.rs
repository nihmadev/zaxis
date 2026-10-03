//! Versioned uploads, shared allocations, independent samplers, managed LRU.
use super::{diagnostics::RendererStage, RenderError, Renderer};
use crate::{DrawData, TextureFilter, TextureId};
use std::sync::{Arc, Weak};

pub(super) struct GpuTexture {
    pub(super) texture: wgpu::Texture,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) size: [u32; 2],
    pub(super) revision: u64,
    pub(super) filter: TextureFilter,
    pub(super) managed: bool,
    pub(super) last_used: u64,
    pub(super) allocation: u64,
    pub(super) pixels: Weak<Vec<u8>>,
}
pub(super) fn create_bindings(device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::Sampler) {
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
pub(super) fn nearest_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zaxis nearest sampler"),
        ..Default::default()
    })
}
impl Renderer {
    pub(super) fn prepare_textures(&mut self, data: &DrawData) -> Result<(), RenderError> {
        if self.texture_source != Some(data.source) {
            self.textures.retain(|id, _| *id == TextureId::WHITE);
            self.texture_source = Some(data.source);
        }
        let active: std::collections::HashSet<_> =
            data.commands.iter().map(|c| c.texture).collect();
        let limit = self.device.limits().max_texture_dimension_2d;
        let mut payloads = std::collections::HashSet::new();
        let mut allocations = std::collections::HashSet::new();
        let mut active_bytes = 0u64;
        for image in &data.textures {
            if image.id == TextureId::WHITE
                || !payloads.insert(image.id)
                || image.size.contains(&0)
                || image.size.iter().any(|s| *s > limit)
                || image.pixels.len() as u64
                    != u64::from(image.size[0]) * u64::from(image.size[1]) * 4
            {
                return Err(RenderError::Texture(format!(
                    "invalid texture {:?}, dimensions {:?}, device limit {limit}",
                    image.id, image.size
                )));
            }
            if active.contains(&image.id)
                && data
                    .texture_options
                    .get(&image.id)
                    .is_some_and(|o| o.managed)
                && allocations.insert((Arc::as_ptr(&image.pixels), image.size, image.revision))
            {
                active_bytes += image.pixels.len() as u64;
            }
        }
        if active
            .iter()
            .any(|id| *id != TextureId::WHITE && !payloads.contains(id))
        {
            return Err(RenderError::InvalidDrawData(
                "complete texture payload required for every referenced ID",
            ));
        }
        if active_bytes > data.texture_budget_bytes {
            return Err(RenderError::Texture(format!(
                "visible image set needs {active_bytes} bytes, GPU budget is {}",
                data.texture_budget_bytes
            )));
        }
        if active
            .iter()
            .filter(|id| data.texture_options.get(id).is_some_and(|o| o.managed))
            .count()
            > data.texture_binding_budget
        {
            return Err(RenderError::Texture(
                "visible image bindings exceed binding budget".into(),
            ));
        }
        self.evict_images(
            data.texture_budget_bytes.saturating_sub(active_bytes),
            &active,
        );
        for image in &data.textures {
            let options = data
                .texture_options
                .get(&image.id)
                .copied()
                .unwrap_or_default();
            if options.managed && !active.contains(&image.id) {
                continue;
            }
            let lookup = self.diagnostics.start();
            let cached = self
                .textures
                .get(&image.id)
                .is_some_and(|t| t.size == image.size && t.revision == image.revision);
            self.diagnostics.end(RendererStage::TextureLookup, lookup);
            let sampler = if options.filter == TextureFilter::Nearest {
                &self.nearest_sampler
            } else {
                &self.sampler
            };
            if cached {
                let t = self.textures.get_mut(&image.id).unwrap();
                if t.filter != options.filter {
                    t.bind_group = binding(
                        &self.device,
                        &self.texture_layout,
                        sampler,
                        &t.texture,
                        &mut self.diagnostics,
                    );
                    t.filter = options.filter;
                }
                t.managed = options.managed;
                t.last_used = self.stats.presented_frames;
                continue;
            }
            let shared = self
                .textures
                .iter()
                .find(|(id, t)| {
                    **id != image.id
                        && t.size == image.size
                        && t.revision == image.revision
                        && t.pixels
                            .upgrade()
                            .is_some_and(|p| Arc::ptr_eq(&p, &image.pixels))
                })
                .map(|(_, t)| (t.texture.clone(), t.allocation));
            let mut gpu = if let Some((texture, allocation)) = shared {
                GpuTexture {
                    bind_group: binding(
                        &self.device,
                        &self.texture_layout,
                        sampler,
                        &texture,
                        &mut self.diagnostics,
                    ),
                    texture,
                    size: image.size,
                    revision: image.revision,
                    filter: options.filter,
                    managed: options.managed,
                    last_used: self.stats.presented_frames,
                    allocation,
                    pixels: Arc::downgrade(&image.pixels),
                }
            } else {
                let reusable = self
                    .textures
                    .get(&image.id)
                    .filter(|t| t.size == image.size)
                    .is_some_and(|t| {
                        self.textures
                            .values()
                            .filter(|o| o.allocation == t.allocation)
                            .count()
                            == 1
                    });
                let gpu = if reusable {
                    let mut t = self.textures.remove(&image.id).unwrap();
                    let start = self.diagnostics.start();
                    write_texture(&self.queue, &t.texture, image.size, &image.pixels);
                    self.diagnostics.end(RendererStage::WriteTextureCpu, start);
                    if t.filter != options.filter {
                        t.bind_group = binding(
                            &self.device,
                            &self.texture_layout,
                            sampler,
                            &t.texture,
                            &mut self.diagnostics,
                        );
                    }
                    t.revision = image.revision;
                    self.stats.texture_reuploads += 1;
                    t
                } else {
                    self.stats.texture_creations += 1;
                    let mut t = create_texture_profiled(
                        &self.device,
                        &self.queue,
                        &self.texture_layout,
                        sampler,
                        image.size,
                        &image.pixels,
                        image.revision,
                        &mut self.diagnostics,
                    );
                    t.allocation = self.stats.texture_creations;
                    t
                };
                self.stats.texture_uploads += 1;
                self.stats.texture_upload_bytes += image.pixels.len() as u64;
                gpu
            };
            gpu.pixels = Arc::downgrade(&image.pixels);
            gpu.filter = options.filter;
            gpu.managed = options.managed;
            gpu.last_used = self.stats.presented_frames;
            self.textures.insert(image.id, gpu);
        }
        while self.textures.values().filter(|t| t.managed).count() > data.texture_binding_budget {
            let id = self
                .textures
                .iter()
                .filter(|(id, t)| t.managed && !active.contains(id))
                .min_by_key(|(id, t)| (t.last_used, id.0))
                .map(|(&id, _)| id);
            let Some(id) = id else {
                break;
            };
            self.textures.remove(&id);
            self.stats.texture_evictions += 1;
        }
        self.stats.image_resident_bytes_estimate = self.image_bytes();
        Ok(())
    }
    fn image_bytes(&self) -> u64 {
        let mut seen = std::collections::HashSet::new();
        self.textures
            .values()
            .filter(|t| t.managed && seen.insert(t.allocation))
            .map(|t| u64::from(t.size[0]) * u64::from(t.size[1]) * 4)
            .sum()
    }
    fn evict_images(&mut self, budget: u64, active: &std::collections::HashSet<TextureId>) {
        loop {
            let is_idle = |t: &GpuTexture| {
                t.managed
                    && !self
                        .textures
                        .iter()
                        .any(|(id, o)| active.contains(id) && o.allocation == t.allocation)
            };
            let mut seen = std::collections::HashSet::new();
            let idle_bytes: u64 = self
                .textures
                .values()
                .filter(|t| is_idle(t) && seen.insert(t.allocation))
                .map(|t| u64::from(t.size[0]) * u64::from(t.size[1]) * 4)
                .sum();
            if idle_bytes <= budget {
                break;
            }
            let allocation = self
                .textures
                .iter()
                .filter(|(_, t)| is_idle(t))
                .min_by_key(|(id, t)| (t.last_used, id.0))
                .map(|(_, t)| t.allocation);
            let Some(allocation) = allocation else {
                break;
            };
            self.textures
                .retain(|_, t| !t.managed || t.allocation != allocation);
            self.stats.texture_evictions += 1;
        }
    }
    pub fn clear_image_textures(&mut self) {
        let mut allocations = std::collections::HashSet::new();
        self.textures.retain(|_, t| {
            if t.managed {
                allocations.insert(t.allocation);
                false
            } else {
                true
            }
        });
        self.stats.texture_evictions += allocations.len() as u64;
        self.stats.image_resident_bytes_estimate = 0;
    }
    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }
    pub(super) fn pipeline_for(
        &self,
        command: &crate::DrawCommand,
        data: &DrawData,
    ) -> &wgpu::RenderPipeline {
        if command.scroll_hint {
            &self.scroll_hint_pipeline
        } else if data
            .texture_options
            .get(&command.texture)
            .is_some_and(|o| o.managed && o.filter == TextureFilter::Linear)
        {
            &self.image_pipeline
        } else {
            &self.pipeline
        }
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
pub(super) fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    size: [u32; 2],
    pixels: &[u8],
    revision: u64,
) -> GpuTexture {
    create_texture_profiled(
        device,
        queue,
        layout,
        sampler,
        size,
        pixels,
        revision,
        &mut Default::default(),
    )
}
fn create_texture_profiled(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    size: [u32; 2],
    pixels: &[u8],
    revision: u64,
    diagnostics: &mut super::diagnostics::Diagnostics,
) -> GpuTexture {
    let start = diagnostics.start();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("zaxis RGBA texture"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    diagnostics.end(RendererStage::CreateTextureCpu, start);
    let start = diagnostics.start();
    write_texture(queue, &texture, size, pixels);
    diagnostics.end(RendererStage::WriteTextureCpu, start);
    let bind_group = binding(device, layout, sampler, &texture, diagnostics);
    GpuTexture {
        texture,
        bind_group,
        size,
        revision,
        filter: TextureFilter::Linear,
        managed: false,
        last_used: 0,
        allocation: 0,
        pixels: Weak::new(),
    }
}
pub(super) fn write_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: [u32; 2],
    pixels: &[u8],
) {
    queue.write_texture(
        texture.as_image_copy(),
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size[0] * 4),
            rows_per_image: Some(size[1]),
        },
        wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
    );
}
