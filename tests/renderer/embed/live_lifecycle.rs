//! What happens to a texture that is updated in place when the renderer cannot build on the
//! previous revision, and across renderers sharing one: the whole image is uploaded, and the
//! pixels are right either way.

use super::{
    harness::max_diff,
    live::{embedded, expected, fresh, gpu, region_rows, Noise, Rig, View},
    scene,
};
use std::sync::Arc;
use zaxis::{
    vec2, DrawCommand, DrawData, EmbedViewport, Image, ImageFit, ImageSource, Rect, Root,
    TextureId, TextureImage, TextureOptions, TextureRegion, TextureUpdate, Vertex,
};

const SIZE: [u32; 2] = [48, 40];

fn update(rig: &mut Rig<'_>, model: &mut [u8], noise: &mut Noise, region: TextureRegion) {
    let (rows, stride) = region_rows(model, SIZE[0], region, noise, Some(255), 8);
    rig.ctx
        .update_image_region(rig.handle, region, &rows, stride)
        .unwrap();
}

fn start<'h>(host: &'h super::harness::Host, noise: &mut Noise) -> (Rig<'h>, Vec<u8>) {
    let model = noise.bytes((SIZE[0] * SIZE[1] * 4) as usize, Some(255));
    let mut rig = Rig::new(
        host,
        ImageSource::rgba(SIZE, model.clone()).live(),
        View::exact(SIZE),
    );
    rig.pass();
    (rig, model)
}

#[test]
fn a_texture_without_a_base_is_uploaded_whole() {
    let Some(host) = gpu("live image fallbacks") else {
        return;
    };
    let mut noise = Noise(21);
    let (mut rig, mut model) = start(&host, &mut noise);
    let region = TextureRegion::new(5, 6, 7, 8);
    update(&mut rig, &mut model, &mut noise, region);
    let shown = rig.pass();
    assert_eq!(rig.stats().texture_region_updates, 1);
    assert_eq!(max_diff(&shown, &expected(&model)), 0);

    // Evicted: the next revision has nothing to build on.
    rig.embed.clear_image_textures();
    update(&mut rig, &mut model, &mut noise, region);
    let before = rig.stats();
    let shown = rig.pass();
    let after = rig.stats();
    assert_eq!(after.texture_creations, before.texture_creations + 1);
    assert_eq!(
        after.texture_update_fallbacks,
        before.texture_update_fallbacks + 1
    );
    assert_eq!(after.texture_region_updates, before.texture_region_updates);
    assert_eq!(max_diff(&shown, &expected(&model)), 0);

    // A renderer of its own, with a store of its own, meets the image at its latest revision.
    update(&mut rig, &mut model, &mut noise, region);
    rig.build();
    let mut other = embedded(&host, &rig.spec);
    other
        .prepare(rig.ctx.draw_data(), EmbedViewport::whole(SIZE))
        .unwrap();
    let stats = other.stats();
    assert_eq!(stats.texture_region_updates, 0);
    assert_eq!(stats.texture_update_fallbacks, 1, "{stats:?}");
    let shown = host.in_pass(&rig.spec, false, |pass| {
        other.record(pass).unwrap();
    });
    assert_eq!(max_diff(&shown, &expected(&model)), 0);

    // A frame that was built and never presented: the next one skips a revision.
    update(
        &mut rig,
        &mut model,
        &mut noise,
        TextureRegion::new(0, 0, 3, 3),
    );
    rig.build();
    update(
        &mut rig,
        &mut model,
        &mut noise,
        TextureRegion::new(20, 20, 3, 3),
    );
    let before = rig.stats();
    let shown = rig.pass();
    let after = rig.stats();
    assert_eq!(after.texture_region_updates, before.texture_region_updates);
    assert_eq!(
        after.texture_update_fallbacks,
        before.texture_update_fallbacks + 1
    );
    assert_eq!(after.texture_uploads, before.texture_uploads + 1);
    assert_eq!(max_diff(&shown, &expected(&model)), 0);

    // A replaced source is a new base: the texture is written whole but keeps its binding.
    let replaced = noise.bytes(model.len(), Some(255));
    rig.ctx
        .update_image(rig.handle, ImageSource::rgba(SIZE, replaced.clone()).live())
        .unwrap();
    let before = rig.stats();
    let shown = rig.pass();
    let after = rig.stats();
    assert_eq!(after.texture_region_updates, before.texture_region_updates);
    assert_eq!(after.texture_reuploads, before.texture_reuploads + 1);
    assert_eq!(after.texture_creations, before.texture_creations);
    assert_eq!(max_diff(&shown, &expected(&replaced)), 0);
}

