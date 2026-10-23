//! Regions of an image written in place: the pixels after a series of updates, and what
//! reached the GPU to get them.

use super::{
    harness::max_diff,
    live::{expected, fresh, gpu, region_rows, Noise, Rig, View},
};
use zaxis::{ImageSource, TextureRegion};

/// Sizes that straddle the 256-byte row alignment of copies: 1, 255, 257 and 4096 texels.
const SIZES: [[u32; 2]; 5] = [[1, 37], [255, 19], [257, 33], [4096, 8], [64, 64]];

/// The regions of one step, aimed at the edges and at overlaps of an image of `size`.
fn regions(noise: &mut Noise, [w, h]: [u32; 2], step: usize) -> Vec<TextureRegion> {
    let inside = |noise: &mut Noise| {
        let (rw, rh) = (1 + noise.below(w.min(40)), 1 + noise.below(h.min(12)));
        TextureRegion::new(noise.below(w - rw + 1), noise.below(h - rh + 1), rw, rh)
    };
    match step % 8 {
        0 => vec![TextureRegion::new(0, 0, 1, 1)],
        1 => vec![TextureRegion::new(w - 1, h - 1, 1, 1)],
        2 => vec![TextureRegion::new(0, noise.below(h), w, 1)],
        3 => vec![TextureRegion::new(noise.below(w), 0, 1, h)],
        4 => vec![inside(noise), inside(noise), inside(noise)],
        5 => {
            let a = inside(noise);
            // Overlapping and then touching the first one, where the image has room.
            let b = TextureRegion::new(
                a.x.saturating_sub(1).min(w - 1),
                a.y,
                1.max(a.width / 2),
                a.height,
            );
            let c = TextureRegion::new((a.x + a.width).min(w - 1), a.y, 1, a.height);
            vec![a, b, c]
        }
        6 => vec![TextureRegion::new(0, 0, w, h)],
        _ => vec![inside(noise)],
    }
}

#[test]
fn a_series_of_regions_shows_the_model_and_creates_nothing() {
    let Some(host) = gpu("live image regions") else {
        return;
    };
    let mut noise = Noise(7);
    for size @ [w, h] in SIZES {
        let mut model = noise.bytes((w * h * 4) as usize, Some(255));
        let view = View::exact(size);
        let mut rig = Rig::new(&host, ImageSource::rgba([w, h], model.clone()).live(), view);
        let shown = rig.pass();
        // Interpolated coordinates of a 4096-wide quad can be a fraction of a step off.
        assert!(
            max_diff(&shown, &expected(&model)) <= 1,
            "first frame {size:?}"
        );
        let first = rig.stats();
        assert_eq!(first.texture_region_updates, 0);
        for step in 0..16 {
            let before = rig.stats();
            let mut bytes = 0u64;
            let mut reported = Vec::new();
            for region in regions(&mut noise, size, step) {
                let pad = [0, 4, 256 - (region.width as usize * 4) % 256][noise.below(3) as usize];
                let (rows, stride) = region_rows(&mut model, w, region, &mut noise, Some(255), pad);
                rig.ctx
                    .update_image_region(rig.handle, region, &rows, stride)
                    .unwrap();
                reported.push(region);
            }
            let shown = rig.pass();
            let after = rig.stats();
            assert!(
                max_diff(&shown, &expected(&model)) <= 1,
                "{size:?} step {step}"
            );
            // Same texture and bind group; only the regions were written.
            assert_eq!(
                after.texture_creations, before.texture_creations,
                "{size:?} {step}"
            );
            assert_eq!(after.texture_reuploads, before.texture_reuploads);
            assert_eq!(after.texture_uploads, before.texture_uploads);
            assert_eq!(after.texture_upload_bytes, before.texture_upload_bytes);
            assert_eq!(
                after.texture_bind_group_creations,
                before.texture_bind_group_creations
            );
            assert_eq!(
                after.texture_update_fallbacks,
                before.texture_update_fallbacks
            );
            assert_eq!(
                after.texture_region_updates,
                before.texture_region_updates + 1
            );
            let update = rig.ctx.draw_data().texture_updates[0].clone();
            let written = TextureRegion::coalesce(&update.regions, size, 8);
            bytes += written.iter().map(|r| r.area() * 4).sum::<u64>();
            assert_eq!(
                after.texture_region_bytes - before.texture_region_bytes,
                bytes
            );
            assert_eq!(
                after.texture_writes - before.texture_writes,
                written.len() as u64
            );
            assert!(bytes <= u64::from(w * h * 4));
            for r in &reported {
                assert!(
                    written.iter().any(|w| w.union(*r) == *w),
                    "{r:?} not in {written:?}"
                );
            }
        }
        // The same final pixels as a context that was never updated.
        let again = fresh(&host, ImageSource::rgba([w, h], model.clone()), view);
        assert_eq!(max_diff(&rig.pass(), &again), 0, "{size:?}");
    }
}

