//! `--smoke-test`: the page drives a context and an embedded renderer on a device of its own;
//! what the renderer shows must be the page, step after step, with one texture and one bind
//! group from the first frame to the last.
use super::page::{Page, SIZE};
use zaxis::{
    vec2, Context, EmbedOptions, EmbeddedRenderer, ExternalAlpha, ExternalTextureOptions, Image,
    ImageFit, ImageSource, Rect, Root,
};

pub fn run() -> Result<(), String> {
    scene(false)?;
    scene(true)
}

/// One scene: the page reaches the image by regions (`external == false`) or through a texture
/// of the host's own that it writes itself and registers with the renderer (`true`).
fn scene(external: bool) -> Result<(), String> {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        println!("live_image smoke: SKIPPED, no adapter");
        return Ok(());
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .map_err(|e| e.to_string())?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut embed = EmbeddedRenderer::new_with_adapter(
        device.clone(),
        queue.clone(),
        &adapter,
        EmbedOptions::new(format),
    )
    .map_err(|e| e.to_string())?;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live_image target"),
        size: wgpu::Extent3d {
            width: SIZE[0],
            height: SIZE[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[format.add_srgb_suffix()],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor {
        format: Some(format.add_srgb_suffix()),
        ..Default::default()
    });
    let mut context = Context::new();
    context.set_viewport(zaxis::winit::dpi::PhysicalSize::new(SIZE[0], SIZE[1]), 1.0);
    let mut page = Page::new();
    // The host's own texture, in the layout the page has, written with the host's queue.
    let host_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live_image host texture"),
        size: wgpu::Extent3d {
            width: SIZE[0],
            height: SIZE[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let write_page = |page: &Page| {
        queue.write_texture(
            host_texture.as_image_copy(),
            &page.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE[0] * 4),
                rows_per_image: None,
            },
            host_texture.size(),
        );
    };
    let registration = if external {
        write_page(&page);
        Some(
            embed
                .register_external_texture(
                    &host_texture,
                    ExternalTextureOptions {
                        alpha: ExternalAlpha::Opaque,
                        ..Default::default()
                    },
                )
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    let source = match &registration {
        Some(texture) => ImageSource::external(texture.image()),
        None => ImageSource::bgra_opaque(SIZE, page.pixels.clone()),
    };
    let handle = context.load_image(source).map_err(|e| e.to_string())?;
    let registered = embed.stats();
    let mut changed = 0u64;
    for step in 0..=60 {
        if step > 0 {
            for region in page.step() {
                if external {
                    continue;
                }
                let (rows, stride) = page.rows(region);
                context
                    .update_image_region(handle, region, rows, stride)
                    .map_err(|e| e.to_string())?;
                changed += region.area() * 4;
            }
            if external {
                write_page(&page);
                context.request_repaint();
            }
        }
        context.run(|context| {
            Root::new().padding(Default::default()).show(context, |ui| {
                let rect =
                    Rect::from_min_size(vec2(0.0, 0.0), vec2(SIZE[0] as f32, SIZE[1] as f32));
                ui.at("page", rect, |ui| {
                    ui.add(Image::new(handle).size(rect.size()).fit(ImageFit::Stretch));
                });
            });
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        embed
            .render_to(
                &mut encoder,
                &view,
                None,
                zaxis::EmbedLoad::Clear(zaxis::Color::BLACK),
                context.draw_data(),
            )
            .map_err(|e| e.to_string())?;
        queue.submit([encoder.finish()]);
        if step % 10 == 0 {
            check(&device, &queue, &target, &page, step)?;
        }
    }
    let stats = embed.stats();
    if external {
        // The host wrote; the renderer made and uploaded nothing after the registration.
        if stats.texture_writes != 0
            || stats.texture_creations != registered.texture_creations
            || stats.texture_bind_group_creations != registered.texture_bind_group_creations
            || stats.external_textures != 1
        {
            return Err(format!("the renderer touched a host texture: {stats:?}"));
        }
        println!("live_image smoke (external texture): 61 frames match the page, nothing uploaded by the renderer");
        return Ok(());
    }
    // The glyph atlas does not appear in this scene: one image texture, made once.
    if stats.texture_creations != 1 || stats.texture_bind_group_creations != 1 {
        return Err(format!("textures or bind groups were recreated: {stats:?}"));
    }
    if stats.texture_region_updates != 60 || stats.texture_update_fallbacks != 0 {
        return Err(format!("not every step was a region update: {stats:?}"));
    }
    let frame = u64::from(SIZE[0] * SIZE[1] * 4);
    if stats.texture_region_bytes >= 60 * frame / 4 {
        return Err(format!(
            "{} bytes written for {changed} changed",
            stats.texture_region_bytes
        ));
    }
    println!(
        "live_image smoke: 61 frames match the page; {} bytes written in {} regions (a frame is {frame})",
        stats.texture_region_bytes, stats.texture_region_writes
    );
    Ok(())
}

fn check(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target: &wgpu::Texture,
    page: &Page,
    step: u32,
) -> Result<(), String> {
    let row = (SIZE[0] * 4).next_multiple_of(256);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * SIZE[1]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(SIZE[1]),
            },
        },
        target.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| e.to_string())?;
    rx.recv().unwrap().map_err(|e| e.to_string())?;
    let data = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|e| e.to_string())?;
    for y in 0..SIZE[1] as usize {
        for x in 0..SIZE[0] as usize {
            let at = y * row as usize + x * 4;
            let p = (y * SIZE[0] as usize + x) * 4;
            // The page is BGRA; the target reads back as RGBA.
            let want = [page.pixels[p + 2], page.pixels[p + 1], page.pixels[p]];
            let got = &data[at..at + 3];
            if got.iter().zip(want).any(|(a, b)| a.abs_diff(b) > 1) {
                return Err(format!(
                    "step {step}: pixel ({x}, {y}) is {got:?}, the page has {want:?}"
                ));
            }
        }
    }
    Ok(())
}