#[test]
fn a_sibling_made_after_updates_began_sees_the_current_image() {
    let Some(host) = gpu("live image sibling") else {
        return;
    };
    let mut noise = Noise(33);
    let (mut rig, mut model) = start(&host, &mut noise);
    for step in 0..3 {
        update(
            &mut rig,
            &mut model,
            &mut noise,
            TextureRegion::new(step * 4, step, 9, 5),
        );
        rig.pass();
    }
    let mut sibling = rig.embed.create_sibling(rig.spec.options()).unwrap();
    sibling
        .prepare(rig.ctx.draw_data(), EmbedViewport::whole(SIZE))
        .unwrap();
    let stats = sibling.stats();
    // The store is shared, so the sibling finds the texture current: no write at all.
    assert_eq!(
        (stats.texture_writes, stats.texture_creations),
        (0, 0),
        "{stats:?}"
    );
    let shown = host.in_pass(&rig.spec, false, |pass| {
        sibling.record(pass).unwrap();
    });
    assert_eq!(max_diff(&shown, &expected(&model)), 0);
    // Whichever renderer meets the next revision first writes it; the other finds it done.
    update(
        &mut rig,
        &mut model,
        &mut noise,
        TextureRegion::new(1, 30, 40, 2),
    );
    rig.build();
    sibling
        .prepare(rig.ctx.draw_data(), EmbedViewport::whole(SIZE))
        .unwrap();
    rig.embed
        .prepare(rig.ctx.draw_data(), EmbedViewport::whole(SIZE))
        .unwrap();
    assert_eq!(sibling.stats().texture_region_updates, 1);
    assert_eq!(rig.stats().texture_writes, rig.stats().texture_writes);
    let first = host.in_pass(&rig.spec, false, |pass| {
        rig.embed.record(pass).unwrap();
    });
    assert_eq!(max_diff(&first, &expected(&model)), 0);
}

#[test]
fn windows_sharing_resources_share_the_update() {
    let Some(host) = gpu("live image windows") else {
        return;
    };
    let resources = zaxis::SharedResources::new();
    let mut noise = Noise(44);
    let mut model = noise.bytes((SIZE[0] * SIZE[1] * 4) as usize, Some(255));
    let mut a = scene::sharing(&resources, SIZE, 1.0);
    let mut b = scene::sharing(&resources, SIZE, 1.0);
    let handle = a
        .load_image(ImageSource::bgra(SIZE, model.clone()))
        .unwrap();
    let spec = super::harness::Spec::new(wgpu::TextureFormat::Rgba8UnormSrgb, SIZE);
    let mut first = embedded(&host, &spec);
    let mut second = first.create_sibling(spec.options()).unwrap();
    let show = |c: &mut zaxis::Context| {
        c.run(|c| {
            Root::new().padding(Default::default()).show(c, |ui| {
                let rect =
                    Rect::from_min_size(vec2(0.0, 0.0), vec2(SIZE[0] as f32, SIZE[1] as f32));
                ui.at("image", rect, |ui| {
                    ui.add(Image::new(handle).size(rect.size()).fit(ImageFit::Stretch));
                });
            });
        });
    };
    let present = |c: &mut zaxis::Context, e: &mut zaxis::EmbeddedRenderer| {
        show(c);
        e.prepare(c.draw_data(), EmbedViewport::whole(SIZE))
            .unwrap();
        host.in_pass(&spec, false, |pass| {
            e.record(pass).unwrap();
        })
    };
    present(&mut a, &mut first);
    present(&mut b, &mut second);
    for step in 0..5u32 {
        let region = TextureRegion::new(step * 3, step * 2, 10, 6);
        let (rows, stride) = region_rows(&mut model, SIZE[0], region, &mut noise, Some(255), 4);
        // The BGRA image is shown as it is: red and blue are the host's business.
        a.update_image_region(handle, region, &rows, stride)
            .unwrap();
        let one = present(&mut a, &mut first);
        let two = present(&mut b, &mut second);
        assert_eq!(max_diff(&one, &two), 0, "step {step}");
        let want: Vec<_> = model
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| [p[2], p[1], p[0], p[3]])
            .collect();
        assert_eq!(max_diff(&one, &want), 0, "step {step}");
    }
    assert_eq!(first.stats().texture_region_updates, 5);
    // The second window found every revision written already.
    assert_eq!(second.stats().texture_writes, 0);
    assert_eq!(second.stats().texture_creations, 0);
}

#[test]
fn the_backdrop_filter_sees_the_update() {
    let Some(host) = gpu("live image blur") else {
        return;
    };
    let mut noise = Noise(55);
    let model = noise.bytes((SIZE[0] * SIZE[1] * 4) as usize, Some(255));
    let mut view = View::exact(SIZE);
    view.blur = Some((Rect::from_min_size(vec2(4.0, 4.0), vec2(30.0, 24.0)), 6.0));
    let mut rig = Rig::new(&host, ImageSource::rgba(SIZE, model.clone()).live(), view);
    let before = rig.texture();
    let mut model = model;
    let region = TextureRegion::new(10, 8, 12, 12);
    let (rows, stride) = region_rows(&mut model, SIZE[0], region, &mut noise, Some(255), 0);
    rig.ctx
        .update_image_region(rig.handle, region, &rows, stride)
        .unwrap();
    let after = rig.texture();
    assert_ne!(max_diff(&before, &after), 0);
    assert_eq!(rig.stats().texture_region_updates, 1);
    let mut want = Rig::new(&host, ImageSource::rgba(SIZE, model.clone()).live(), view);
    assert_eq!(
        max_diff(&after, &want.texture()),
        0,
        "blurred pixels are those of a full upload"
    );
}

