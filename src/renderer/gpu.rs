//! The part of a renderer that belongs to the device, not to a window: queue handles,
//! layouts and samplers, the pipelines and texture store shared by a family of renderers, and
//! the per-renderer geometry buffers, viewport uniform, material blocks and backdrop targets.
//!
//! [`Renderer`](super::Renderer) adds a surface to it; [`EmbeddedRenderer`] adds the format
//! of a pass owned by the host. Neither needs the other.
//!
//! [`EmbeddedRenderer`]: super::EmbeddedRenderer

use super::{
    blur::BlurRenderer,
    diagnostics::Diagnostics,
    geometry::create_buffer,
    materials::{MaterialPipelines, MaterialUniforms},
    pipeline_cache::{BlurCache, PipelineCache},
    textures::{self, create_texture, TextureStore},
    viewport, RendererStats,
};
use std::sync::{Arc, Mutex};

pub(super) struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// Whether the adapter renders to and filters `Rgba16Float`, for the backdrop pyramid.
    pub half_float: bool,
    /// Shared by the family: built-in pipelines per target, backdrop pipelines per format.
    pub pipelines: Arc<Mutex<PipelineCache>>,
    pub blur_pipelines: Arc<Mutex<BlurCache>>,
    pub materials: Arc<Mutex<MaterialPipelines>>,
    pub store: Arc<Mutex<TextureStore>>,
    pub device_lost: Arc<Mutex<Option<String>>>,
    pub viewport_layout: wgpu::BindGroupLayout,
    pub texture_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub nearest_sampler: wgpu::Sampler,
    /// Per renderer: freed when it is dropped.
    pub blur: Option<BlurRenderer>,
    pub blur_idle_frames: u32,
    pub material_uniforms: MaterialUniforms,
    pub material_errors: Vec<String>,
    pub uniform: wgpu::Buffer,
    pub viewport_group: wgpu::BindGroup,
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub vertex_capacity: u64,
    pub index_capacity: u64,
    pub uploaded: Option<(u64, u64)>,
    pub uploaded_sizes: [usize; 2],
    pub viewport_value: [f32; 8],
    pub stats: RendererStats,
    pub diagnostics: Diagnostics,
}

impl Gpu {
    /// Layouts, samplers, an empty texture store and buffers for a device nothing has used.
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        half_float: bool,
        device_lost: Arc<Mutex<Option<String>>>,
    ) -> Self {
        let viewport_layout = viewport::create_layout(&device);
        let (uniform, viewport_group) = viewport::create_uniform(&device, &viewport_layout);
        let (texture_layout, sampler) = textures::create_bindings(&device);
        let nearest_sampler = textures::nearest_sampler(&device);
        let materials = MaterialPipelines::new(&device, &queue, &viewport_layout, &texture_layout);
        let material_uniforms = MaterialUniforms::new(&device, &materials.params_layout);
        let white = create_texture(
            &device,
            &queue,
            &texture_layout,
            &sampler,
            [1, 1],
            &[255; 4],
            0,
        );
        Self {
            vertices: create_buffer(&device, 256, wgpu::BufferUsages::VERTEX, "zaxis vertices"),
            indices: create_buffer(&device, 256, wgpu::BufferUsages::INDEX, "zaxis indices"),
            pipelines: Arc::new(Mutex::new(PipelineCache::new(
                &device,
                &viewport_layout,
                &texture_layout,
            ))),
            device,
            queue,
            half_float,
            blur_pipelines: Arc::default(),
            materials: Arc::new(Mutex::new(materials)),
            store: Arc::new(Mutex::new(TextureStore::new(white))),
            device_lost,
            viewport_layout,
            texture_layout,
            sampler,
            nearest_sampler,
            blur: None,
            blur_idle_frames: 0,
            material_uniforms,
            material_errors: Vec::new(),
            uniform,
            viewport_group,
            vertex_capacity: 256,
            index_capacity: 256,
            uploaded: None,
            uploaded_sizes: [0; 2],
            viewport_value: [1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            stats: RendererStats::default(),
            diagnostics: Default::default(),
        }
    }

    /// Another renderer on the same device. Pipelines, layouts, samplers, the device-loss
    /// state and the texture store are shared; buffers, the viewport uniform, material blocks
    /// and backdrop targets are the new renderer's own. Cheap: nothing is compiled.
    pub fn sibling(&self) -> Self {
        self.store.lock().expect("texture store mutex").shared = true;
        let (uniform, viewport_group) =
            viewport::create_uniform(&self.device, &self.viewport_layout);
        let params_layout = self
            .materials
            .lock()
            .expect("material pipelines mutex")
            .params_layout
            .clone();
        Self {
            device: self.device.clone(),
            queue: self.queue.clone(),
            half_float: self.half_float,
            pipelines: Arc::clone(&self.pipelines),
            blur_pipelines: Arc::clone(&self.blur_pipelines),
            materials: Arc::clone(&self.materials),
            store: Arc::clone(&self.store),
            device_lost: Arc::clone(&self.device_lost),
            viewport_layout: self.viewport_layout.clone(),
            texture_layout: self.texture_layout.clone(),
            sampler: self.sampler.clone(),
            nearest_sampler: self.nearest_sampler.clone(),
            blur: None,
            blur_idle_frames: 0,
            material_uniforms: MaterialUniforms::new(&self.device, &params_layout),
            material_errors: Vec::new(),
            uniform,
            viewport_group,
            vertices: create_buffer(
                &self.device,
                256,
                wgpu::BufferUsages::VERTEX,
                "zaxis vertices",
            ),
            indices: create_buffer(
                &self.device,
                256,
                wgpu::BufferUsages::INDEX,
                "zaxis indices",
            ),
            vertex_capacity: 256,
            index_capacity: 256,
            uploaded: None,
            uploaded_sizes: [0; 2],
            viewport_value: [1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            stats: RendererStats::default(),
            diagnostics: Default::default(),
        }
    }

    /// Why the device was lost, once it was.
    pub fn lost(&self) -> Option<String> {
        self.device_lost
            .lock()
            .expect("device callback mutex")
            .clone()
    }

    /// Format of the backdrop pyramid for a canvas of `canvas`: half-float where the adapter
    /// renders to and filters it, else the canvas format.
    pub fn intermediate(&self, canvas: wgpu::TextureFormat) -> wgpu::TextureFormat {
        if self.half_float {
            wgpu::TextureFormat::Rgba16Float
        } else {
            canvas
        }
    }
}

/// Whether `adapter` renders to and filters `Rgba16Float` as the backdrop pyramid needs.
pub(super) fn half_float(adapter: &wgpu::Adapter) -> bool {
    super::blur::intermediate_format(adapter, wgpu::TextureFormat::Rgba8Unorm)
        == wgpu::TextureFormat::Rgba16Float
}
