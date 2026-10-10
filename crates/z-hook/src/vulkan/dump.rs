//! Writes one composited frame to a PNG, to see what the overlay drew without a screenshot
//! tool: `ZAXIS_HOOK_DUMP=/path/frame.png`, and `ZAXIS_HOOK_DUMP_FRAME=N` (default 60) for
//! which overlay frame, or `ZAXIS_HOOK_DUMP_EVERY=1` to rewrite the file with every frame from
//! that one on (the file then holds the latest). Diagnostic: it waits for the GPU.

use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering::Relaxed},
};
use zaxis::wgpu;

static FRAMES: AtomicU64 = AtomicU64::new(0);

pub(super) struct Request {
    path: PathBuf,
    bytes_per_row: u32,
    buffer: wgpu::Buffer,
    size: [u32; 2],
    bgra: bool,
}

/// Count the frame and, if it is the one to dump, prepare the readback buffer.
pub(super) fn request(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: [u32; 2],
) -> Option<Request> {
    let path = std::env::var_os("ZAXIS_HOOK_DUMP")?;
    let frame = FRAMES.fetch_add(1, Relaxed);
    let wanted = std::env::var("ZAXIS_HOOK_DUMP_FRAME")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    let every = std::env::var_os("ZAXIS_HOOK_DUMP_EVERY").is_some();
    if frame != wanted && !(every && frame > wanted) {
        return None;
    }
    let bgra = match format {
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => true,
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => false,
        other => {
            log::warn!("z-hook: cannot dump a {other:?} frame");
            return None;
        }
    };
    let bytes_per_row = (size[0] * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("z-hook dump"),
        size: u64::from(bytes_per_row) * u64::from(size[1]),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    Some(Request {
        path: path.into(),
        bytes_per_row,
        buffer,
        size,
        bgra,
    })
}

impl Request {
    pub fn copy(&self, encoder: &mut wgpu::CommandEncoder, texture: &wgpu::Texture) {
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.bytes_per_row),
                    rows_per_image: Some(self.size[1]),
                },
            },
            wgpu::Extent3d {
                width: self.size[0],
                height: self.size[1],
                depth_or_array_layers: 1,
            },
        );
    }

    /// After the submit: wait, read and write the file.
    pub fn write(self, device: &wgpu::Device) {
        let slice = self.buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let Ok(data) = slice.get_mapped_range() else {
            log::warn!("z-hook: the frame dump buffer did not map");
            return;
        };
        let [width, height] = self.size;
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height as usize {
            let start = row * self.bytes_per_row as usize;
            for px in data[start..start + width as usize * 4]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|px| &px[..])
            {
                if self.bgra {
                    pixels.extend_from_slice(&[px[2], px[1], px[0], 255]);
                } else {
                    pixels.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
        }
        drop(data);
        match image::RgbaImage::from_raw(width, height, pixels) {
            Some(image) => {
                // Written beside and renamed, so a reader never sees half a file.
                let partial = self.path.with_extension("partial.png");
                let written = image
                    .save(&partial)
                    .and_then(|()| std::fs::rename(&partial, &self.path).map_err(Into::into));
                match written {
                    Ok(()) => log::debug!("z-hook: wrote {}", self.path.display()),
                    Err(error) => {
                        log::warn!("z-hook: cannot write {}: {error}", self.path.display())
                    }
                }
            }
            None => log::warn!("z-hook: frame dump has the wrong size"),
        }
    }
}
