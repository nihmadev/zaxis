//! Pipelines of user materials, created on first use and kept in a bounded cache.

use crate::{renderer::textures, MaterialId, MaterialSource, Vertex};
use std::{collections::HashMap, sync::Arc};

/// Pipelines kept between frames. The pipelines a frame draws with are never evicted, so a
/// frame may exceed it; the registry limit bounds that.
pub const MAX_PIPELINES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    id: MaterialId,
    format: wgpu::TextureFormat,
    samples: u32,
}

struct Entry {
    wgsl: Arc<str>,
    /// `None` after a failed build, so a broken material is not rebuilt every frame.
    pipeline: Option<wgpu::RenderPipeline>,
    used: u64,
}

/// Layouts, shared bindings and the pipeline cache of one device. Windows of the device
/// share one value; dropping the renderer drops every pipeline, and the next renderer builds
/// them again from the sources in the draw data.
pub struct MaterialPipelines {
    device: wgpu::Device,
    pub backdrop_layout: wgpu::BindGroupLayout,
    pub params_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    /// Bound as the backdrop of draws that do not read it: transparent black.
    pub no_backdrop: wgpu::BindGroup,
    entries: HashMap<Key, Entry>,
    clock: u64,
    errors: Vec<String>,
    builds: u64,
    failures: u64,
}

impl MaterialPipelines {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let backdrop_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zaxis material backdrop layout"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let params_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zaxis material params layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    // Checked against the 1 KiB binding at draw time, whatever the schema size.
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zaxis material pipeline layout"),
            bind_group_layouts: &[
                Some(viewport_layout),
                Some(texture_layout),
                Some(&backdrop_layout),
                Some(&params_layout),
            ],
            immediate_size: 0,
        });
        let sampler = textures::nearest_sampler(device);
        let blank = blank_view(device, queue);
        let no_backdrop = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zaxis material no backdrop"),
            layout: &backdrop_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&blank),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&blank),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Self {
            device: device.clone(),
            backdrop_layout,
            params_layout,
            pipeline_layout,
            no_backdrop,
            entries: HashMap::new(),
            clock: 0,
            errors: Vec::new(),
            builds: 0,
            failures: 0,
        }
    }

    /// Start a frame: pipelines used from now on are safe from eviction until the next one.
    pub fn begin_frame(&mut self) {
        self.clock += 1;
    }

    /// The pipeline of `source`, built now if it is new. `None` when the device rejected it;
    /// the reason is kept for [`take_errors`](Self::take_errors) and the material is not
    /// tried again until its source changes.
    pub fn pipeline(
        &mut self,
        source: &MaterialSource,
        format: wgpu::TextureFormat,
        samples: u32,
    ) -> Option<wgpu::RenderPipeline> {
        let key = Key {
            id: source.id,
            format,
            samples,
        };
        if let Some(entry) = self.entries.get_mut(&key) {
            if Arc::ptr_eq(&entry.wgsl, &source.wgsl) || entry.wgsl == source.wgsl {
                entry.used = self.clock;
                return entry.pipeline.clone();
            }
        }
        let pipeline = self.build(source, format, samples);
        self.entries.insert(
            key,
            Entry {
                wgsl: Arc::clone(&source.wgsl),
                pipeline: pipeline.clone(),
                used: self.clock,
            },
        );
        pipeline
    }

    /// End of the frame's lookups: drop the least recently used pipelines past the limit.
    pub fn trim(&mut self) {
        while self.entries.len() > MAX_PIPELINES {
            let oldest = self
                .entries
                .iter()
                .filter(|(_, e)| e.used < self.clock)
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| *k);
            match oldest {
                Some(key) => self.entries.remove(&key),
                None => break,
            };
        }
    }

    /// Pipelines currently cached, failed builds included.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Pipelines built and builds that failed, since creation.
    pub fn counters(&self) -> (u64, u64) {
        (self.builds, self.failures)
    }
    /// Messages of failed builds since the last call.
    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
    }

    fn build(
        &mut self,
        source: &MaterialSource,
        format: wgpu::TextureFormat,
        samples: u32,
    ) -> Option<wgpu::RenderPipeline> {
        self.builds += 1;
        // Sources are validated before registration, so a failure here is a device limit or
        // a driver; a native device reports it synchronously through the error scope. A
        // browser reports asynchronously and the pipeline fails on use instead.
        #[cfg(not(target_arch = "wasm32"))]
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = create(&self.device, &self.pipeline_layout, source, format, samples);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(error) = pollster::block_on(scope.pop()) {
            self.failures += 1;
            self.errors
                .push(format!("material `{}`: {error}", source.label));
            return None;
        }
        Some(pipeline)
    }
}

fn create(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    source: &MaterialSource,
    format: wgpu::TextureFormat,
    samples: u32,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&source.label),
        source: wgpu::ShaderSource::Wgsl(source.wgsl.as_ref().into()),
    });
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&source.label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &ATTRIBUTES,
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_material"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                // The shader returns premultiplied color, scaled once by coverage.
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

/// A 1×1 transparent texture view, bound where a shader's input is unused.
fn blank_view(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    use wgpu::util::DeviceExt;
    device
        .create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("zaxis blank texture"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[0; 4],
        )
        .create_view(&Default::default())
}
