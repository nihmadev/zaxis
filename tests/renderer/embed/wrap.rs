//! Read back the actual renderer: what a wrap paints is where its input regions are, at
//! every scale factor and through a clipping scroll area.
use super::harness::{host, pixel, Spec};
use zaxis::testing::Inspect;
use zaxis::winit::dpi::PhysicalSize;
use zaxis::{
    vec2, Color, Context, EmbedViewport, EmbeddedRenderer, Id, Padding, Rect, Root, ScrollArea,
    Sense, Shape, Wrap,
};

const SIZES: [(f32, f32); 6] = [
    (70.0, 30.0),
    (50.0, 50.0),
    (90.0, 20.0),
    (60.0, 40.0),
    (80.0, 25.0),
    (45.0, 35.0),
];
const COLORS: [Color; 6] = [
    Color::rgb(235, 40, 40),
    Color::rgb(40, 220, 60),
    Color::rgb(50, 80, 240),
    Color::rgb(240, 220, 40),
    Color::rgb(230, 50, 220),
    Color::rgb(40, 225, 225),
];

fn near(pixel: [u8; 4], color: Color) -> bool {
    (0..3).all(|i| pixel[i].abs_diff(color.0[i]) <= 12)
}

fn scene(c: &mut Context, scroll: Option<f32>) -> Vec<Id> {
    let mut ids = Vec::new();
    c.run(|c| {
        Root::new().padding(Padding::all(0.0)).show(c, |ui| {
            let mut wrapped = |ui: &mut zaxis::Ui<'_>| {
                Wrap::new("w").width(220.0).gap(8.0).show(ui, |ui| {
                    for (i, (w, h)) in SIZES.into_iter().enumerate() {
                        let rect = ui.allocate_space(vec2(w, h));
                        ui.paint(Shape::rect(rect, COLORS[i]));
                        ids.push(ui.interact(rect, ("block", i), Sense::CLICK).id);
                    }
                });
            };
            match scroll {
                None => wrapped(ui),
                Some(offset) => {
                    ScrollArea::vertical()
                        .id_source("clip")
                        .max_height(80.0)
                        .max_width(240.0)
                        .scroll_offset(vec2(0.0, offset))
                        .show(ui, wrapped);
                }
            }
        })
    });
    ids
}

#[test]
fn pixels_of_each_item_are_inside_its_input_region_at_every_scale() {
    let Some(host) = host("Wrap readback") else {
        return;
    };
    for dpi in [1.0, 1.25, 1.5, 2.0] {
        for scroll in [None, Some(0.0), Some(45.0)] {
            let size = [(300.0 * dpi) as u32, (240.0 * dpi) as u32];
            let spec = Spec::new(wgpu::TextureFormat::Rgba8UnormSrgb, size);
            let mut c = Context::new();
            c.set_viewport(PhysicalSize::new(size[0], size[1]), dpi);
            scene(&mut c, scroll);
            let ids = scene(&mut c, scroll);
            let mut renderer = EmbeddedRenderer::new_with_adapter(
                host.device.clone(),
                host.queue.clone(),
                &host.adapter,
                spec.options(),
            )
            .unwrap();
            renderer
                .prepare(c.draw_data(), EmbedViewport::whole(size))
                .unwrap();
            let pixels = host.in_pass(&spec, false, |pass| {
                renderer.record(pass).unwrap();
            });
            let regions: Vec<Rect> = ids
                .iter()
                .map(|id| {
                    c.probe()
                        .previous_hits
                        .iter()
                        .find(|h| h.id == *id)
                        .map(|h| h.rect.intersect(h.clip))
                        .unwrap_or_default()
                })
                .collect();
            let mut painted = [0_usize; 6];
            for y in 0..size[1] {
                for x in 0..size[0] {
                    let rgba = pixel(&pixels, size[0], x, y);
                    let p = vec2(x as f32 + 0.5, y as f32 + 0.5) / dpi as f32;
                    for (i, color) in COLORS.into_iter().enumerate() {
                        if near(rgba, color) {
                            painted[i] += 1;
                            // Edge pixels may be blended into the region's border by half a
                            // pixel; the interior of what is painted is the region.
                            let region = regions[i];
                            let grown = Rect::from_min_max(
                                region.min - vec2(1.0, 1.0),
                                region.max + vec2(1.0, 1.0),
                            );
                            assert!(
                                grown.contains(p),
                                "block {i} painted at {p:?} outside its region {region:?} \
                                 (dpi {dpi}, scroll {scroll:?})"
                            );
                            if region.contains(p)
                                && region.contains(p - vec2(1.0, 1.0))
                                && region.contains(p + vec2(1.0, 1.0))
                            {
                                assert_eq!(c.hit_test(p).map(|h| h.id), Some(ids[i]));
                            }
                        }
                    }
                }
            }
            for (i, region) in regions.iter().enumerate() {
                if region.is_empty() {
                    assert_eq!(
                        painted[i], 0,
                        "block {i} is clipped away and paints nothing"
                    );
                    continue;
                }
                let centre = region.center();
                let rgba = pixel(
                    &pixels,
                    size[0],
                    (centre.x * dpi as f32) as u32,
                    (centre.y * dpi as f32) as u32,
                );
                assert!(
                    near(rgba, COLORS[i]),
                    "block {i} is not painted where it takes input: {rgba:?} at {centre:?}; regions {regions:?}; dpi {dpi}, scroll {scroll:?}"
                );
                assert_eq!(c.hit_test(centre).map(|h| h.id), Some(ids[i]));
            }
            if scroll == Some(45.0) {
                assert!(
                    regions.iter().any(|r| r.is_empty()),
                    "something is scrolled out"
                );
                assert!(painted.iter().any(|n| *n > 0));
            }
            if dpi == 1.5 && scroll.is_none() {
                std::fs::create_dir_all("target/wrap-validation").unwrap();
                let rgba: Vec<u8> = pixels.into_iter().flatten().collect();
                image::save_buffer(
                    "target/wrap-validation/readback.png",
                    &rgba,
                    size[0],
                    size[1],
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
