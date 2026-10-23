//! Creating textures and writing payload bytes into them.
use super::*;

/// Rows converted and written at once when BGRA has to become RGBA.
const BAND_BYTES: usize = 1 << 20;

pub(super) fn create_texture_profiled(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    size: [u32; 2],
    pixels: &[u8],
    revision: u64,
    texels: Texels,
    diagnostics: &mut crate::renderer::diagnostics::Diagnostics,
) -> GpuTexture {
    let start = diagnostics.start();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("zaxis image texture"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: texels.gpu_format(),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    diagnostics.end(RendererStage::CreateTextureCpu, start);
    let start = diagnostics.start();
    write_texture(queue, &texture, size, pixels, texels.convert);
    diagnostics.end(RendererStage::WriteTextureCpu, start);
    let bind_group = binding(device, layout, sampler, &texture, diagnostics);
    GpuTexture {
        texture,
        bind_group,
        size,
        texels,
        revision,
        filter: TextureFilter::Linear,
        managed: false,
        last_used: 0,
        allocation: 0,
        pixels: Weak::new(),
        external: false,
    }
}

/// Write the complete payload; with `convert`, in bands of swapped rows.
pub(crate) fn write_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: [u32; 2],
    pixels: &[u8],
    convert: bool,
) {
    if !convert {
        write_rows(
            queue,
            texture,
            [0, 0],
            size,
            pixels,
            0,
            size[0] as usize * 4,
        );
        return;
    }
    let row = size[0] as usize * 4;
    let rows = (BAND_BYTES / row.max(1)).clamp(1, size[1] as usize);
    let mut scratch = Vec::with_capacity(rows * row);
    for (band, chunk) in pixels.chunks(rows * row).enumerate() {
        swap_red_blue(chunk, &mut scratch);
        let y = (band * rows) as u32;
        let height = (chunk.len() / row) as u32;
        write_rows(queue, texture, [0, y], [size[0], height], &scratch, 0, row);
    }
}

/// Write `region` of a payload of `size` whose rows are `size[0] * 4` bytes apart. Only
/// the region's rows are read from `pixels`.
pub(super) fn write_region(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: [u32; 2],
    pixels: &[u8],
    region: crate::TextureRegion,
    convert: bool,
) {
    let stride = size[0] as usize * 4;
    let first = region.y as usize * stride + region.x as usize * 4;
    let extent = [region.width, region.height];
    let origin = [region.x, region.y];
    if !convert {
        write_rows(queue, texture, origin, extent, pixels, first, stride);
        return;
    }
    let row = region.width as usize * 4;
    let mut scratch = Vec::with_capacity(row * region.height as usize);
    for y in 0..region.height as usize {
        let at = first + y * stride;
        let start = scratch.len();
        scratch.extend_from_slice(&pixels[at..at + row]);
        swap_in_place(&mut scratch[start..]);
    }
    write_rows(queue, texture, origin, extent, &scratch, 0, row);
}

fn write_rows(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    origin: [u32; 2],
    extent: [u32; 2],
    data: &[u8],
    offset: usize,
    stride: usize,
) {
    queue.write_texture(
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

fn swap_red_blue(src: &[u8], dst: &mut Vec<u8>) {
    dst.clear();
    dst.extend_from_slice(src);
    swap_in_place(dst);
}

fn swap_in_place(bytes: &mut [u8]) {
    for texel in bytes.as_chunks_mut::<4>().0 {
        texel.swap(0, 2);
    }
}
