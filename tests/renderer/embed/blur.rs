//! Backdrop effects: `render_to` filters what the host drew; a pass cannot, and says so.

use super::{
    harness::{host, max_diff, pixel, Host, Spec},
    scene,
};
use wgpu::TextureFormat as F;
use zaxis::{
    vec2, Color, EmbedLoad, EmbedViewport, EmbeddedRenderer, Material, ParamKind, PhysicalRect,
    Rect,
};

const SIZE: [u32; 2] = [320, 224];
/// A panel over the lower left of the triangle, which has sharp edges there.
const PANEL: Rect = Rect {
    min: vec2(40.0, 100.0),
    max: vec2(200.0, 190.0),
};

fn embedded(host: &Host, spec: &Spec) -> EmbeddedRenderer {
    EmbeddedRenderer::new_with_adapter(
        host.device.clone(),
        host.queue.clone(),
        &host.adapter,
        spec.options(),
    )
    .unwrap()
}

/// Largest difference between horizontal neighbours inside the panel.
fn sharpness(image: &[[u8; 4]]) -> u8 {
    let mut sharpest = 0;
    for y in PANEL.min.y as u32 + 8..PANEL.max.y as u32 - 8 {
        for x in PANEL.min.x as u32 + 8..PANEL.max.x as u32 - 8 {
            let (a, b) = (pixel(image, SIZE[0], x, y), pixel(image, SIZE[0], x + 1, y));
            sharpest = sharpest.max(a.iter().zip(b).map(|(a, b)| a.abs_diff(b)).max().unwrap());
        }
    }
    sharpest
}

#[test]
fn render_to_blurs_what_the_host_drew() {
    let Some(host) = host("embedded blur") else {
        return;
    };
    let spec = Spec::new(F::Bgra8UnormSrgb, SIZE);
    let data = scene::blur_panel(SIZE, 1.0, PANEL, 8.0);
    let scene_only = host.in_pass(&spec, true, |_| {});
    let mut embed = embedded(&host, &spec);
    let keep = host.frame(
        &spec,
        true,
        wgpu::TextureUsages::empty(),
        |encoder, target| {
            embed
                .render_to(encoder, &target.view, None, EmbedLoad::Keep, &data)
                .unwrap();
        },
    );
    assert!(
        sharpness(&scene_only) > 60,
        "the panel must sit over a sharp edge of the scene"
    );
    assert!(
        sharpness(&keep) < sharpness(&scene_only) / 2,
        "blurred {} sharp {}",
        sharpness(&keep),
        sharpness(&scene_only)
    );
    // Outside the panel nothing changed.
    assert_eq!(
        pixel(&keep, SIZE[0], 300, 10),
        pixel(&scene_only, SIZE[0], 300, 10)
    );
    assert_eq!(
        pixel(&keep, SIZE[0], 5, 5),
        pixel(&scene_only, SIZE[0], 5, 5)
    );
    // Cleared first, there is nothing to see through the panel but the clear color.
    let clear = Color::rgb(30, 30, 30);
    let cleared = host.frame(
        &spec,
        true,
        wgpu::TextureUsages::empty(),
        |encoder, target| {
            embed
                .render_to(encoder, &target.view, None, EmbedLoad::Clear(clear), &data)
                .unwrap();
        },
    );
    assert_eq!(pixel(&cleared, SIZE[0], 120, 150), [30, 30, 30, 255]);
    assert_ne!(
        pixel(&cleared, SIZE[0], 120, 150),
        pixel(&keep, SIZE[0], 120, 150)
    );
    assert!(embed.stats().blur_effects >= 2);
}

#[test]
fn render_to_in_a_region_blurs_the_host_scene_in_that_region_only() {
    let Some(host) = host("embedded blur region") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, [400, 300]);
    let region = PhysicalRect::new(60, 40, SIZE[0], SIZE[1]);
    let data = scene::blur_panel(SIZE, 1.0, PANEL, 8.0);
    let mut embed = embedded(&host, &spec);
    let scene_only = host.in_pass(&spec, true, |_| {});
    let image = host.frame(
        &spec,
        true,
        wgpu::TextureUsages::empty(),
        |encoder, target| {
            embed
                .render_to(encoder, &target.view, Some(region), EmbedLoad::Keep, &data)
                .unwrap();
        },
    );
    assert!(super::harness::same_outside(
        &image,
        &scene_only,
        400,
        region
    ));
    assert!(max_diff(&image, &scene_only) > 30);
}

