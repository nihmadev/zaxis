//! Textures of the host's device shown by `Image`s without a copy.

use super::{
    harness::{max_diff, Host, Spec},
    live::{embedded, gpu, Noise},
    scene,
};
use wgpu::TextureFormat as F;
use zaxis::{
    vec2, Context, EmbedError, EmbedViewport, EmbeddedRenderer, ExternalAlpha, ExternalEncoding,
    ExternalImage, ExternalTexture, ExternalTextureError, ExternalTextureOptions, Image, ImageFit,
    ImageSource, Rect, RenderError, Root,
};

const SIZE: [u32; 2] = [40, 24];

fn host_texture(host: &Host, format: F, size: [u32; 2], views: &[F]) -> wgpu::Texture {
    host.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("host texture"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: views,
    })
}

/// What the host does: write bytes into its texture with its own queue.
fn write(host: &Host, texture: &wgpu::Texture, bytes: &[u8]) {
    host.queue.write_texture(
        texture.as_image_copy(),
        bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(texture.width() * 4),
            rows_per_image: None,
        },
        texture.size(),
    );
}

fn show(context: &mut Context, image: ExternalImage, target: [u32; 2]) {
    context.run(|context| {
        Root::new().padding(Default::default()).show(context, |ui| {
            let rect =
                Rect::from_min_size(vec2(0.0, 0.0), vec2(target[0] as f32, target[1] as f32));
            ui.at("external", rect, |ui| {
                ui.add(
                    Image::new(ImageSource::external(image))
                        .size(rect.size())
                        .fit(ImageFit::Stretch),
                );
            });
        });
    });
}

fn present(
    host: &Host,
    spec: &Spec,
    embed: &mut EmbeddedRenderer,
    context: &mut Context,
    image: ExternalImage,
) -> Result<Vec<[u8; 4]>, EmbedError> {
    show(context, image, spec.size);
    embed.prepare(context.draw_data(), EmbedViewport::whole(spec.size))?;
    Ok(host.in_pass(spec, false, |pass| {
        embed.record(pass).unwrap();
    }))
}

fn pixels(noise: &mut Noise) -> Vec<u8> {
    noise.bytes((SIZE[0] * SIZE[1] * 4) as usize, Some(255))
}

fn rgba(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.as_chunks::<4>().0.to_vec()
}

#[test]
fn the_host_texture_is_shown_and_its_changes_need_no_new_binding() {
    let Some(host) = gpu("external texture") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
    let mut embed = embedded(&host, &spec);
    let mut context = scene::context(SIZE, 1.0);
    let mut noise = Noise(4);
    let texture = host_texture(&host, F::Rgba8UnormSrgb, SIZE, &[]);
    let first = pixels(&mut noise);
    write(&host, &texture, &first);
    let external = embed
        .register_external_texture(&texture, Default::default())
        .unwrap();
    assert_eq!(external.size(), SIZE);
    assert!(external.id().0 >= ExternalImage::FIRST_ID);
    let shown = present(&host, &spec, &mut embed, &mut context, external.image()).unwrap();
    assert!(max_diff(&shown, &rgba(&first)) <= 1);
    let before = embed.stats();
    assert_eq!(before.external_textures, 1);
    for step in 0..5 {
        let next = pixels(&mut noise);
        write(&host, &texture, &next);
        let shown = present(&host, &spec, &mut embed, &mut context, external.image()).unwrap();
        assert!(max_diff(&shown, &rgba(&next)) <= 1, "step {step}");
    }
    let after = embed.stats();
    // The host wrote; the renderer neither uploaded, created nor bound anything.
    assert_eq!(after.texture_writes, before.texture_writes);
    assert_eq!(after.texture_creations, before.texture_creations);
    assert_eq!(
        after.texture_bind_group_creations,
        before.texture_bind_group_creations
    );
    assert_eq!(after.texture_uploads, before.texture_uploads);
    assert_eq!(
        after.image_resident_bytes_estimate, 0,
        "the host's memory is not ours"
    );
}

