use cosmic_text::{fontdb, LayoutGlyph};
use std::sync::Arc;
use zaxis::shapes::Mesh;
use zaxis::text::fonts::font_system;
use zaxis::text::*;
use zaxis::{Color, FontFamily, FontWeight, Vec2};

fn text() -> TextSystem {
    TextSystem::new(FontFamily::inter())
}

fn regular_and_bold() -> FontFamily {
    FontFamily::new(include_bytes!("../../assets/Inter-Regular.ttf")).with_weight(
        FontWeight::BOLD,
        include_bytes!("../../assets/Inter-Bold.ttf"),
    )
}

fn face_of(glyph: &LayoutGlyph) -> (String, u16) {
    let fonts = font_system().lock().unwrap();
    let face = fonts.db().face(glyph.font_id).expect("shaped face exists");
    (face.families[0].0.clone(), face.weight.0)
}

#[test]
fn weights_clamp_to_the_css_scale() {
    assert_eq!(FontWeight::new(0), FontWeight::THIN);
    assert_eq!(FontWeight::new(1500), FontWeight::BLACK);
    assert_eq!(FontWeight::from(650).value(), 650);
    assert_eq!(FontWeight::default(), FontWeight::REGULAR);
}

#[test]
fn nearest_face_follows_the_css_matching_order() {
    let family = FontFamily::new(Vec::new()).with_weight(FontWeight::BOLD, Vec::new());
    assert_eq!(family.weights(), [FontWeight::REGULAR, FontWeight::BOLD]);
    let pick = |w| family.resolve(FontWeight::new(w)).value();
    assert_eq!(pick(400), 400);
    assert_eq!(pick(700), 700);
    // 400-500 prefer the lighter face when nothing sits between them.
    assert_eq!(pick(500), 400);
    // Above 500 the heavier face wins; below 400 the lighter one, then heavier.
    assert_eq!(pick(600), 700);
    assert_eq!(pick(900), 700);
    assert_eq!(pick(300), 400);
    assert_eq!(pick(100), 400);
    let light = FontFamily::new(Vec::new())
        .with_weight(FontWeight::LIGHT, Vec::new())
        .with_weight(FontWeight::BOLD, Vec::new());
    assert_eq!(light.resolve(FontWeight::EXTRA_LIGHT), FontWeight::LIGHT);
    assert_eq!(light.resolve(FontWeight::MEDIUM), FontWeight::REGULAR);
    assert_eq!(light.resolve(FontWeight::new(350)), FontWeight::LIGHT);
    // Declaring a weight twice keeps the last file only.
    let replaced = FontFamily::new(Vec::new()).with_weight(FontWeight::REGULAR, Vec::new());
    assert_eq!(replaced.weights(), [FontWeight::REGULAR]);
}

#[test]
#[cfg(feature = "bundled-weights")]
fn each_weight_shapes_with_its_own_font_file() {
    let mut text = text();
    let sample = "Hamburgefonstiv";
    let mut widths = Vec::new();
    for weight in [
        FontWeight::REGULAR,
        FontWeight::MEDIUM,
        FontWeight::SEMIBOLD,
        FontWeight::BOLD,
    ] {
        let layout = text.layout(sample, 16.0, weight, f32::INFINITY);
        let (family, face_weight) = face_of(&layout.glyphs[0].0);
        assert!(family.starts_with("zaxis-"));
        assert_eq!(face_weight, weight.value(), "{weight:?} uses its own file");
        assert_eq!(layout.glyphs[0].0.font_weight.0, weight.value());
        widths.push(layout.size.x);
    }
    // Real bold outlines have wider advances; a thickened Regular would not.
    assert!(
        widths.windows(2).all(|pair| pair[0] < pair[1]),
        "{widths:?}"
    );
}

