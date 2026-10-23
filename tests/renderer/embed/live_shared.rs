//! Textures that share an allocation or compete for the budget while one of them is updated.

use super::{
    harness::{max_diff, Spec},
    live::{embedded, expected, gpu, region_rows, Noise},
    scene,
};
use std::sync::Arc;
use wgpu::TextureFormat as F;
use zaxis::{
    vec2, EmbedViewport, Image, ImageFit, ImageHandle, ImageSource, Rect, Root, TextureFilter,
    TextureRegion,
};

const W: u32 = 32;
const H: u32 = 24;

/// Images side by side, each `W` by `H` at 1:1: (handle, filter).
fn row(c: &mut zaxis::Context, images: &[(ImageHandle, TextureFilter)]) {
    c.run(|c| {
        Root::new().padding(Default::default()).show(c, |ui| {
            for (i, (handle, filter)) in images.iter().enumerate() {
                let rect =
                    Rect::from_min_size(vec2((i as u32 * W) as f32, 0.0), vec2(W as f32, H as f32));
                ui.at(i, rect, |ui| {
                    ui.add(
                        Image::new(*handle)
                            .size(rect.size())
                            .fit(ImageFit::Stretch)
                            .filter(*filter),
                    );
                });
            }
        });
    });
}

/// The model of images side by side.
fn side_by_side(models: &[&[u8]]) -> Vec<[u8; 4]> {
    let mut out = vec![[0; 4]; (models.len() as u32 * W * H) as usize];
    for (i, model) in models.iter().enumerate() {
        for y in 0..H as usize {
            for x in 0..W as usize {
                let at = (y * W as usize + x) * 4;
                out[y * models.len() * W as usize + i * W as usize + x] =
                    [model[at], model[at + 1], model[at + 2], model[at + 3]];
            }
        }
    }
    out
}

#[test]
fn both_filters_of_one_image_are_written_once() {
    let Some(host) = gpu("live image filters") else {
        return;
    };
    let size = [2 * W, H];
    let spec = Spec::new(F::Rgba8UnormSrgb, size);
    let mut noise = Noise(2);
    let mut model = noise.bytes((W * H * 4) as usize, Some(255));
    let mut c = scene::context(size, 1.0);
    let h = c
        .load_image(ImageSource::rgba([W, H], model.clone()).live())
        .unwrap();
    let both = [(h, TextureFilter::Linear), (h, TextureFilter::Nearest)];
    let mut embed = embedded(&host, &spec);
    let mut present = |c: &mut zaxis::Context| {
        row(c, &both);
        embed
            .prepare(c.draw_data(), EmbedViewport::whole(size))
            .unwrap();
        host.in_pass(&spec, false, |pass| {
            embed.record(pass).unwrap();
        })
    };
    present(&mut c);
    for step in 0..4u32 {
        let region = TextureRegion::new(step, step * 2, 9, 7);
        let (rows, stride) = region_rows(&mut model, W, region, &mut noise, Some(255), 12);
        c.update_image_region(h, region, &rows, stride).unwrap();
        let shown = present(&mut c);
        assert_eq!(
            max_diff(&shown, &side_by_side(&[&model, &model])),
            0,
            "step {step}"
        );
    }
    let stats = embed.stats();
    assert_eq!(stats.texture_region_updates, 4, "{stats:?}");
    assert_eq!(stats.texture_update_fallbacks, 0, "{stats:?}");
    // One allocation, written once per update, not once per filter.
    assert_eq!(stats.texture_region_writes, 4, "{stats:?}");
}

