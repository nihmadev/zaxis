//! Ordered backdrop effects. Offscreen attachments are allocated only for blur frames.
use super::{textures::TextureStore, viewport::scissor, Renderer};
use std::sync::Arc;
use crate::{Color, DrawData, Rect, Vec2};
use wgpu::util::DeviceExt;

struct Target {
    view: wgpu::TextureView,
    group: wgpu::BindGroup,
}

/// Pipelines and layout, created once and shared by every window of a device.
pub(super) struct BlurPipelines {
    layout: wgpu::BindGroupLayout,
    gaussian: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    copy: wgpu::RenderPipeline,
}

/// Offscreen targets and parameters of one window; freed on resize and when it closes.
pub(super) struct BlurRenderer {
    canvas: Target,
    backdrop: Target,
    low: Target,
    scratch: Target,
    blurred: Target,
    pipelines: Arc<BlurPipelines>,
    parameters: Vec<(f32, [wgpu::BindGroup; 2])>,
    downsample: u32,
}

impl BlurPipelines {
    fn new(renderer: &Renderer) -> Self {
        let device = &renderer.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zaxis blur parameters"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("zaxis Gaussian blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/blur.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zaxis blur pipeline layout"),
            bind_group_layouts: &[Some(&renderer.texture_layout), Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("zaxis blur fullscreen pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: renderer.attachment_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            layout,
            gaussian: pipeline("fs_blur"),
            down: pipeline("fs_down"),
            copy: pipeline("fs_copy"),
        }
    }
}

impl BlurRenderer {
    fn new(renderer: &Renderer) -> Self {
        let pipelines = Arc::clone(
            renderer
                .blur_pipelines
                .get_or_init(|| Arc::new(BlurPipelines::new(renderer))),
        );
        let target = || Self::target(renderer, 1);
        Self {
            canvas: target(),
            backdrop: target(),
            low: target(),
            scratch: target(),
            blurred: target(),
            pipelines,
            parameters: Vec::new(),
            downsample: 1,
        }
    }

    fn target(renderer: &Renderer, divisor: u32) -> Target {
        let device = &renderer.device;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zaxis blur attachment"),
            size: wgpu::Extent3d {
                width: renderer.config.width.div_ceil(divisor),
                height: renderer.config.height.div_ceil(divisor),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: renderer.attachment_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zaxis blur texture"),
            layout: &renderer.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                },
            ],
        });
        Target { view, group }
    }

    fn prepare(&mut self, renderer: &Renderer, data: &DrawData) {
        let radii: Vec<_> = data
            .commands
            .iter()
            .filter_map(|c| c.blur.map(|r| r * data.scale_factor))
            .collect();
        let min_sigma = radii.iter().copied().fold(f32::INFINITY, f32::min);
        let downsample = if min_sigma >= 8.0 {
            4
        } else if min_sigma >= 4.0 {
            2
        } else {
            1
        };
        if self.downsample != downsample {
            self.low = Self::target(renderer, downsample);
            self.scratch = Self::target(renderer, downsample);
            self.blurred = Self::target(renderer, downsample);
            self.downsample = downsample;
        }
        if self
            .parameters
            .iter()
            .map(|p| p.0)
            .eq(radii.iter().copied())
        {
            return;
        }
        self.parameters = radii
            .into_iter()
            .map(|sigma| {
                let samples = (3.0 * sigma / self.downsample as f32)
                    .ceil()
                    .clamp(1.0, 32.0);
                let groups = [
                    [sigma / renderer.config.width as f32, 0.0, samples, 0.0],
                    [0.0, sigma / renderer.config.height as f32, samples, 0.0],
                ]
                .map(|step| {
                    let buffer =
                        renderer
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("zaxis blur step"),
                                contents: bytemuck::cast_slice(&step),
                                usage: wgpu::BufferUsages::UNIFORM,
                            });
                    renderer
                        .device
                        .create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("zaxis blur step group"),
                            layout: &self.pipelines.layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: buffer.as_entire_binding(),
                            }],
                        })
                });
                (sigma, groups)
            })
            .collect();
    }

    fn fullscreen(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        input: &wgpu::BindGroup,
        parameters: &wgpu::BindGroup,
        pipeline: &wgpu::RenderPipeline,
        region: Option<[u32; 4]>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zaxis blur fullscreen pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        if let Some([x, y, w, h]) = region {
            pass.set_scissor_rect(x, y, w, h);
        }
        pass.set_bind_group(0, input, &[]);
        pass.set_bind_group(1, parameters, &[]);
        pass.draw(0..3, 0..1);
    }

    fn filter_region(&self, [x, y, width, height]: [u32; 4]) -> [u32; 4] {
        let d = self.downsample;
        let left = x / d;
        let top = y / d;
        // One extra texel supports bilinear sampling at the composite edges.
        let right = (x + width)
            .div_ceil(d)
            .saturating_add(1)
            .min(self.blurred.view.texture().width());
        let bottom = (y + height)
            .div_ceil(d)
            .saturating_add(1)
            .min(self.blurred.view.texture().height());
        let left = left.saturating_sub(1);
        let top = top.saturating_sub(1);
        [left, top, right - left, bottom - top]
    }

    fn segment(
        &self,
        renderer: &Renderer,
        store: &TextureStore,
        encoder: &mut wgpu::CommandEncoder,
        data: &DrawData,
        range: std::ops::Range<usize>,
        clear: Option<Color>,
    ) {
        let load = match clear {
            Some(color) => {
                let c = color.linear();
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: c[0] as f64,
                    g: c[1] as f64,
                    b: c[2] as f64,
                    a: c[3] as f64,
                })
            }
            None => wgpu::LoadOp::Load,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zaxis backdrop segment"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.canvas.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        if data.indices.is_empty() {
            return;
        }
        pass.set_bind_group(0, &renderer.viewport_group, &[]);
        pass.set_vertex_buffer(0, renderer.vertices.slice(..));
        pass.set_index_buffer(renderer.indices.slice(..), wgpu::IndexFormat::Uint32);
        for command in &data.commands[range] {
            if command.indices.is_empty() {
                continue;
            }
            let Some([x, y, w, h]) =
                scissor(command.clip_rect, data.scale_factor, renderer.physical_size)
            else {
                continue;
            };
            pass.set_scissor_rect(x, y, w, h);
            if command.blur.is_some() {
                pass.set_pipeline(&renderer.backdrop_pipeline);
                pass.set_bind_group(1, &self.blurred.group, &[]);
                pass.set_bind_group(2, &self.backdrop.group, &[]);
            } else {
                pass.set_pipeline(renderer.pipeline_for(command, data));
                pass.set_bind_group(1, &store.textures[&command.texture].bind_group, &[]);
            }
            pass.draw_indexed(command.indices.clone(), 0, 0..1);
        }
    }
}