#[test]
#[cfg(feature = "bundled-weights")]
fn missing_weights_use_the_nearest_file_and_never_fake_bold() {
    let mut text = TextSystem::new(regular_and_bold());
    let regular = text.layout("Hamburg", 16.0, FontWeight::REGULAR, f32::INFINITY);
    let bold = text.layout("Hamburg", 16.0, FontWeight::BOLD, f32::INFINITY);
    assert!(!Arc::ptr_eq(&regular, &bold));
    // 500 resolves to Regular, 600 and above to Bold: the very same layouts.
    let medium = text.layout("Hamburg", 16.0, FontWeight::MEDIUM, f32::INFINITY);
    let semibold = text.layout("Hamburg", 16.0, FontWeight::SEMIBOLD, f32::INFINITY);
    let black = text.layout("Hamburg", 16.0, FontWeight::BLACK, f32::INFINITY);
    assert!(Arc::ptr_eq(&regular, &medium));
    assert!(Arc::ptr_eq(&bold, &semibold));
    assert!(Arc::ptr_eq(&bold, &black));
    assert_eq!(face_of(&semibold.glyphs[0].0).1, 700);
    assert!(semibold
        .glyphs
        .iter()
        .all(|(g, _)| g.cache_key_flags.is_empty()));
}

#[test]
fn layout_keys_separate_weight_size_and_family() {
    let mut text = text();
    let regular = text.layout("Key", 16.0, FontWeight::REGULAR, f32::INFINITY);
    assert!(Arc::ptr_eq(
        &regular,
        &text.layout("Key", 16.0, FontWeight::REGULAR, f32::INFINITY)
    ));
    #[cfg(feature = "bundled-weights")]
    {
        let medium = text.layout("Key", 16.0, FontWeight::MEDIUM, f32::INFINITY);
        assert!(!Arc::ptr_eq(&regular, &medium));
        // 450 resolves to the 500 file, so it shares that layout.
        assert_eq!(
            text.font_key(FontWeight::new(450)),
            text.font_key(FontWeight::MEDIUM)
        );
        assert!(Arc::ptr_eq(
            &medium,
            &text.layout("Key", 16.0, FontWeight::new(450), f32::INFINITY)
        ));
        assert_ne!(
            text.font_key(FontWeight::REGULAR),
            text.font_key(FontWeight::BOLD)
        );
    }
    assert!(!Arc::ptr_eq(
        &regular,
        &text.layout("Key", 17.0, FontWeight::REGULAR, f32::INFINITY)
    ));
    let other = TextSystem::new(regular_and_bold());
    assert_ne!(
        text.font_key(FontWeight::REGULAR).family,
        other.font_key(FontWeight::REGULAR).family
    );
    assert_ne!(
        text.font_key(FontWeight::REGULAR),
        other.font_key(FontWeight::REGULAR)
    );
    assert_eq!(
        text.font_key(FontWeight::REGULAR).style,
        fontdb::Style::Normal
    );
}

#[test]
#[cfg(feature = "bundled-weights")]
fn glyph_cache_keys_name_face_and_weight() {
    let mut text = text();
    let key = |text: &mut TextSystem, weight| {
        let layout = text.layout("H", 16.0, weight, f32::INFINITY);
        layout.glyphs[0].0.physical((0.0, 0.0), 1.0).cache_key
    };
    let regular = key(&mut text, FontWeight::REGULAR);
    let bold = key(&mut text, FontWeight::BOLD);
    assert_ne!(regular, bold);
    assert_ne!(regular.font_id, bold.font_id);
    assert_eq!(regular.font_size_bits, bold.font_size_bits);
    assert_eq!(regular.glyph_id, bold.glyph_id);
}

#[test]
#[cfg(feature = "bundled-weights")]
fn changing_one_weight_rebuilds_only_that_layout() {
    let mut text = text();
    text.begin_frame();
    let a = text.layout("Header", 16.0, FontWeight::REGULAR, f32::INFINITY);
    let b = text.layout("Neighbour", 16.0, FontWeight::REGULAR, f32::INFINITY);
    text.end_frame();
    text.begin_frame();
    let a2 = text.layout("Header", 16.0, FontWeight::SEMIBOLD, f32::INFINITY);
    let b2 = text.layout("Neighbour", 16.0, FontWeight::REGULAR, f32::INFINITY);
    text.end_frame();
    assert!(!Arc::ptr_eq(&a, &a2));
    assert!(Arc::ptr_eq(&b, &b2));
    // The regular layout of the changed text is dropped with the frame.
    assert_eq!(text.layouts.len(), 2);
}

