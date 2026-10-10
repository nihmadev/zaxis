//! Errors instead of panics, device loss, a changed target and resizing.

use super::{
    harness::{host, max_diff, Host, Spec},
    scene,
};
use wgpu::TextureFormat as F;
use zaxis::{
    EmbedError, EmbedLoad, EmbedOptions, EmbedViewport, EmbeddedRenderer, PhysicalRect, Rect,
};

const SIZE: [u32; 2] = [320, 224];

fn make(host: &Host, spec: &Spec) -> EmbeddedRenderer {
    EmbeddedRenderer::new_with_adapter(
        host.device.clone(),
        host.queue.clone(),
        &host.adapter,
        spec.options(),
    )
    .unwrap()
}

fn panel() -> Rect {
    Rect::from_min_size(zaxis::vec2(30.0, 30.0), zaxis::vec2(160.0, 120.0))
}

#[test]
fn options_the_pass_cannot_use_are_refused_up_front() {
    let Some(host) = host("embedded options") else {
        return;
    };
    let bad = [
        EmbedOptions::new(F::R8Unorm),
        EmbedOptions::new(F::Depth32Float),
        EmbedOptions::new(F::Rgba8UnormSrgb).sample_count(3),
        EmbedOptions::new(F::Rgba8UnormSrgb).depth_format(F::Rgba8Unorm),
    ];
    for options in bad {
        let result = EmbeddedRenderer::new(host.device.clone(), host.queue.clone(), options);
        assert!(
            matches!(result, Err(EmbedError::InvalidOptions(_))),
            "{options:?}"
        );
    }
}

#[test]
fn mistakes_come_back_as_errors_that_say_what_to_change() {
    let Some(host) = host("embedded errors") else {
        return;
    };
    let spec = Spec::new(F::Bgra8UnormSrgb, SIZE);
    let data = scene::controls(SIZE, 1.0);
    let mut embed = make(&host, &spec);
    host.in_pass(&spec, false, |pass| {
        assert!(matches!(embed.record(pass), Err(EmbedError::NotPrepared)));
    });
    let outside = EmbedViewport::region(SIZE, PhysicalRect::new(100, 100, 320, 224));
    assert!(matches!(
        embed.prepare(&data, outside),
        Err(EmbedError::InvalidViewport(_))
    ));
    let wrong_size = EmbedViewport::region([400, 300], PhysicalRect::new(0, 0, 400, 300));
    let message = embed.prepare(&data, wrong_size).unwrap_err().to_string();
    assert!(message.contains("320.0x224.0"), "{message}");
    // render_to: a texture that is not the described format, or multisampled.
    let blur = scene::blur_panel(SIZE, 1.0, panel(), 8.0);
    let mut encoder = host.device.create_command_encoder(&Default::default());
    let other = Spec::new(F::Rgba16Float, SIZE);
    let target = host.target(&other, wgpu::TextureUsages::empty());
    let error = embed
        .render_to(&mut encoder, &target.view, None, EmbedLoad::Keep, &blur)
        .unwrap_err();
    assert!(matches!(error, EmbedError::TargetMismatch(_)), "{error}");
    let msaa = Spec { samples: 4, ..spec };
    if host.supported(&msaa) {
        let texture = host.texture(&msaa, wgpu::TextureUsages::RENDER_ATTACHMENT, 4);
        let view = texture.create_view(&Default::default());
        let error = embed
            .render_to(&mut encoder, &view, None, EmbedLoad::Keep, &blur)
            .unwrap_err();
        assert!(matches!(error, EmbedError::TargetMismatch(_)), "{error}");
    }
    // A backdrop effect over the host's pixels needs to copy them.
    let write_only = host.texture(&spec, wgpu::TextureUsages::RENDER_ATTACHMENT, 1);
    let view = write_only.create_view(&wgpu::TextureViewDescriptor {
        format: Some(spec.view_format()),
        ..Default::default()
    });
    let error = embed
        .render_to(&mut encoder, &view, None, EmbedLoad::Keep, &blur)
        .unwrap_err();
    match error {
        EmbedError::TargetUsage { missing, .. } => {
            assert_eq!(missing, wgpu::TextureUsages::COPY_SRC)
        }
        other => panic!("{other}"),
    }
    // Without Keep nothing is read back, and a frame without blur never needs it.
    embed
        .render_to(
            &mut encoder,
            &view,
            None,
            EmbedLoad::Clear(zaxis::Color::BLACK),
            &blur,
        )
        .unwrap();
    embed
        .render_to(&mut encoder, &view, None, EmbedLoad::Keep, &data)
        .unwrap();
}