impl Renderer {
    pub(super) fn render_blur(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        data: &DrawData,
        clear: Color,
        output: &wgpu::TextureView,
        store: &TextureStore,
    ) {
        let mut blur = self.blur.take().unwrap_or_else(|| BlurRenderer::new(self));
        blur.prepare(self, data);
        let effects: Vec<_> = data
            .commands
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.blur.map(|_| i))
            .collect();
        blur.segment(self, store, encoder, data, 0..effects[0], Some(clear));
        for (n, &index) in effects.iter().enumerate() {
            let parameters = &blur.parameters[n].1;
            let command = &data.commands[index];
            let mut min = Vec2::splat(f32::INFINITY);
            let mut max = Vec2::splat(f32::NEG_INFINITY);
            for &i in &data.indices[command.indices.start as usize..command.indices.end as usize] {
                let position = Vec2::from_array(data.vertices[i as usize].position);
                min = min.min(position);
                max = max.max(position);
            }
            let bounds = Rect::from_min_max(min, max).intersect(command.clip_rect);
            if let Some(region) = scissor(bounds, data.scale_factor, self.physical_size) {
                // Keep only the affected resolved texels before writing the next
                // segment. Coverage mixes both RGBA backgrounds at blur edges.
                let [x, y, width, height] = region;
                let origin = wgpu::Origin3d { x, y, z: 0 };
                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        origin,
                        ..blur.canvas.view.texture().as_image_copy()
                    },
                    wgpu::TexelCopyTextureInfo {
                        origin,
                        ..blur.backdrop.view.texture().as_image_copy()
                    },
                    wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
                // Horizontal samples are needed three sigma above and below the
                // composite region so the vertical pass never samples cleared pixels.
                let padding = Vec2::splat(
                    command.blur.unwrap() * 3.0 + 2.0 * blur.downsample as f32 / data.scale_factor,
                );
                let padded = Rect::from_min_max(bounds.min - padding, bounds.max + padding);
                let horizontal_region = scissor(padded, data.scale_factor, self.physical_size)
                    .map(|region| blur.filter_region(region));
                let source = if blur.downsample > 1 {
                    blur.fullscreen(
                        encoder,
                        &blur.low.view,
                        &blur.canvas.group,
                        &parameters[0],
                        &blur.pipelines.down,
                        horizontal_region,
                    );
                    &blur.low.group
                } else {
                    &blur.canvas.group
                };
                blur.fullscreen(
                    encoder,
                    &blur.scratch.view,
                    source,
                    &parameters[0],
                    &blur.pipelines.gaussian,
                    horizontal_region,
                );
                blur.fullscreen(
                    encoder,
                    &blur.blurred.view,
                    &blur.scratch.group,
                    &parameters[1],
                    &blur.pipelines.gaussian,
                    Some(blur.filter_region(region)),
                );
            }
            let end = effects.get(n + 1).copied().unwrap_or(data.commands.len());
            blur.segment(self, store, encoder, data, index..end, None);
        }
        blur.fullscreen(
            encoder,
            output,
            &blur.canvas.group,
            &blur.parameters[0].1[0],
            &blur.pipelines.copy,
            None,
        );
        self.blur = Some(blur);
    }
}