#[test]
fn a_pass_cannot_blur_and_reports_what_it_skipped() {
    let Some(host) = host("embedded blur in a pass") else {
        return;
    };
    let spec = Spec::new(F::Bgra8UnormSrgb, SIZE);
    let data = scene::blur_panel(SIZE, 1.0, PANEL, 8.0);
    let scene_only = host.in_pass(&spec, true, |_| {});
    let mut embed = embedded(&host, &spec);
    embed.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
    let mut report = None;
    let image = host.in_pass(&spec, true, |pass| {
        report = Some(embed.record(pass).unwrap());
    });
    let report = report.unwrap();
    assert!(!report.skipped_backdrop.is_empty());
    for index in &report.skipped_backdrop {
        assert!(data.commands[*index].blur.is_some());
    }
    assert!(report.degraded_materials.is_empty());
    // The host's pixels are exactly as it drew them: no wrong blur, no white box.
    assert_eq!(max_diff(&image, &scene_only), 0);
}

fn invert() -> Material {
    Material::new(
        "invert",
        "fn material(in: MaterialInput, p: Params) -> vec4<f32> {
            let b = backdrop_sharp(in);
            return vec4<f32>(vec3<f32>(1.0) - b.rgb, 1.0);
        }",
    )
    .param("unused", ParamKind::F32)
    .reads_backdrop()
}

#[test]
fn a_material_that_reads_the_backdrop_sees_it_only_through_render_to() {
    let Some(host) = host("embedded backdrop material") else {
        return;
    };
    let spec = Spec::new(F::Bgra8UnormSrgb, SIZE);
    let mut context = scene::see_through(SIZE, 1.0);
    let id = context.register_material(&invert());
    let data = scene::bare(&mut context, &mut |ui| {
        ui.material(PANEL, id).show(ui);
    });
    let mut embed = embedded(&host, &spec);
    let scene_only = host.in_pass(&spec, true, |_| {});
    // In a pass the backdrop is transparent black: the inversion is white.
    embed.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
    let mut report = None;
    let in_pass = host.in_pass(&spec, true, |pass| {
        report = Some(embed.record(pass).unwrap());
    });
    assert!(!report.unwrap().degraded_materials.is_empty());
    assert_eq!(pixel(&in_pass, SIZE[0], 120, 150), [255, 255, 255, 255]);
    // Rendered into the texture, it inverts the host's pixel.
    let image = host.frame(
        &spec,
        true,
        wgpu::TextureUsages::empty(),
        |encoder, target| {
            embed
                .render_to(encoder, &target.view, None, EmbedLoad::Keep, &data)
                .unwrap();
        },
    );
    let seen = pixel(&scene_only, SIZE[0], 120, 150);
    let got = pixel(&image, SIZE[0], 120, 150);
    let invert = |c: u8| {
        let lin = |v: u8| (f32::from(v) / 255.0).powf(2.2);
        let enc = |v: f32| (v.max(0.0).powf(1.0 / 2.2) * 255.0).round();
        enc(1.0 - lin(c)) as u8
    };
    for channel in 0..3 {
        assert!(
            got[channel].abs_diff(invert(seen[channel])) <= 6,
            "channel {channel}: host {seen:?} inverted {got:?}"
        );
    }
}

#[test]
fn blur_agrees_across_target_formats() {
    let Some(host) = host("embedded blur formats") else {
        return;
    };
    let data = scene::blur_panel(SIZE, 1.25, PANEL, 8.0);
    let render = |format| {
        let spec = Spec::new(format, SIZE);
        let mut embed = embedded(&host, &spec);
        host.frame(
            &spec,
            true,
            wgpu::TextureUsages::empty(),
            |encoder, target| {
                embed
                    .render_to(encoder, &target.view, None, EmbedLoad::Keep, &data)
                    .unwrap();
            },
        )
    };
    let reference = render(F::Rgba8UnormSrgb);
    let scene_only = host.in_pass(&Spec::new(F::Rgba8UnormSrgb, SIZE), true, |_| {});
    assert!(
        max_diff(&reference, &scene_only) > 30,
        "the panel changed nothing"
    );
    for format in [F::Bgra8UnormSrgb, F::Rgba8Unorm, F::Bgra8Unorm] {
        assert_eq!(max_diff(&render(format), &reference), 0, "{format:?}");
    }
    // A float canvas keeps more precision than the sRGB bytes it is compared with.
    assert!(max_diff(&render(F::Rgba16Float), &reference) <= 4);
}