#[test]
fn bytes_written_follow_the_area_of_the_region_not_the_frame() {
    let Some(host) = gpu("live image bytes") else {
        return;
    };
    let size = [1440, 900];
    let mut noise = Noise(3);
    let mut model = noise.bytes((size[0] * size[1] * 4) as usize, Some(255));
    let mut rig = Rig::new(
        &host,
        ImageSource::bgra_opaque(size, model.clone()),
        View::exact(size),
    );
    rig.pass();
    let full = rig.stats();
    assert_eq!(
        full.texture_upload_bytes % u64::from(size[0] * size[1] * 4),
        0
    );
    for rows in [1u32, 16, 64, 225] {
        let region = TextureRegion::new(0, 300, size[0], rows);
        let (pixels, stride) = region_rows(&mut model, size[0], region, &mut noise, Some(255), 0);
        rig.ctx
            .update_image_region(rig.handle, region, &pixels, stride)
            .unwrap();
        let before = rig.stats();
        rig.pass();
        let after = rig.stats();
        assert_eq!(
            after.texture_region_bytes - before.texture_region_bytes,
            u64::from(size[0] * rows * 4)
        );
        assert_eq!(after.texture_upload_bytes, before.texture_upload_bytes);
    }
    // Nothing changed: nothing is written, however often the frame is prepared.
    let idle = rig.stats();
    for _ in 0..5 {
        rig.pass();
    }
    let after = rig.stats();
    assert_eq!(after.texture_writes, idle.texture_writes);
    assert_eq!(after.texture_region_bytes, idle.texture_region_bytes);
    assert_eq!(after.texture_creations, idle.texture_creations);
    assert_eq!(
        after.texture_bind_group_creations,
        idle.texture_bind_group_creations
    );
}

#[test]
fn straight_alpha_regions_and_render_to_agree_with_a_full_upload() {
    let Some(host) = gpu("live image alpha") else {
        return;
    };
    let size = [96, 40];
    let mut noise = Noise(11);
    let mut model = noise.bytes((size[0] * size[1] * 4) as usize, None);
    let mut view = View::exact(size);
    view.radius = 6.0;
    view.opacity = 0.8;
    let mut rig = Rig::new(&host, ImageSource::rgba(size, model.clone()).live(), view);
    rig.pass();
    for step in 0..6 {
        let region = TextureRegion::new(step * 9, step * 3, 30 + step, 9);
        let (rows, stride) = region_rows(
            &mut model,
            size[0],
            region,
            &mut noise,
            None,
            256 - (region.width as usize * 4) % 256,
        );
        rig.ctx
            .update_image_region(rig.handle, region, &rows, stride)
            .unwrap();
        let by_pass = rig.pass();
        let want = fresh(&host, ImageSource::rgba(size, model.clone()), view);
        assert_eq!(max_diff(&by_pass, &want), 0, "pass, step {step}");
        let mut other = Rig::new(&host, ImageSource::rgba(size, model.clone()), view);
        other.settle();
        assert_eq!(
            max_diff(&other.texture(), &rig.texture()),
            0,
            "render_to, step {step}"
        );
    }
    assert!(rig.stats().texture_region_updates >= 6);
}
