//! `prepare` + `record` into the host's pass: parity with the reference for every format,
//! sample count, depth attachment and scale, the interface over the host's scene, and the
//! state of the pass afterwards.

use super::{
    harness::{host, max_diff, pixel, Host, Spec},
    reference::Reference,
    scene,
};
use wgpu::TextureFormat as F;
use zaxis::{EmbedViewport, EmbeddedRenderer, PhysicalRect};

const SIZE: [u32; 2] = [320, 224];
/// Texture formats a host commonly renders to; the unorm ones through an sRGB view.
const FORMATS: [F; 4] = [
    F::Bgra8UnormSrgb,
    F::Rgba8Unorm,
    F::Rgba8UnormSrgb,
    F::Rgba16Float,
];

fn embedded(host: &Host, spec: &Spec) -> EmbeddedRenderer {
    EmbeddedRenderer::new_with_adapter(
        host.device.clone(),
        host.queue.clone(),
        &host.adapter,
        spec.options(),
    )
    .expect("options accepted")
}

#[test]
fn record_matches_the_reference_for_every_format_samples_depth_and_scale() {
    let Some(host) = host("embedded record parity") else {
        return;
    };
    let mut compared = 0;
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let data = scene::controls(SIZE, scale);
        for format in FORMATS {
            for samples in [1, 4] {
                for depth in [false, true] {
                    let spec = Spec {
                        samples,
                        depth,
                        ..Spec::new(format, SIZE)
                    };
                    if !host.supported(&spec) {
                        eprintln!("SKIPPED {spec:?}: not supported by the adapter");
                        continue;
                    }
                    let mut embed = embedded(&host, &spec);
                    embed.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
                    let mut report = None;
                    let got = host.in_pass(&spec, true, |pass| {
                        report = Some(embed.record(pass).unwrap());
                    });
                    let report = report.unwrap();
                    assert!(report.draw_calls > 0);
                    assert!(report.skipped_backdrop.is_empty());
                    let want_ui =
                        Reference::prepare(&host, &spec, &data, PhysicalRect::whole(SIZE));
                    let want = host.in_pass(&spec, true, |pass| want_ui.record(pass));
                    assert_eq!(
                        max_diff(&got, &want),
                        0,
                        "{spec:?} at scale {scale} differs from the reference"
                    );
                    let host_only = host.in_pass(&spec, true, |_| {});
                    assert!(
                        max_diff(&got, &host_only) > 100,
                        "{spec:?} at scale {scale}: the interface drew nothing"
                    );
                    compared += 1;
                }
            }
        }
    }
    assert!(
        compared >= 16,
        "only {compared} combinations were supported"
    );
}

#[test]
fn every_format_agrees_with_the_srgb_window_format() {
    let Some(host) = host("embedded cross-format parity") else {
        return;
    };
    let data = scene::controls(SIZE, 1.25);
    let render = |format| {
        let spec = Spec::new(format, SIZE);
        let mut embed = embedded(&host, &spec);
        embed.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
        host.in_pass(&spec, true, |pass| {
            embed.record(pass).unwrap();
        })
    };
    let window = render(F::Rgba8UnormSrgb);
    for format in [F::Bgra8UnormSrgb, F::Rgba8Unorm, F::Bgra8Unorm] {
        assert_eq!(max_diff(&render(format), &window), 0, "{format:?}");
    }
    // A float target stores linear values: blending is the same, only rounding differs.
    assert!(max_diff(&render(F::Rgba16Float), &window) <= 3);
}

#[test]
fn translucent_controls_blend_over_the_hosts_scene() {
    let Some(host) = host("embedded translucency") else {
        return;
    };
    let spec = Spec {
        depth: true,
        samples: 4,
        ..Spec::new(F::Bgra8UnormSrgb, SIZE)
    };
    if !host.supported(&spec) {
        eprintln!("SKIPPED translucency: 4x MSAA unsupported");
        return;
    }
    let data = scene::controls(SIZE, 1.0);
    let mut embed = embedded(&host, &spec);
    embed.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
    let with_scene = host.in_pass(&spec, true, |pass| {
        embed.record(pass).unwrap();
    });
    let without_scene = host.in_pass(&spec, false, |pass| {
        embed.record(pass).unwrap();
    });
    let scene_only = host.in_pass(&spec, true, |_| {});
    // Where the card is translucent the scene shows through it.
    let differing = with_scene
        .iter()
        .zip(&without_scene)
        .zip(&scene_only)
        .filter(|((a, b), s)| a != b && a != s)
        .count();
    assert!(
        differing > 500,
        "{differing} pixels mix the scene and the card"
    );
    // The host's depth attachment is untouched by a flat interface: the triangle is intact
    // where the interface is not.
    assert_eq!(
        pixel(&with_scene, SIZE[0], 300, 10),
        pixel(&scene_only, SIZE[0], 300, 10)
    );
}

#[test]
fn record_hands_the_pass_back_with_default_viewport_and_scissor() {
    let Some(host) = host("embedded pass state") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
    let region = PhysicalRect::new(40, 30, 240, 160);
    let data = scene::controls(region.size(), 1.0);
    let mut embed = embedded(&host, &spec);
    embed
        .prepare(&data, EmbedViewport::region(SIZE, region))
        .unwrap();
    // After record the host draws its scene without setting a viewport or scissor again: it
    // must still cover the whole target, not just the region the interface used.
    let image = host.in_pass(&spec, false, |pass| {
        embed.record(pass).unwrap();
        scene::host_scene(&host, pass, &spec);
    });
    let scene_only = host.in_pass(&spec, true, |_| {});
    assert!(super::harness::same_outside(
        &image,
        &scene_only,
        SIZE[0],
        region
    ));
}
