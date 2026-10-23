//! The headless device the rows draw on: its target textures, presenting a window and
//! reading pixels back.
use super::{Mode, Window, FORMAT};
use crate::BenchResult;
use zaxis::{EmbedLoad, EmbedViewport};

pub struct Host {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
}

pub fn target(host: &Host, size: [u32; 2]) -> wgpu::Texture {
    host.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live bench target"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[FORMAT.add_srgb_suffix()],
    })
}

pub fn present(
    host: &Host,
    window: &mut Window,
    size: [u32; 2],
    mode: Mode,
    wait: bool,
) -> BenchResult<()> {
    let (Some(renderer), Some(target)) = (&mut window.renderer, &window.target) else {
        return Ok(());
    };
    let view = target.create_view(&wgpu::TextureViewDescriptor {
        format: Some(FORMAT.add_srgb_suffix()),
        ..Default::default()
    });
    let data = window.ctx.draw_data();
    let mut encoder = host.device.create_command_encoder(&Default::default());
    if mode == Mode::Pass {
        renderer.prepare(data, EmbedViewport::whole(size))?;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live bench pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        renderer.record(&mut pass)?;
    } else {
        renderer.render_to(
            &mut encoder,
            &view,
            None,
            EmbedLoad::Clear(zaxis::Color::BLACK),
            data,
        )?;
    }
    host.queue.submit([encoder.finish()]);
    // Without `--gpu-wait` the queue is only kept from growing; with it each frame completes
    // before the next starts, so a slow GPU cannot hide in the next frame's submit.
    host.device.poll(if wait {
        wgpu::PollType::wait_indefinitely()
    } else {
        wgpu::PollType::Poll
    })?;
    Ok(())
}

pub fn read(host: &Host, texture: &wgpu::Texture, [width, height]: [u32; 2]) -> Vec<[u8; 4]> {
    let row = (width * 4).next_multiple_of(256);
    let buffer = host.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = host.device.create_command_encoder(&Default::default());
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
        texture.size(),
    );
    host.queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
    host.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let data = buffer.slice(..).get_mapped_range().unwrap();
    (0..height)
        .flat_map(|y| {
            data[(y * row) as usize..][..(width * 4) as usize]
                .as_chunks::<4>()
                .0
                .to_vec()
        })
        .collect()
}

/// What a host does with its own texture: write `rows` of `data` (from `offset`, `stride`
/// bytes apart) into the rectangle at `origin`.
pub fn write_texture(
    host: &Host,
    texture: &wgpu::Texture,
    origin: [u32; 2],
    extent: [u32; 2],
    data: &[u8],
    offset: usize,
    stride: usize,
) {
    host.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: origin[0],
                y: origin[1],
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: offset as u64,
            bytes_per_row: Some(stride as u32),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: extent[0],
            height: extent[1],
            depth_or_array_layers: 1,
        },
    );
}