#[test]
fn formats_encodings_and_alpha_are_read_as_declared() {
    let Some(host) = gpu("external formats") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
    let mut noise = Noise(6);
    let bytes = noise.bytes((SIZE[0] * SIZE[1] * 4) as usize, None);
    let opaque: Vec<u8> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect();
    let swapped: Vec<u8> = opaque
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[2], p[1], p[0], 255])
        .collect();
    /// A case: name, texture format, its view formats, options, bytes written, bytes shown.
    type Case<'a> = (
        &'a str,
        F,
        &'a [F],
        ExternalTextureOptions,
        &'a [u8],
        &'a [u8],
    );
    let cases: Vec<Case<'_>> = vec![
        (
            "rgba srgb",
            F::Rgba8UnormSrgb,
            &[],
            Default::default(),
            &opaque,
            &opaque,
        ),
        (
            "bgra srgb",
            F::Bgra8UnormSrgb,
            &[],
            Default::default(),
            &swapped,
            &opaque,
        ),
        (
            "rgba unorm holding srgb",
            F::Rgba8Unorm,
            &[F::Rgba8UnormSrgb],
            Default::default(),
            &opaque,
            &opaque,
        ),
        (
            "rgba srgb holding linear",
            F::Rgba8UnormSrgb,
            &[F::Rgba8Unorm],
            ExternalTextureOptions {
                encoding: ExternalEncoding::Linear,
                ..Default::default()
            },
            &opaque,
            // Linear bytes drawn into an sRGB target come out encoded.
            &[],
        ),
        (
            "opaque alpha",
            F::Rgba8UnormSrgb,
            &[],
            ExternalTextureOptions {
                alpha: ExternalAlpha::Opaque,
                ..Default::default()
            },
            &bytes,
            &opaque,
        ),
    ];
    for (name, format, views, options, written, want) in cases {
        let mut embed = embedded(&host, &spec);
        let mut context = scene::context(SIZE, 1.0);
        let texture = host_texture(&host, format, SIZE, views);
        write(&host, &texture, written);
        let external = embed.register_external_texture(&texture, options).unwrap();
        let shown = present(&host, &spec, &mut embed, &mut context, external.image()).unwrap();
        if want.is_empty() {
            // Not the bytes: encoded, hence brighter in the middle of the range.
            assert!(max_diff(&shown, &rgba(written)) > 8, "{name}");
        } else {
            assert!(max_diff(&shown, &rgba(want)) <= 1, "{name}");
        }
    }
}

#[test]
fn what_cannot_be_shown_is_an_error_not_a_wgpu_panic() {
    let Some(host) = gpu("external errors") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
    let mut embed = embedded(&host, &spec);
    let register = |embed: &mut EmbeddedRenderer, texture: &wgpu::Texture| {
        embed.register_external_texture(texture, Default::default())
    };
    let make = |format, usage, samples, mips| {
        host.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 16,
                height: 16,
                depth_or_array_layers: 1,
            },
            mip_level_count: mips,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let binding = wgpu::TextureUsages::TEXTURE_BINDING;
    let missing = make(F::Rgba8UnormSrgb, wgpu::TextureUsages::COPY_DST, 1, 1);
    assert!(matches!(
        register(&mut embed, &missing),
        Err(ExternalTextureError::Usage { .. })
    ));
    let msaa = make(
        F::Rgba8UnormSrgb,
        binding | wgpu::TextureUsages::RENDER_ATTACHMENT,
        4,
        1,
    );
    assert!(matches!(
        register(&mut embed, &msaa),
        Err(ExternalTextureError::Shape(_))
    ));
    let mips = make(F::Rgba8UnormSrgb, binding, 1, 2);
    assert!(matches!(
        register(&mut embed, &mips),
        Err(ExternalTextureError::Shape(_))
    ));
    let gray = make(F::R8Unorm, binding, 1, 1);
    assert!(matches!(
        register(&mut embed, &gray),
        Err(ExternalTextureError::Format(_))
    ));
    // A view format the texture was never created with.
    let plain = make(F::Rgba8Unorm, binding, 1, 1);
    let error = register(&mut embed, &plain).unwrap_err();
    assert!(matches!(error, ExternalTextureError::View(_)), "{error}");
    // A texture of another device.
    let (other, _) = pollster::block_on(host.adapter.request_device(&Default::default())).unwrap();
    let foreign = other.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: F::Rgba8UnormSrgb,
        usage: binding,
        view_formats: &[],
    });
    let error = register(&mut embed, &foreign).unwrap_err();
    assert!(
        matches!(error, ExternalTextureError::DifferentDevice),
        "{error}"
    );
    assert!(!error.to_string().is_empty());
    assert_eq!(embed.stats().external_textures, 0);
    // The renderer is untouched and still registers a good one.
    let good = host_texture(&host, F::Rgba8UnormSrgb, SIZE, &[]);
    assert!(register(&mut embed, &good).is_ok());
    embed.notify_device_lost("test");
    assert!(matches!(
        register(&mut embed, &good),
        Err(ExternalTextureError::DeviceLost(_))
    ));
}