#[test]
fn another_image_on_the_same_buffer_is_never_touched() {
    let Some(host) = gpu("live image aliases") else {
        return;
    };
    let size = [2 * W, H];
    let spec = Spec::new(F::Rgba8UnormSrgb, size);
    let mut noise = Noise(8);
    let model = Arc::new(noise.bytes((W * H * 4) as usize, Some(255)));
    let mut c = scene::context(size, 1.0);
    // Two entries over one buffer: the live one is updated, the plain one is not.
    let live = c
        .load_image(ImageSource::rgba([W, H], model.clone()).live())
        .unwrap();
    let plain = c
        .load_image(ImageSource::rgba([W, H], model.clone()))
        .unwrap();
    assert_ne!(live, plain);
    let pair = [
        (live, TextureFilter::Linear),
        (plain, TextureFilter::Linear),
    ];
    let mut embed = embedded(&host, &spec);
    let start = std::time::Instant::now();
    let present = |c: &mut zaxis::Context, embed: &mut zaxis::EmbeddedRenderer| {
        row(c, &pair);
        embed
            .prepare(c.draw_data(), EmbedViewport::whole(size))
            .unwrap();
        host.in_pass(&spec, false, |pass| {
            embed.record(pass).unwrap();
        })
    };
    while !c.image_state(plain).is_ready() {
        present(&mut c, &mut embed);
        assert!(start.elapsed().as_secs() < 10);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let shown = present(&mut c, &mut embed);
    assert_eq!(max_diff(&shown, &side_by_side(&[&model, &model])), 0);
    let mut changed = model.as_ref().clone();
    let region = TextureRegion::new(3, 3, 10, 10);
    let (rows, stride) = region_rows(&mut changed, W, region, &mut noise, Some(255), 0);
    c.update_image_region(live, region, &rows, stride).unwrap();
    let shown = present(&mut c, &mut embed);
    assert_eq!(
        max_diff(&shown, &side_by_side(&[&changed, &model])),
        0,
        "the update reached the other image"
    );
    // Not a region write: the allocation was shared, so the updated image got a texture
    // of its own with the whole frame.
    let stats = embed.stats();
    assert_eq!(stats.texture_region_updates, 0, "{stats:?}");
    assert!(stats.texture_update_fallbacks >= 1, "{stats:?}");
    // From then on it is alone, and regions are written again.
    let (rows, stride) = region_rows(&mut changed, W, region, &mut noise, Some(255), 0);
    c.update_image_region(live, region, &rows, stride).unwrap();
    let shown = present(&mut c, &mut embed);
    assert_eq!(max_diff(&shown, &side_by_side(&[&changed, &model])), 0);
    assert_eq!(embed.stats().texture_region_updates, 1);
    let _ = expected;
}

#[test]
fn an_evicted_texture_is_rebuilt_whole_and_the_budget_counts_the_full_texture() {
    let Some(host) = gpu("live image budget") else {
        return;
    };
    let size = [W, H];
    let spec = Spec::new(F::Rgba8UnormSrgb, size);
    let mut noise = Noise(12);
    let mut c = scene::context(size, 1.0);
    let frame = (W * H * 4) as u64;
    // Room for one image at a time.
    let mut limits = c.image_limits();
    limits.gpu_cache_bytes = frame + frame / 2;
    c.set_image_limits(limits);
    let mut a = noise.bytes(frame as usize, Some(255));
    let b = noise.bytes(frame as usize, Some(255));
    let ha = c
        .load_image(ImageSource::rgba([W, H], a.clone()).live())
        .unwrap();
    let hb = c
        .load_image(ImageSource::rgba([W, H], b.clone()).live())
        .unwrap();
    let mut embed = embedded(&host, &spec);
    present(&host, &spec, &mut embed, &mut c, ha);
    // A small region is cheap, but the texture still counts in full.
    let region = TextureRegion::new(0, 0, 2, 2);
    let (rows, stride) = region_rows(&mut a, W, region, &mut noise, Some(255), 0);
    c.update_image_region(ha, region, &rows, stride).unwrap();
    present(&host, &spec, &mut embed, &mut c, ha);
    assert_eq!(embed.stats().texture_region_updates, 1);
    assert!(embed.stats().image_resident_bytes_estimate >= frame);
    // The other image takes the room; the first is evicted while it is not drawn.
    let shown = present(&host, &spec, &mut embed, &mut c, hb);
    assert_eq!(max_diff(&shown, &expected(&b)), 0);
    assert!(embed.stats().texture_evictions >= 1, "{:?}", embed.stats());
    let (rows, stride) = region_rows(&mut a, W, region, &mut noise, Some(255), 0);
    c.update_image_region(ha, region, &rows, stride).unwrap();
    let before = embed.stats();
    let shown = present(&host, &spec, &mut embed, &mut c, ha);
    let after = embed.stats();
    assert_eq!(max_diff(&shown, &expected(&a)), 0);
    assert_eq!(after.texture_region_updates, before.texture_region_updates);
    assert_eq!(after.texture_creations, before.texture_creations + 1);
    assert_eq!(
        after.texture_update_fallbacks,
        before.texture_update_fallbacks + 1
    );
}

fn present(
    host: &super::harness::Host,
    spec: &Spec,
    embed: &mut zaxis::EmbeddedRenderer,
    c: &mut zaxis::Context,
    h: ImageHandle,
) -> Vec<[u8; 4]> {
    row(c, &[(h, TextureFilter::Linear)]);
    embed
        .prepare(c.draw_data(), EmbedViewport::whole(spec.size))
        .unwrap();
    host.in_pass(spec, false, |pass| {
        embed.record(pass).unwrap();
    })
}