/// A frame built by hand: one textured quad over a target of `size`.
fn quad(size: [u32; 2], texture: TextureImage, updates: Vec<TextureUpdate>) -> DrawData {
    let mut data = DrawData::new(vec2(size[0] as f32, size[1] as f32), 1.0);
    let (w, h) = (size[0] as f32, size[1] as f32);
    data.vertices = [
        ([0.0, 0.0], [0.0, 0.0]),
        ([w, 0.0], [1.0, 0.0]),
        ([w, h], [1.0, 1.0]),
        ([0.0, h], [0.0, 1.0]),
    ]
    .map(|(position, uv)| Vertex {
        position,
        uv,
        color: [1.0; 4],
    })
    .to_vec();
    data.indices = vec![0, 1, 2, 0, 2, 3];
    data.commands.push(DrawCommand {
        indices: 0..6,
        clip_rect: Rect::from_min_size(vec2(0.0, 0.0), data.logical_size),
        texture: texture.id,
        ..Default::default()
    });
    data.texture_options
        .insert(texture.id, TextureOptions::default());
    data.textures.push(texture);
    data.texture_updates = updates;
    data.revision = 1;
    data
}

#[test]
fn updates_that_do_not_fit_the_payload_are_ignored_and_a_valid_one_writes_only_its_regions() {
    let Some(host) = gpu("hand-built updates") else {
        return;
    };
    let spec = super::harness::Spec::new(wgpu::TextureFormat::Rgba8UnormSrgb, SIZE);
    let image = |bytes: &[u8], revision| TextureImage {
        id: TextureId(900),
        size: SIZE,
        pixels: Arc::new(bytes.to_vec()),
        revision,
    };
    let len = (SIZE[0] * SIZE[1] * 4) as usize;
    let solid = |px: [u8; 4]| -> Vec<u8> { (0..len / 4).flat_map(|_| px).collect() };
    let (a, b) = (solid([30, 60, 90, 255]), solid([200, 200, 200, 255]));
    let draw = |embed: &mut zaxis::EmbeddedRenderer, data: &DrawData| {
        embed.prepare(data, EmbedViewport::whole(SIZE)).unwrap();
        host.in_pass(&spec, false, |pass| {
            embed.record(pass).unwrap();
        })
    };
    let update = |base, to, regions: Vec<TextureRegion>| TextureUpdate {
        id: TextureId(900),
        base_revision: base,
        to_revision: to,
        regions,
    };
    let bad = [
        update(1, 2, vec![]),
        update(1, 2, vec![TextureRegion::new(40, 0, 20, 4)]),
        update(1, 2, vec![TextureRegion::new(0, 0, 0, 4)]),
        update(2, 2, vec![TextureRegion::new(0, 0, 4, 4)]),
        update(1, 3, vec![TextureRegion::new(0, 0, 4, 4)]),
        update(0, 2, vec![TextureRegion::new(0, 0, u32::MAX, 4)]),
        TextureUpdate {
            id: TextureId(901),
            ..update(1, 2, vec![TextureRegion::new(0, 0, 4, 4)])
        },
    ];
    for bad in bad {
        let mut embed = embedded(&host, &spec);
        let first = quad(SIZE, image(&a, 1), vec![]);
        draw(&mut embed, &first);
        let before = embed.stats();
        let mut next = quad(SIZE, image(&b, 2), vec![bad.clone()]);
        next.source = first.source;
        let shown = draw(&mut embed, &next);
        let after = embed.stats();
        assert_eq!(
            after.texture_region_updates, before.texture_region_updates,
            "{bad:?}"
        );
        assert_eq!(
            after.texture_creations, before.texture_creations,
            "{bad:?}: reuses the texture"
        );
        assert!(
            shown.iter().all(|p| p[0] == 200),
            "{bad:?} must upload the payload"
        );
    }
    // A valid update trusts that nothing outside its regions changed.
    let mut embed = embedded(&host, &spec);
    let first = quad(SIZE, image(&a, 1), vec![]);
    draw(&mut embed, &first);
    let region = TextureRegion::new(4, 4, 8, 8);
    let mut next = quad(SIZE, image(&b, 2), vec![update(1, 2, vec![region])]);
    next.source = first.source;
    let shown = draw(&mut embed, &next);
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let inside = region.fits(SIZE) && (4..12).contains(&x) && (4..12).contains(&y);
            let want = if inside {
                [200, 200, 200, 255]
            } else {
                [30, 60, 90, 255]
            };
            assert_eq!(shown[(y * SIZE[0] + x) as usize], want, "({x}, {y})");
        }
    }
    assert_eq!(embed.stats().texture_region_bytes, 8 * 8 * 4);
    let _ = fresh;
}
