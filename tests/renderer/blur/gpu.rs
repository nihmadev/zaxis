//! A headless device with the library's own bind group layouts, canvas upload and readback.
//! Skipped, with a message, when there is no adapter or `ZAXIS_SKIP_GPU_TESTS` is set.

use zaxis::renderer::{textures, viewport};

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub struct Gpu {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub viewport_layout: wgpu::BindGroupLayout,
    pub uniform: wgpu::Buffer,
    pub viewport_group: wgpu::BindGroup,
    pub texture_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

pub fn gpu(name: &str) -> Option<Gpu> {
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
    let (viewport_layout, uniform, viewport_group) = viewport::create_bindings(&device);
    let (texture_layout, sampler) = textures::create_bindings(&device);
    Some(Gpu {
        adapter,
        device,
        queue,
        viewport_layout,
        uniform,
        viewport_group,
        texture_layout,
        sampler,
    })
}

impl Gpu {
    pub fn texture(&self, [width, height]: [u32; 2], format: wgpu::TextureFormat) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("blur test texture"),
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
        })
    }

    /// Upload RGBA8 texels (row-major) into a new canvas texture.
    pub fn canvas(&self, size: [u32; 2], pixels: &[[u8; 4]]) -> wgpu::Texture {
        let texture = self.texture(size, FORMAT);
        self.queue.write_texture(
            texture.as_image_copy(),
            pixels.as_flattened(),
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
        texture
    }

    /// A bind group sampling `view` with the library's clamping linear sampler.
    pub fn group(&self, view: &wgpu::TextureView) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur test texture group"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Read a 4-byte-per-texel texture back as RGBA8 rows.
    pub fn read(&self, texture: &wgpu::Texture) -> Vec<[u8; 4]> {
        let (width, height) = (texture.width(), texture.height());
        let padded = (width * 4).next_multiple_of(256);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur test readback"),
            size: u64::from(padded * height),
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
                    bytes_per_row: Some(padded),
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
        let bytes = buffer.slice(..).get_mapped_range().unwrap();
        let mut out = Vec::with_capacity((width * height) as usize);
        for row in 0..height as usize {
            let line = &bytes[row * padded as usize..][..width as usize * 4];
            out.extend(line.as_chunks::<4>().0.iter().copied());
        }
        out
    }
}
