//! A host application in miniature: its own device, a target it draws a scene into, and
//! readback of the result as sRGB bytes whatever the format.

use wgpu::util::DeviceExt;
use zaxis::{EmbedOptions, PhysicalRect};

pub const CLEAR: [f64; 4] = [0.05, 0.08, 0.2, 1.0];

pub struct Host {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

pub fn host(name: &str) -> Option<Host> {
    if std::env::var_os("ZAXIS_SKIP_GPU_TESTS").is_some() {
        eprintln!("SKIPPED {name}: ZAXIS_SKIP_GPU_TESTS");
        return None;
    }
    let adapter =
        pollster::block_on(wgpu::Instance::default().request_adapter(&Default::default()))
            .inspect_err(|_| eprintln!("SKIPPED {name}: no adapter"))
            .ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .inspect_err(|_| eprintln!("SKIPPED {name}: no device"))
        .ok()?;
    Some(Host {
        adapter,
        device,
        queue,
    })
}

/// The pass a host draws in.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// Format of the texture; the pass renders through its sRGB view where there is one.
    pub format: wgpu::TextureFormat,
    pub samples: u32,
    pub depth: bool,
    pub size: [u32; 2],
}

impl Spec {
    pub fn new(format: wgpu::TextureFormat, size: [u32; 2]) -> Self {
        Self {
            format,
            samples: 1,
            depth: false,
            size,
        }
    }

    pub fn options(&self) -> EmbedOptions {
        let options = EmbedOptions::new(self.format).sample_count(self.samples);
        if self.depth {
            options.depth_format(DEPTH)
        } else {
            options
        }
    }

    pub fn view_format(&self) -> wgpu::TextureFormat {
        self.options().attachment_format()
    }
}

pub const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

/// The attachments of one host frame.
pub struct Target {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub msaa: Option<wgpu::TextureView>,
    pub depth: Option<wgpu::TextureView>,
}

impl Host {
    pub fn supported(&self, spec: &Spec) -> bool {
        let features = self.adapter.get_texture_format_features(spec.view_format());
        features
            .allowed_usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
            && features.flags.sample_count_supported(spec.samples)
            && features
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::BLENDABLE)
    }

    pub fn texture(&self, spec: &Spec, usage: wgpu::TextureUsages, samples: u32) -> wgpu::Texture {
        let view = spec.view_format();
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("host target"),
            size: wgpu::Extent3d {
                width: spec.size[0],
                height: spec.size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: spec.format,
            usage,
            view_formats: if view == spec.format {
                &[]
            } else {
                std::slice::from_ref(&view)
            },
        })
    }

    pub fn target(&self, spec: &Spec, extra: wgpu::TextureUsages) -> Target {
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING
            | extra;
        let view_of = |texture: &wgpu::Texture| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(spec.view_format()),
                ..Default::default()
            })
        };
        let texture = self.texture(spec, usage, 1);
        let view = view_of(&texture);
        let msaa = (spec.samples > 1).then(|| {
            view_of(&self.texture(spec, wgpu::TextureUsages::RENDER_ATTACHMENT, spec.samples))
        });
        let depth = spec.depth.then(|| {
            self.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("host depth"),
                    size: wgpu::Extent3d {
                        width: spec.size[0],
                        height: spec.size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: spec.samples,
                    dimension: wgpu::TextureDimension::D2,
                    format: DEPTH,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        });
        Target {
            texture,
            view,
            msaa,
            depth,
        }
    }

    /// One host pass: clear, the host's scene if asked, then `draw`. Returns the pixels.
    pub fn in_pass(
        &self,
        spec: &Spec,
        scene: bool,
        draw: impl FnOnce(&mut wgpu::RenderPass<'_>),
    ) -> Vec<[u8; 4]> {
        let target = self.target(spec, wgpu::TextureUsages::empty());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = self.begin(&mut encoder, &target, spec, true);
            if scene {
                super::scene::host_scene(self, &mut pass, spec);
            }
            draw(&mut pass);
        }
        self.queue.submit([encoder.finish()]);
        self.read(&target.texture, spec)
    }

    /// A host frame whose scene pass is over; `then` encodes more into the same encoder.
    pub fn frame(
        &self,
        spec: &Spec,
        scene: bool,
        extra: wgpu::TextureUsages,
        then: impl FnOnce(&mut wgpu::CommandEncoder, &Target),
    ) -> Vec<[u8; 4]> {
        let target = self.target(spec, extra);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = self.begin(&mut encoder, &target, spec, true);
            if scene {
                super::scene::host_scene(self, &mut pass, spec);
            }
        }
        then(&mut encoder, &target);
        self.queue.submit([encoder.finish()]);
        self.read(&target.texture, spec)
    }

    pub fn begin<'a>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
        target: &'a Target,
        spec: &Spec,
        clear: bool,
    ) -> wgpu::RenderPass<'a> {
        let load = if clear {
            wgpu::LoadOp::Clear(wgpu::Color {
                r: CLEAR[0],
                g: CLEAR[1],
                b: CLEAR[2],
                a: CLEAR[3],
            })
        } else {
            wgpu::LoadOp::Load
        };
        let (view, resolve) = match &target.msaa {
            Some(msaa) => (msaa, Some(&target.view)),
            None => (&target.view, None),
        };
        let _ = spec;
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("host pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: target.depth.as_ref().map(|view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Discard,
                    }),
                }
            }),
            ..Default::default()
        })
    }

    /// The texture as sRGB-encoded RGBA bytes, row-major.
    pub fn read(&self, texture: &wgpu::Texture, spec: &Spec) -> Vec<[u8; 4]> {
        let [width, height] = spec.size;
        let bytes_per_pixel = spec.format.block_copy_size(None).unwrap();
        let row = (width * bytes_per_pixel).next_multiple_of(256);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        let data = buffer.slice(..).get_mapped_range().unwrap();
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            let line = &data[(y * row) as usize..][..(width * bytes_per_pixel) as usize];
            for x in 0..width as usize {
                pixels.push(decode(spec.format, &line[x * bytes_per_pixel as usize..]));
            }
        }
        pixels
    }
}

