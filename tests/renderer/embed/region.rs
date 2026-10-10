//! The interface in a rectangle of the host's target: offset, DPI and clipping at the edge.

use super::{
    harness::{host, max_diff, same_outside, Host, Spec},
    reference::Reference,
    scene,
};
use wgpu::TextureFormat as F;
use zaxis::{EmbedViewport, EmbeddedRenderer, PhysicalRect};

const TARGET: [u32; 2] = [400, 300];

fn embedded(host: &Host, spec: &Spec) -> EmbeddedRenderer {
    EmbeddedRenderer::new_with_adapter(
        host.device.clone(),
        host.queue.clone(),
        &host.adapter,
        spec.options(),
    )
    .unwrap()
}

#[test]
fn a_region_draws_what_a_target_of_its_size_would_and_nothing_outside() {
    let Some(host) = host("embedded regions") else {
        return;
    };
    let spec = Spec::new(F::Bgra8UnormSrgb, TARGET);
    // Physical sizes that are whole logical sizes at each scale, at odd offsets, at the
    // edges and at the far corner.
    let cases = [
        (1.0, PhysicalRect::new(37, 21, 240, 180)),
        (1.25, PhysicalRect::new(11, 7, 250, 200)),
        (1.5, PhysicalRect::new(99, 75, 300, 225)),
        (2.0, PhysicalRect::new(160, 100, 240, 200)),
        (1.5, PhysicalRect::new(0, 0, 300, 225)),
    ];
    let host_only = host.in_pass(&spec, true, |_| {});
    for (scale, region) in cases {
        let data = scene::controls(region.size(), scale);
        let mut embed = embedded(&host, &spec);
        embed
            .prepare(&data, EmbedViewport::region(TARGET, region))
            .unwrap();
        let placed = host.in_pass(&spec, true, |pass| {
            embed.record(pass).unwrap();
        });
        assert!(
            same_outside(&placed, &host_only, TARGET[0], region),
            "scale {scale}: pixels outside {region:?} changed"
        );
        let want_ui = Reference::prepare(&host, &spec, &data, region);
        let want = host.in_pass(&spec, true, |pass| want_ui.record(pass));
        assert_eq!(max_diff(&placed, &want), 0, "scale {scale}, {region:?}");
        // The same data drawn into a target of the size of the region, shifted by hand.
        let own = Spec::new(F::Bgra8UnormSrgb, region.size());
        let mut alone = embedded(&host, &own);
        alone
            .prepare(&data, EmbedViewport::whole(region.size()))
            .unwrap();
        let small = host.in_pass(&own, false, |pass| {
            alone.record(pass).unwrap();
        });
        let ui_only = host.in_pass(&spec, false, |pass| {
            embed.record(pass).unwrap();
        });
        let clear = host.in_pass(&spec, false, |_| {});
        let mut equal = 0;
        for y in 0..region.height {
            for x in 0..region.width {
                let inside = ui_only[((region.y + y) * TARGET[0] + region.x + x) as usize];
                let alone = small[(y * region.width + x) as usize];
                // Rasterizing at an offset may round a gradient by one step.
                assert!(
                    inside.iter().zip(alone).all(|(a, b)| a.abs_diff(b) <= 2),
                    "scale {scale}, {region:?}: pixel ({x}, {y}) of the region differs"
                );
                equal += u32::from(inside != clear[0]);
            }
        }
        assert!(equal > 1000, "scale {scale}: the interface drew too little");
    }
}

#[test]
fn geometry_beyond_the_region_never_reaches_the_host_pixels_around_it() {
    let Some(host) = host("embedded clipping") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, TARGET);
    let region = PhysicalRect::new(100, 80, 200, 120);
    // The draw data believes its viewport is the region, but the shapes extend far past it.
    let mut context = scene::see_through(region.size(), 1.0);
    let mut data = scene::bare(&mut context, &mut |ui| {
        let wide = zaxis::Rect::from_min_size(zaxis::vec2(-80.0, -60.0), zaxis::vec2(600.0, 400.0));
        ui.paint(zaxis::Shape::rect(wide, zaxis::Color::rgb(250, 60, 60)));
    });
    // Neither the shapes nor their clips stay inside the viewport the data was built for.
    let huge =
        zaxis::Rect::from_min_size(zaxis::vec2(-1000.0, -1000.0), zaxis::vec2(3000.0, 3000.0));
    for command in &mut data.commands {
        command.clip_rect = huge;
    }
    let mut embed = embedded(&host, &spec);
    embed
        .prepare(&data, EmbedViewport::region(TARGET, region))
        .unwrap();
    let host_only = host.in_pass(&spec, true, |_| {});
    let image = host.in_pass(&spec, true, |pass| {
        embed.record(pass).unwrap();
    });
    assert!(same_outside(&image, &host_only, TARGET[0], region));
    assert_eq!(image[(region.y * TARGET[0] + region.x) as usize][0], 250);
    let last = ((region.y + region.height - 1) * TARGET[0] + region.x + region.width - 1) as usize;
    assert_eq!(image[last][0], 250);
}

#[test]
fn render_to_draws_a_region_exactly_like_record() {
    let Some(host) = host("embedded render_to parity") else {
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, TARGET);
    let region = PhysicalRect::new(37, 21, 250, 200);
    let data = scene::controls(region.size(), 1.25);
    let mut embed = embedded(&host, &spec);
    embed
        .prepare(&data, EmbedViewport::region(TARGET, region))
        .unwrap();
    let recorded = host.in_pass(&spec, true, |pass| {
        embed.record(pass).unwrap();
    });
    let rendered = host.frame(
        &spec,
        true,
        wgpu::TextureUsages::empty(),
        |encoder, target| {
            embed
                .render_to(
                    encoder,
                    &target.view,
                    Some(region),
                    zaxis::EmbedLoad::Keep,
                    &data,
                )
                .unwrap();
        },
    );
    assert_eq!(max_diff(&recorded, &rendered), 0);
    assert!(embed.stats().presented_frames >= 2);
}