#[test]
fn a_lost_device_stops_every_renderer_on_it_and_a_new_one_recovers() {
    let Some(host) = host("embedded device loss") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
    let data = scene::controls(SIZE, 1.0);
    let mut embed = make(&host, &spec);
    let mut sibling = embed.create_sibling(spec.options()).unwrap();
    embed.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
    assert!(!embed.is_device_lost());
    embed.notify_device_lost("driver reset");
    assert!(sibling.is_device_lost());
    let error = sibling
        .prepare(&data, EmbedViewport::whole(SIZE))
        .unwrap_err();
    assert!(
        matches!(&error, EmbedError::DeviceLost(m) if m == "driver reset"),
        "{error}"
    );
    host.in_pass(&spec, false, |pass| {
        assert!(matches!(embed.record(pass), Err(EmbedError::DeviceLost(_))));
    });
    // The host makes a new renderer on its new device; the same draw data draws again.
    let mut again = make(&host, &spec);
    again.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
    let image = host.in_pass(&spec, true, |pass| {
        again.record(pass).unwrap();
    });
    assert!(max_diff(&image, &host.in_pass(&spec, true, |_| {})) > 100);
}

#[test]
fn a_new_target_format_rebuilds_only_what_depends_on_it() {
    let Some(host) = host("embedded format change") else {
        return;
    };
    let before = Spec::new(F::Bgra8UnormSrgb, SIZE);
    let after = Spec::new(F::Rgba16Float, SIZE);
    let data = scene::controls(SIZE, 1.25);
    let mut embed = make(&host, &before);
    embed.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
    let uploaded = embed.stats();
    embed.set_options(after.options()).unwrap();
    // The frame prepared for the old target is not drawn into the new one.
    host.in_pass(&after, false, |pass| {
        assert!(matches!(embed.record(pass), Err(EmbedError::NotPrepared)));
    });
    embed.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
    // Buffers and atlas stayed: same revision, no new upload.
    assert_eq!(embed.stats().geometry_uploads, uploaded.geometry_uploads);
    assert_eq!(embed.stats().texture_uploads, uploaded.texture_uploads);
    let got = host.in_pass(&after, true, |pass| {
        embed.record(pass).unwrap();
    });
    let mut fresh = make(&host, &after);
    fresh.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
    let want = host.in_pass(&after, true, |pass| {
        fresh.record(pass).unwrap();
    });
    assert_eq!(max_diff(&got, &want), 0);
    // An invalid new format leaves the renderer as it was.
    assert!(embed.set_options(EmbedOptions::new(F::R8Unorm)).is_err());
    assert_eq!(*embed.options(), after.options());
}

#[test]
fn resizing_the_region_rebuilds_the_backdrop_targets_for_the_new_size() {
    let Some(host) = host("embedded resize") else {
        return;
    };
    let mut embed = make(&host, &Spec::new(F::Rgba8UnormSrgb, SIZE));
    let mut bytes = Vec::new();
    for size in [[320, 224], [160, 112], [320, 224]] {
        let spec = Spec::new(F::Rgba8UnormSrgb, size);
        let data = scene::blur_panel(size, 1.0, panel(), 8.0);
        let got = host.frame(
            &spec,
            true,
            wgpu::TextureUsages::empty(),
            |encoder, target| {
                embed
                    .render_to(encoder, &target.view, None, EmbedLoad::Keep, &data)
                    .unwrap();
            },
        );
        let mut fresh = make(&host, &spec);
        let want = host.frame(
            &spec,
            true,
            wgpu::TextureUsages::empty(),
            |encoder, target| {
                fresh
                    .render_to(encoder, &target.view, None, EmbedLoad::Keep, &data)
                    .unwrap();
            },
        );
        assert_eq!(max_diff(&got, &want), 0, "{size:?}");
        bytes.push(embed.stats().blur_target_bytes);
    }
    assert!(bytes[1] < bytes[0] && bytes[2] == bytes[0], "{bytes:?}");
}

#[test]
fn a_clear_color_is_premultiplied_for_targets_composited_with_alpha() {
    let Some(host) = host("embedded clear alpha") else {
        return;
    };
    let half_red = zaxis::Color::rgba(255, 0, 0, 128);
    let data = scene::bare(&mut scene::see_through(SIZE, 1.0), &mut |_| {});
    let render = |alpha| {
        let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
        let mut embed = EmbeddedRenderer::new(
            host.device.clone(),
            host.queue.clone(),
            spec.options().alpha(alpha),
        )
        .unwrap();
        host.frame(
            &spec,
            false,
            wgpu::TextureUsages::empty(),
            |encoder, target| {
                embed
                    .render_to(
                        encoder,
                        &target.view,
                        None,
                        EmbedLoad::Clear(half_red),
                        &data,
                    )
                    .unwrap();
            },
        )[0]
    };
    assert_eq!(render(zaxis::EmbedAlpha::Opaque), [255, 0, 0, 128]);
    let premultiplied = render(zaxis::EmbedAlpha::Premultiplied);
    assert!(
        premultiplied[0].abs_diff(128) <= 1 && premultiplied[3] == 128,
        "{premultiplied:?}"
    );
}

/// The renderer may be prepared on the thread that builds the interface and recorded on the
/// one that records commands, behind a lock.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_renderer_moves_between_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<EmbeddedRenderer>();
    assert_send_sync::<zaxis::RecordReport>();
    assert_send_sync::<EmbedError>();
}
