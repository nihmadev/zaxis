use super::*;
use unicode_segmentation::UnicodeSegmentation;

#[test]
fn resizing_unwrapped_text_reuses_layout_until_a_real_line_break() {
    let mut text = text();
    text.begin_frame();
    let original = text.layout("Short caption", 16.0, 300.0);
    for width in [300.375, 280.25, 260.75, 220.5] {
        assert!(Arc::ptr_eq(
            &original,
            &text.layout("Short caption", 16.0, width)
        ));
        assert_eq!(
            text.measure_with_wrap("Short caption", 16.0, width).1,
            f32::INFINITY
        );
    }
    let wrapped = text.layout("Short caption", 16.0, 25.0);
    assert!(!Arc::ptr_eq(&original, &wrapped));
    assert!(wrapped.size.y > original.size.y);
    assert_eq!(text.measure_with_wrap("Short caption", 16.0, 25.0).1, 25.0);
    let changed = text.layout("Another caption", 16.0, 300.0);
    assert!(!Arc::ptr_eq(&original, &changed));
    let rtl = text.layout("مرحبا بالعالم", 16.0, 300.0);
    assert!(!rtl.width_independent);
    assert!(!Arc::ptr_eq(
        &rtl,
        &text.layout("مرحبا بالعالم", 16.0, 310.0)
    ));
}

fn text() -> TextSystem {
    TextSystem::new(
        FontArc::try_from_slice(include_bytes!("../../assets/Lato-Regular.ttf")).unwrap(),
    )
}

#[test]
fn measured_layout_is_shared_by_paint_and_live_frames_only() {
    let mut text = text();
    text.begin_frame();
    let caption = "AV kerning\nUnicode λ and wrapped words";
    let size = text.measure(caption, 16.0, 90.0);
    let layout = text.layout(caption, 16.0, 90.0);
    assert_eq!(layout.size, size);
    assert!(Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 90.0)));
    let color = Color::rgba(150, 80, 200, 120);
    let mut mesh = Mesh::default();
    text.paint(&mut mesh, caption, Vec2::ZERO, 16.0, 90.0, color, 1.5);
    assert!(Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 90.0)));
    assert!(!mesh.vertices.is_empty());
    assert!(mesh.vertices.iter().all(|v| v.color == color.linear()));
    assert!(!Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 180.0)));
    assert!(!Arc::ptr_eq(&layout, &text.layout(caption, 20.0, 90.0)));
    text.end_frame();
    text.begin_frame();
    assert!(Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 90.0)));
    text.end_frame();
    assert_eq!(text.layouts.len(), 1);
    text.begin_frame();
    text.measure("replacement", 16.0, 90.0);
    text.end_frame();
    assert_eq!(text.layouts.len(), 1);
    assert!(!Arc::ptr_eq(&layout, &text.layout(caption, 16.0, 90.0)));
}

#[test]
fn cache_key_collision_cannot_substitute_another_caption() {
    let mut text = text();
    let wanted = text.build_layout("AV", 16.0, f32::INFINITY);
    text.layouts.insert(
        Id::new(("AV", 16.0_f32.to_bits(), f32::INFINITY.to_bits())),
        CachedLayout {
            text: "wrong".into(),
            size: 16.0_f32.to_bits(),
            wrap: f32::INFINITY.to_bits(),
            layout: Arc::new(text.build_layout("wrong", 16.0, f32::INFINITY)),
            last_frame: 0,
        },
    );
    let layout = text.layout("AV", 16.0, f32::INFINITY);
    assert_eq!(layout.carets, wanted.carets);
    assert_eq!(layout.glyphs.len(), wanted.glyphs.len());
    assert_eq!(layout.size, wanted.size);
}

#[test]
#[cfg(feature = "bundled-emoji")]
fn unicode_clusters_use_real_glyphs_and_color_emoji_at_multiple_scales() {
    let mut text = text();
    for sample in ["е\u{301}", "👩‍💻", "🇷🇺", "👍🏽"] {
        let layout = text.layout(sample, 16.0, f32::INFINITY);
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
                f32::INFINITY,
                Color::WHITE,
                scale,
            );
            assert!(!mesh.vertices.is_empty());
            if sample != "е\u{301}" {
                assert!(text.glyphs.values().flatten().any(|g| g.colored));
                assert!(text.pages.iter().any(|page| page
                    .image
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
    let layout = text.layout("AV ffi е\u{301}👩‍💻", 16.0, f32::INFINITY);
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
    for size in [13.0, 14.0, 20.0] {
        let layout = text.layout("Привет", size, f32::INFINITY);
        let em_scale = size * text.font.height_unscaled() / text.font.units_per_em().unwrap();
        let h = text
            .font
            .outline_glyph(Glyph {
                id: text.font.glyph_id('H'),
                scale: em_scale.into(),
                position: point(0.0, 0.0),
            })
            .unwrap()
            .px_bounds();
        let g = text
            .font
            .outline_glyph(Glyph {
                id: text.font.glyph_id('g'),
                scale: em_scale.into(),
                position: point(0.0, 0.0),
            })
            .unwrap()
            .px_bounds();
        let offset = text.centered_line_offset("Привет", size);
        let top = offset + layout.baseline + h.min.y;
        let bottom = offset + layout.baseline + g.max.y;
        assert!((top + bottom - layout.size.y).abs() < 0.001);
        assert!((offset - text.centered_line_offset("Placeholder", size)).abs() < 0.001);
    }
}