fn decode(format: wgpu::TextureFormat, texel: &[u8]) -> [u8; 4] {
    use wgpu::TextureFormat as F;
    match format {
        F::Rgba8Unorm | F::Rgba8UnormSrgb => [texel[0], texel[1], texel[2], texel[3]],
        F::Bgra8Unorm | F::Bgra8UnormSrgb => [texel[2], texel[1], texel[0], texel[3]],
        F::Rgba16Float => {
            let half = |i: usize| f16(u16::from_le_bytes([texel[i * 2], texel[i * 2 + 1]]));
            let encode = |linear: f32| {
                let c = linear.clamp(0.0, 1.0);
                let s = if c <= 0.003_130_8 {
                    c * 12.92
                } else {
                    1.055 * c.powf(1.0 / 2.4) - 0.055
                };
                (s * 255.0).round() as u8
            };
            [
                encode(half(0)),
                encode(half(1)),
                encode(half(2)),
                (half(3).clamp(0.0, 1.0) * 255.0).round() as u8,
            ]
        }
        other => panic!("unsupported readback {other:?}"),
    }
}

fn f16(bits: u16) -> f32 {
    let sign = if bits >> 15 == 1 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f32::from(bits & 0x3ff);
    match exponent {
        0 => sign * fraction * 2f32.powi(-24),
        31 => sign * f32::INFINITY,
        e => sign * (1.0 + fraction / 1024.0) * 2f32.powi(e - 15),
    }
}

/// The largest per-channel difference of two images.
pub fn max_diff(a: &[[u8; 4]], b: &[[u8; 4]]) -> u8 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)))
        .max()
        .unwrap_or(0)
}

pub fn pixel(image: &[[u8; 4]], width: u32, x: u32, y: u32) -> [u8; 4] {
    image[(y * width + x) as usize]
}

/// Whether `rect` of the image is identical in both.
pub fn same_outside(a: &[[u8; 4]], b: &[[u8; 4]], width: u32, rect: PhysicalRect) -> bool {
    a.iter().zip(b).enumerate().all(|(i, (a, b))| {
        let (x, y) = (i as u32 % width, i as u32 / width);
        let inside =
            x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height;
        inside || a == b
    })
}

#[allow(dead_code)]
pub fn buffer_init(host: &Host, contents: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    host.device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents,
            usage,
        })
}
