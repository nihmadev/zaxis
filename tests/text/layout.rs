use ab_glyph::{point, Font, FontVec, Glyph};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;
use zaxis::shapes::Mesh;
use zaxis::text::{layout::CachedLayout, *};
use zaxis::{Color, FontFamily, FontWeight, Id, Vec2};

#[test]
fn resizing_unwrapped_text_reuses_layout_until_a_real_line_break() {
    let mut text = text();
    text.begin_frame();
    let original = text.layout("Short caption", 16.0, FontWeight::REGULAR, 300.0);
    for width in [300.375, 280.25, 260.75, 220.5] {
        assert!(Arc::ptr_eq(
            &original,
            &text.layout("Short caption", 16.0, FontWeight::REGULAR, width)
        ));
        assert_eq!(
            text.measure_with_wrap("Short caption", 16.0, FontWeight::REGULAR, width)
                .1,
            f32::INFINITY
        );
    }
    let wrapped = text.layout("Short caption", 16.0, FontWeight::REGULAR, 25.0);
    assert!(!Arc::ptr_eq(&original, &wrapped));
    assert!(wrapped.size.y > original.size.y);
    assert_eq!(
        text.measure_with_wrap("Short caption", 16.0, FontWeight::REGULAR, 25.0)
            .1,
        25.0
    );
    let changed = text.layout("Another caption", 16.0, FontWeight::REGULAR, 300.0);
    assert!(!Arc::ptr_eq(&original, &changed));
    let rtl = text.layout("مرحبا بالعالم", 16.0, FontWeight::REGULAR, 300.0);
    assert!(!rtl.width_independent);
    assert!(!Arc::ptr_eq(
        &rtl,
        &text.layout("مرحبا بالعالم", 16.0, FontWeight::REGULAR, 310.0)
    ));
}

fn text() -> TextSystem {
    TextSystem::new(FontFamily::inter())
}

#[test]
fn measured_layout_is_shared_by_paint_and_live_frames_only() {
    let mut text = text();
    text.begin_frame();
    let caption = "AV kerning\nUnicode λ and wrapped words";
    let size = text.measure(caption, 16.0, FontWeight::REGULAR, 90.0);
    let layout = text.layout(caption, 16.0, FontWeight::REGULAR, 90.0);
    assert_eq!(layout.size, size);
    assert!(Arc::ptr_eq(
        &layout,
        &text.layout(caption, 16.0, FontWeight::REGULAR, 90.0)
    ));
    let color = Color::rgba(150, 80, 200, 120);
    let mut mesh = Mesh::default();
    text.paint(
        &mut mesh,
        caption,
        Vec2::ZERO,
        16.0,
        FontWeight::REGULAR,
        90.0,
        color,
        1.5,
    );
    assert!(Arc::ptr_eq(
        &layout,
        &text.layout(caption, 16.0, FontWeight::REGULAR, 90.0)
    ));
    assert!(!mesh.vertices.is_empty());
    assert!(mesh.vertices.iter().all(|v| v.color == color.linear()));
    assert!(!Arc::ptr_eq(
        &layout,
        &text.layout(caption, 16.0, FontWeight::REGULAR, 180.0)
    ));
    assert!(!Arc::ptr_eq(
        &layout,
        &text.layout(caption, 20.0, FontWeight::REGULAR, 90.0)
    ));
    text.end_frame();
    text.begin_frame();
    assert!(Arc::ptr_eq(
        &layout,
        &text.layout(caption, 16.0, FontWeight::REGULAR, 90.0)
    ));
    text.end_frame();
    assert_eq!(text.layouts.len(), 1);
    text.begin_frame();
    text.measure("replacement", 16.0, FontWeight::REGULAR, 90.0);
    text.end_frame();
    assert_eq!(text.layouts.len(), 1);
    assert!(!Arc::ptr_eq(
        &layout,
        &text.layout(caption, 16.0, FontWeight::REGULAR, 90.0)
    ));
}