#[test]
fn unregistering_dropping_and_re_registering() {
    let Some(host) = gpu("external lifecycle") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
    let mut embed = embedded(&host, &spec);
    let mut context = scene::context(SIZE, 1.0);
    let mut noise = Noise(9);
    let texture = host_texture(&host, F::Rgba8UnormSrgb, SIZE, &[]);
    let bytes = pixels(&mut noise);
    write(&host, &texture, &bytes);
    let external = embed
        .register_external_texture(&texture, Default::default())
        .unwrap();
    let image = external.image();
    present(&host, &spec, &mut embed, &mut context, image).unwrap();
    external.unregister();
    let error = present(&host, &spec, &mut embed, &mut context, image).unwrap_err();
    let EmbedError::Render(RenderError::Texture(message)) = &error else {
        panic!("{error:?}");
    };
    assert!(message.contains("not registered"), "{message}");
    assert_eq!(
        embed.stats().external_textures,
        1,
        "counted at the last successful prepare"
    );
    // The frame the host's texture belongs to is still intact for the host.
    write(&host, &texture, &bytes);
    // Registered again, with a larger texture: the old ExternalImage is refused by size, the
    // new one shows the new texture.
    let bigger = host_texture(&host, F::Rgba8UnormSrgb, [SIZE[0] * 2, SIZE[1]], &[]);
    let again = embed
        .register_external_texture(&bigger, Default::default())
        .unwrap();
    assert_ne!(again.id(), image.id);
    let stale = ExternalImage {
        id: again.id(),
        ..image
    };
    let error = present(&host, &spec, &mut embed, &mut context, stale).unwrap_err();
    assert!(error.to_string().contains("register it again"), "{error}");
    let wide = Spec::new(F::Rgba8UnormSrgb, [SIZE[0] * 2, SIZE[1]]);
    let mut wide_context = scene::context(wide.size, 1.0);
    let mut wide_embed = embedded(&host, &wide);
    let wide_external = wide_embed
        .register_external_texture(&bigger, Default::default())
        .unwrap();
    present(
        &host,
        &wide,
        &mut wide_embed,
        &mut wide_context,
        wide_external.image(),
    )
    .unwrap();
    // A handle outliving its renderer is harmless.
    drop(wide_embed);
    drop(wide_external);
    drop(again);
    // A renderer with a store of its own does not know another's registrations.
    let mut other = embedded(&host, &spec);
    let stranger = embed_register(&host, &texture);
    let error = present(&host, &spec, &mut other, &mut context, stranger.1.image()).unwrap_err();
    assert!(
        matches!(error, EmbedError::Render(RenderError::Texture(_))),
        "{error}"
    );
}

fn embed_register(host: &Host, texture: &wgpu::Texture) -> (EmbeddedRenderer, ExternalTexture) {
    let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
    let mut embed = embedded(host, &spec);
    let external = embed
        .register_external_texture(texture, Default::default())
        .unwrap();
    (embed, external)
}

#[test]
fn siblings_share_registrations_and_windows_share_the_texture() {
    let Some(host) = gpu("external siblings") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
    let resources = zaxis::SharedResources::new();
    let mut a = scene::sharing(&resources, SIZE, 1.0);
    let mut b = scene::sharing(&resources, SIZE, 1.0);
    let mut first = embedded(&host, &spec);
    let mut second = first.create_sibling(spec.options()).unwrap();
    let texture = host_texture(&host, F::Rgba8UnormSrgb, SIZE, &[]);
    let mut noise = Noise(15);
    let external = first
        .register_external_texture(&texture, Default::default())
        .unwrap();
    for step in 0..3 {
        let bytes = pixels(&mut noise);
        write(&host, &texture, &bytes);
        let one = present(&host, &spec, &mut first, &mut a, external.image()).unwrap();
        let two = present(&host, &spec, &mut second, &mut b, external.image()).unwrap();
        assert!(max_diff(&one, &rgba(&bytes)) <= 1, "step {step}");
        assert_eq!(max_diff(&one, &two), 0, "step {step}");
    }
    assert_eq!(second.stats().texture_bind_group_creations, 0);
    assert_eq!(second.stats().external_textures, 1);
    // Another window's whole texture store is cleared and replaced by another producer's
    // frames: the registration stays.
    first.clear_image_textures();
    let again = present(&host, &spec, &mut first, &mut a, external.image()).unwrap();
    assert_eq!(max_diff(&again, &rgba(&pixels_of(&host, &texture))), 0);
}

fn pixels_of(host: &Host, texture: &wgpu::Texture) -> Vec<u8> {
    let row = (texture.width() * 4).next_multiple_of(256);
    let buffer = host.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * texture.height()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = host.device.create_command_encoder(&Default::default());
    let mut copy = texture.as_image_copy();
    copy.texture = texture;
    encoder.copy_texture_to_buffer(
        copy,
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    host.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    host.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    rx.recv().unwrap().unwrap();
    let data = buffer.slice(..).get_mapped_range().unwrap();
    (0..texture.height() as usize)
        .flat_map(|y| data[y * row as usize..][..texture.width() as usize * 4].to_vec())
        .collect()
}
