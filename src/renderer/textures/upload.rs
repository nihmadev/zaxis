use super::*;
pub(super) fn create_texture_profiled(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    size: [u32; 2],
    pixels: &[u8],
    revision: u64,
    diagnostics: &mut crate::renderer::diagnostics::Diagnostics,
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
pub(crate) fn write_texture(
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
