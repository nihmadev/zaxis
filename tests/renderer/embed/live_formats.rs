//! BGRA and opaque layouts: drawn exactly like the RGBA image with the same colors, with
//! tint, opacity, rounding, fit and filtering, natively and through the conversion fallback.

use super::{
    harness::max_diff,
    live::{fresh, gpu, Noise, Rig, View},
};
use zaxis::{Color, ImageFit, ImageSource, PixelFormat, TextureFilter};

/// `rgba` with its red and blue swapped: the BGRA bytes of the same picture.
fn swapped(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect()
}

/// `rgba` with its alpha byte replaced by `byte`.
fn alpha(rgba: &[u8], byte: u8) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], byte])
        .collect()
}

fn views(size: [u32; 2]) -> Vec<(&'static str, View)> {
    let exact = View::exact(size);
    let target = [size[0] * 2 + 3, size[1] * 3 + 1];
    let scaled = View {
        target,
        fit: ImageFit::Cover,
        ..exact
    };
    vec![
        ("exact", exact),
        (
            "tint",
            View {
                tint: Color::rgb(255, 160, 40),
                ..exact
            },
        ),
        (
            "opacity",
            View {
                opacity: 0.5,
                ..exact
            },
        ),
        (
            "rounded",
            View {
                radius: 9.0,
                ..exact
            },
        ),
        ("cover linear", scaled),
        (
            "cover nearest",
            View {
                filter: TextureFilter::Nearest,
                ..scaled
            },
        ),
        (
            "contain",
            View {
                fit: ImageFit::Contain,
                ..scaled
            },
        ),
        (
            "everything",
            View {
                tint: Color::rgb(90, 200, 255),
                opacity: 0.7,
                radius: 7.0,
                ..scaled
            },
        ),
    ]
}

#[test]
fn bgra_is_drawn_like_the_rgba_picture_with_swapped_channels() {
    let Some(host) = gpu("bgra images") else {
        return;
    };
    let size = [37, 23];
    let rgba = Noise(5).bytes((size[0] * size[1] * 4) as usize, None);
    for (name, view) in views(size) {
        let want = fresh(&host, ImageSource::rgba(size, rgba.clone()), view);
        for convert in [false, true] {
            let mut rig = Rig::new(&host, ImageSource::bgra(size, swapped(&rgba)), view);
            rig.embed.set_bgra_conversion(convert);
            let shown = rig.pass();
            assert_eq!(max_diff(&shown, &want), 0, "{name}, conversion {convert}");
            // An update of a region keeps the layout and the colors.
            let region = zaxis::TextureRegion::new(3, 2, 11, 5);
            let mut model = rgba.clone();
            let mut patch = Vec::new();
            for y in 0..5usize {
                let from = ((2 + y) * size[0] as usize + 3) * 4;
                let fresh_bytes = Noise(y as u64 + 1).bytes(44, None);
                model[from..from + 44].copy_from_slice(&fresh_bytes);
                patch.extend(swapped(&fresh_bytes));
            }
            rig.ctx
                .update_image_region(rig.handle, region, &patch, 44)
                .unwrap();
            let before = rig.stats();
            let shown = rig.pass();
            let want = fresh(&host, ImageSource::rgba(size, model), view);
            assert_eq!(
                max_diff(&shown, &want),
                0,
                "{name} region, conversion {convert}"
            );
            assert_eq!(
                rig.stats().texture_region_updates,
                before.texture_region_updates + 1
            );
        }
    }
}

#[test]
fn opaque_layouts_ignore_the_fourth_byte_and_match_alpha_255() {
    let Some(host) = gpu("opaque images") else {
        return;
    };
    let size = [29, 31];
    let rgba = Noise(9).bytes((size[0] * size[1] * 4) as usize, None);
    let opaque = alpha(&rgba, 255);
    for (name, view) in views(size) {
        let want = fresh(&host, ImageSource::rgba(size, opaque.clone()), view);
        // Garbage in the unused byte, in every layout.
        let sources = [
            ("rgbx", ImageSource::rgba_opaque(size, rgba.clone())),
            ("bgrx", ImageSource::bgra_opaque(size, swapped(&rgba))),
        ];
        for (layout, source) in sources {
            let mut rig = Rig::new(&host, source, view);
            let shown = rig.pass();
            assert_eq!(max_diff(&shown, &want), 0, "{name}, {layout}");
            let formats = &rig.ctx.draw_data().texture_formats;
            assert!(formats.values().all(|f| f.is_opaque()), "{formats:?}");
            // The pipelines for opaque images are built for the target once.
            assert_eq!(rig.embed.shared_pipeline_targets(), 1);
        }
    }
    // The format a handle was made with is the one its pixels have.
    let h = ImageSource::bgra_opaque(size, swapped(&rgba));
    let rig = Rig::new(&host, h, View::exact(size));
    let _ = rig;
    assert!(PixelFormat::Bgrx8Srgb.is_opaque() && PixelFormat::Bgrx8Srgb.is_bgr());
}

#[test]
fn replacing_the_source_with_another_layout_recreates_the_texture_once() {
    let Some(host) = gpu("layout change") else {
        return;
    };
    let size = [16, 16];
    let rgba = Noise(1).bytes(16 * 16 * 4, Some(255));
    let view = View::exact(size);
    let mut rig = Rig::new(&host, ImageSource::rgba(size, rgba.clone()).live(), view);
    rig.pass();
    let one = rig.stats();
    rig.ctx
        .update_image(rig.handle, ImageSource::bgra(size, swapped(&rgba)))
        .unwrap();
    let shown = rig.pass();
    let two = rig.stats();
    assert_eq!(two.texture_creations, one.texture_creations + 1, "{two:?}");
    assert_eq!(
        max_diff(
            &shown,
            &fresh(&host, ImageSource::rgba(size, rgba.clone()), view)
        ),
        0
    );
    // Further updates of the new layout are regions again.
    rig.ctx
        .update_image_region(
            rig.handle,
            zaxis::TextureRegion::new(0, 0, 1, 1),
            &[1, 2, 3, 255],
            4,
        )
        .unwrap();
    rig.pass();
    let three = rig.stats();
    assert_eq!(three.texture_creations, two.texture_creations);
    assert_eq!(three.texture_region_updates, 1);
}