#[test]
fn cache_key_collision_cannot_substitute_another_caption() {
    let mut text = text();
    let font = text.font_key(FontWeight::REGULAR);
    let wanted = text.build_layout("AV", 16.0, font, f32::INFINITY);
    text.layouts.insert(
        Id::new((
            "AV",
            16.0_f32.to_bits(),
            f32::INFINITY.to_bits(),
            font,
            8_u16,
        )),
        CachedLayout {
            text: "wrong".into(),
            size: 16.0_f32.to_bits(),
            wrap: f32::INFINITY.to_bits(),
            font,
            tab: 8,
            layout: Arc::new(text.build_layout("wrong", 16.0, font, f32::INFINITY)),
            last_frame: 0,
        },
    );
    let layout = text.layout("AV", 16.0, FontWeight::REGULAR, f32::INFINITY);
    assert_eq!(layout.carets, wanted.carets);
    assert_eq!(layout.glyphs.len(), wanted.glyphs.len());
    assert_eq!(layout.size, wanted.size);
}

#[test]
#[cfg(feature = "bundled-emoji")]
fn unicode_clusters_use_real_glyphs_and_color_emoji_at_multiple_scales() {
    let mut text = text();
    for sample in ["е\u{301}", "👩‍💻", "🇷🇺", "👍🏽"] {
        let layout = text.layout(sample, 16.0, FontWeight::REGULAR, f32::INFINITY);
        assert!(
            layout.glyphs.iter().all(|(g, _)| g.glyph_id != 0),
            "{sample}"
        );
        if sample == "е\u{301}" {
            // A combining accent can be a separate positioned glyph in a
            // valid font; the grapheme still has only its two caret edges.
            assert!((1..=2).contains(&layout.glyphs.len()));
        } else {
            assert_eq!(layout.glyphs.len(), 1, "{sample} must shape as one glyph");
        }
        assert_eq!(layout.carets.len(), 2, "{sample} has one grapheme");
        assert_eq!(layout.carets[0], (0, 0.0));
        assert!((layout.carets[1].1 - layout.size.x).abs() < 0.001);
        for scale in [1.0, 1.5, 2.0] {
            let mut mesh = Mesh::default();
            text.paint(
                &mut mesh,
                sample,
                Vec2::ZERO,
                16.0,
                FontWeight::REGULAR,
                f32::INFINITY,
                Color::WHITE,
                scale,
            );
            assert!(!mesh.vertices.is_empty());
            if sample != "е\u{301}" {
                assert!(text.has_colored_glyph());
                assert!(text.textures().iter().any(|page| page
                    .pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|p| p[3] != 0 && (p[0] != p[1] || p[1] != p[2]))));
            }
        }
    }
}

#[test]
fn caret_positions_follow_shaped_clusters_without_skipping_adjacent_letters() {
    let mut text = text();
    let layout = text.layout(
        "AV ffi е\u{301}👩‍💻",
        16.0,
        FontWeight::REGULAR,
        f32::INFINITY,
    );
    assert!(layout.carets.windows(2).all(|pair| pair[0].1 <= pair[1].1));
    assert!(layout.carets[1].1 > 0.0);
    let v = layout.glyphs.iter().find(|(g, _)| g.start == 1).unwrap();
    assert!((layout.carets[1].1 - v.0.x).abs() < 0.001);
    assert!(layout.carets.iter().all(|(i, _)| "AV ffi е\u{301}👩‍💻"
        .grapheme_indices(true)
        .any(|(start, _)| start == *i)
        || *i == "AV ffi е\u{301}👩‍💻".len()));
}

#[test]
fn centered_field_baseline_matches_the_fonts_cap_and_descender_band() {
    let mut text = text();
    let font =
        FontVec::try_from_vec(include_bytes!("../../assets/Inter-Regular.ttf").to_vec()).unwrap();
    for size in [13.0, 14.0, 20.0] {
        let layout = text.layout("Привет", size, FontWeight::REGULAR, f32::INFINITY);
        let em_scale = size * font.height_unscaled() / font.units_per_em().unwrap();
        let h = font
            .outline_glyph(Glyph {
                id: font.glyph_id('H'),
                scale: em_scale.into(),
                position: point(0.0, 0.0),
            })
            .unwrap()
            .px_bounds();
        let g = font
            .outline_glyph(Glyph {
                id: font.glyph_id('g'),
                scale: em_scale.into(),
                position: point(0.0, 0.0),
            })
            .unwrap()
            .px_bounds();
        let offset = text.centered_line_offset("Привет", size, FontWeight::REGULAR);
        let top = offset + layout.baseline + h.min.y;
        let bottom = offset + layout.baseline + g.max.y;
        assert!((top + bottom - layout.size.y).abs() < 0.001);
        let placeholder = text.centered_line_offset("Placeholder", size, FontWeight::REGULAR);
        assert!((offset - placeholder).abs() < 0.001);
    }
}