#[test]
#[cfg(feature = "bundled-emoji")]
fn emoji_fallback_follows_every_requested_weight() {
    let mut text = text();
    for weight in [FontWeight::REGULAR, FontWeight::MEDIUM, FontWeight::BOLD] {
        let layout = text.layout("Aa 😀 Привет", 16.0, weight, f32::INFINITY);
        assert!(layout.glyphs.iter().all(|(g, _)| g.glyph_id != 0));
        let emoji = layout
            .glyphs
            .iter()
            .find(|(g, _)| face_of(g).0 == "Noto Color Emoji")
            .expect("emoji comes from the bundled color font");
        let resolved = text.font_key(weight).weight.value();
        assert_eq!(face_of(&emoji.0).1, resolved);
    }
    // A family without bold still lays out emoji, using its nearest weight.
    let mut limited = TextSystem::new(FontFamily::new(include_bytes!(
        "../../assets/Inter-Regular.ttf"
    )));
    let layout = limited.layout("😀 Bold?", 16.0, FontWeight::BOLD, f32::INFINITY);
    assert!(layout.glyphs.iter().all(|(g, _)| g.glyph_id != 0));
}

#[test]
fn unsupported_scripts_at_any_weight_do_not_break_layout() {
    let mut text = text();
    for weight in [FontWeight::THIN, FontWeight::MEDIUM, FontWeight::BLACK] {
        let layout = text.layout("漢字 مرحبا ✓", 16.0, weight, f32::INFINITY);
        assert!(layout.size.x > 0.0 && layout.size.y > 0.0);
    }
}

#[test]
#[cfg(feature = "bundled-weights")]
fn atlas_grows_with_used_weights_only_and_settles() {
    let mut text = text();
    let sample = "Settings Параметры 0123456789 .,:;()";
    let paint = |text: &mut TextSystem, weight: FontWeight, scale: f32| {
        for size in [12.0, 14.0, 16.0, 18.0, 24.0] {
            text.paint(
                &mut Mesh::default(),
                sample,
                Vec2::new(10.0, 10.0),
                size,
                weight,
                f32::INFINITY,
                Color::WHITE,
                scale,
            );
        }
    };
    paint(&mut text, FontWeight::REGULAR, 1.0);
    let (regular_glyphs, regular_pages) = (text.glyph_count(), text.page_count());
    for weight in [FontWeight::MEDIUM, FontWeight::SEMIBOLD, FontWeight::BOLD] {
        paint(&mut text, weight, 1.0);
    }
    // Four weights cost four times the glyphs, nothing more; pages stay few.
    // Subpixel bins differ per weight, so the count is only about four times as large.
    let used = text.glyph_count();
    eprintln!("atlas: {regular_glyphs} glyphs/{regular_pages} pages regular, {used} glyphs/{} pages with 4 weights", text.page_count());
    assert!(used > regular_glyphs * 3 && used <= regular_glyphs * 5);
    assert!(text.page_count() <= regular_pages * 4 && text.page_count() <= 2);
    let settled = (text.glyph_count(), text.page_count());
    let revision: u64 = text.page_revision_sum();
    for weight in [FontWeight::REGULAR, FontWeight::BOLD] {
        paint(&mut text, weight, 1.0);
    }
    assert_eq!((text.glyph_count(), text.page_count()), settled);
    assert_eq!(text.page_revision_sum(), revision);
    // Another DPI rasterizes again, and still within a bounded page count.
    paint(&mut text, FontWeight::BOLD, 1.25);
    assert!(text.page_count() <= 4);
}
