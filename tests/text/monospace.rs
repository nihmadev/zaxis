//! Monospace cells, tabular figures and the family-aware layout cache.
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;
use zaxis::text::*;
use zaxis::{FontFamily, FontWeight};

const SIZE: f32 = 16.0;
const MONO: TextFont = TextFont {
    weight: FontWeight::REGULAR,
    family: TextFamily::Monospace,
    tabular: false,
};

fn system() -> TextSystem {
    let mut text = TextSystem::new(FontFamily::inter());
    text.begin_frame();
    text
}

fn width(text: &mut TextSystem, s: &str, font: impl Into<TextFont>) -> f32 {
    text.measure(s, SIZE, font, f32::INFINITY).x
}

fn cells(text: &mut TextSystem) -> f32 {
    text.monospace_metrics(SIZE, FontWeight::REGULAR).cell_width
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn ascii_strings_of_equal_length_are_equally_wide() {
    let mut text = system();
    let cell = cells(&mut text);
    assert!(cell > 0.0);
    let widths: Vec<f32> = ["iiiiiiii", "WWWWWWWW", "00000000", "a1 {}-_.", "        "]
        .iter()
        .map(|s| width(&mut text, s, MONO))
        .collect();
    for w in &widths {
        assert!(near(*w, 8.0 * cell), "{w} vs {}", 8.0 * cell);
    }
    // The same strings are not equally wide in the proportional family.
    let (narrow, wide) = (
        width(&mut text, "iiiiiiii", FontWeight::REGULAR),
        width(&mut text, "WWWWWWWW", FontWeight::REGULAR),
    );
    assert!(wide > narrow * 1.5);
}

#[test]
fn metrics_come_from_the_layout_that_paints() {
    let mut text = system();
    let metrics = text.monospace_metrics(SIZE, FontWeight::REGULAR);
    let layout = text.layout("0123456789", SIZE, MONO, f32::INFINITY);
    assert!(near(layout.size.x, 10.0 * metrics.cell_width));
    assert_eq!(layout.size.y, metrics.line_height);
    let two_lines = text.layout("a\nb", SIZE, MONO, f32::INFINITY);
    assert!(near(two_lines.size.y, 2.0 * metrics.line_height));
}

#[test]
fn primary_font_glyphs_sit_on_the_grid_and_fallback_glyphs_keep_their_real_width() {
    let mut text = system();
    let cell = cells(&mut text);
    for s in ["fn main() { 0x1F }", "iiiiWWWW  ~~"] {
        let layout = text.layout(s, SIZE, MONO, f32::INFINITY);
        for (glyph, _) in &layout.glyphs {
            let n = glyph.x / cell;
            assert!(near(n, n.round()), "{s:?}: glyph at {n} cells");
        }
        assert!(near(layout.size.x, s.chars().count() as f32 * cell));
    }
    // CJK, Cyrillic, color emoji, a ZWJ sequence, a flag and combining marks: the width is
    // the shaped width, not a character count, and nothing overlaps or leaves a gap.
    for s in [
        "abc日本語def",
        "Привет мир",
        "x😀y",
        "👨‍👩‍👧‍👦",
        "🇷🇺",
        "e\u{301}a\u{308}",
    ] {
        let layout = text.layout(s, SIZE, MONO, f32::INFINITY);
        let end = layout
            .glyphs
            .iter()
            .map(|(g, _)| g.x + g.w)
            .fold(0.0, f32::max);
        assert!(
            near(layout.size.x, end),
            "{s:?}: {} vs {end}",
            layout.size.x
        );
        let mut previous = 0.0;
        for (glyph, _) in &layout.glyphs {
            assert!(glyph.x + 1e-3 >= previous, "{s:?}");
            previous = glyph.x;
        }
        let clusters = &layout.lines[0].clusters;
        assert_eq!(clusters.len(), s.graphemes(true).count(), "{s:?}");
        assert!(
            clusters.windows(2).all(|w| w[0].x1 <= w[1].x0 + 1e-3),
            "{s:?}"
        );
    }
    // ASCII before a fallback glyph stays on the grid; what follows it is shifted by the
    // difference between the fallback advance and the cells it replaces.
    let layout = text.layout("abc日def", SIZE, MONO, f32::INFINITY);
    let wide = layout.lines[0].clusters[3];
    assert!(near(wide.x0, 3.0 * cell) && wide.x1 > wide.x0);
    let tail = layout.lines[0].clusters[4].x0;
    assert!(near(tail, wide.x1));
    assert!(
        width(&mut text, "x😀y", MONO) > 2.0 * cell,
        "an emoji is wider than a cell"
    );
    // A combining mark adds no cell and the cluster stays one grapheme.
    let combined = text.layout("e\u{301}", SIZE, MONO, f32::INFINITY);
    assert!(near(combined.size.x, cell));
    assert_eq!(combined.lines[0].clusters.len(), 1);
}

#[test]
fn tab_stops_are_counted_in_cells_even_when_a_tab_starts_on_a_stop() {
    let mut text = system();
    // cosmic-text alone collapses a tab that starts exactly on a stop to zero width at
    // many sizes; every column of every size must land on the next stop.
    for size in [12.0_f32, 13.0, 14.0, 16.0, 17.0, 20.0] {
        let cell = text.monospace_metrics(size, FontWeight::REGULAR).cell_width;
        for tab in [1_u16, 2, 4, 8] {
            for n in 0..20_usize {
                let s = format!("{}\tb", "a".repeat(n));
                let layout = text.layout_with_tab(&s, size, MONO, f32::INFINITY, tab);
                let x = layout.lines[0].clusters.last().unwrap().x0 / cell;
                let stop = f32::from(tab);
                let want = ((n as f32 / stop).floor() + 1.0) * stop;
                assert!(
                    (x - want).abs() < 1e-2,
                    "size {size} tab {tab} after {n}: {x} vs {want}"
                );
                assert!((layout.size.x / cell - (want + 1.0)).abs() < 1e-2);
            }
        }
    }
    // Several tabs, and a tab that follows another.
    let layout = text.layout_with_tab("\t\tx\ty", SIZE, MONO, f32::INFINITY, 4);
    let cell = cells(&mut text);
    let xs: Vec<f32> = layout.lines[0]
        .clusters
        .iter()
        .map(|c| c.x0 / cell)
        .collect();
    for (got, want) in xs.iter().zip([0.0, 4.0, 8.0, 9.0, 12.0]) {
        assert!((got - want).abs() < 1e-2, "{xs:?}");
    }
    // The same text in the proportional family keeps cosmic's own result.
    let plain = text.layout_with_tab("a\tb", SIZE, FontWeight::REGULAR, f32::INFINITY, 4);
    assert!(plain.size.x > 0.0);
}

#[test]
fn tabular_figures_equalize_digits_when_the_font_has_them() {
    let mut text = system();
    let tabular = TextFont::new(FontWeight::REGULAR, TextFamily::Proportional, true);
    let plain = TextFont::from(FontWeight::REGULAR);
    let [one, zero, eight] =
        ["1111111", "0000000", "8888888"].map(|s| width(&mut text, s, tabular));
    assert!(near(one, zero) && near(zero, eight), "{one} {zero} {eight}");
    let proportional = ["1111111", "0000000"].map(|s| width(&mut text, s, plain));
    assert!(
        proportional[0] < proportional[1] - 1.0,
        "Inter's default figures are proportional: {proportional:?}"
    );
    // Rows with different digits have the same width, so right edges align.
    let rows = ["1,234.50", "9,876.43", "1,111.11"].map(|s| width(&mut text, s, tabular));
    assert!(near(rows[0], rows[1]) && near(rows[1], rows[2]), "{rows:?}");
}

#[test]
fn tabular_figures_leave_letters_and_monospace_alone() {
    let mut text = system();
    let tabular = TextFont::new(FontWeight::REGULAR, TextFamily::Proportional, true);
    let plain = TextFont::from(FontWeight::REGULAR);
    assert!(near(
        width(&mut text, "Total", tabular),
        width(&mut text, "Total", plain)
    ));
    // Monospace digits are already tabular: the request normalizes to one cache entry.
    let mono = TextFont::new(FontWeight::REGULAR, TextFamily::Monospace, true);
    assert_eq!(mono, MONO);
    assert_eq!(text.font_key(mono), text.font_key(MONO));
}

#[test]
fn layout_cache_is_keyed_by_family_and_figures() {
    let mut text = system();
    let plain = text.layout("Same words", SIZE, FontWeight::REGULAR, f32::INFINITY);
    let mono = text.layout("Same words", SIZE, MONO, f32::INFINITY);
    let tabular = text.layout(
        "Same words",
        SIZE,
        TextFont::new(FontWeight::REGULAR, TextFamily::Proportional, true),
        f32::INFINITY,
    );
    assert!(!Arc::ptr_eq(&plain, &mono) && !Arc::ptr_eq(&plain, &tabular));
    assert_ne!(mono.size.x, plain.size.x);
    let built = text.builds;
    assert_eq!(built, 3);
    // Each is served from the cache afterwards, in any order.
    assert!(Arc::ptr_eq(
        &mono,
        &text.layout("Same words", SIZE, MONO, f32::INFINITY)
    ));
    assert!(Arc::ptr_eq(
        &plain,
        &text.layout("Same words", SIZE, FontWeight::REGULAR, f32::INFINITY)
    ));
    assert_eq!(text.builds, built);
    // Another size is another layout; the cell width is measured once per size.
    text.layout("Same words", SIZE + 2.0, MONO, f32::INFINITY);
    assert_eq!(text.builds, built + 1);
    assert_ne!(
        text.font_key(MONO),
        text.font_key(FontWeight::REGULAR),
        "the atlas key names the face, so glyphs of the two families never share entries"
    );
}

#[test]
fn plain_text_is_unchanged_by_the_family_machinery() {
    let mut text = system();
    for s in ["Hello, world", "AV kerning", "Привет", "fi ffl 12:30"] {
        let a = text.layout(s, SIZE, FontWeight::REGULAR, 120.0);
        let b = text.layout(s, SIZE, TextFont::from(FontWeight::REGULAR), 120.0);
        assert!(Arc::ptr_eq(&a, &b));
        let key = text.font_key(FontWeight::REGULAR);
        assert!(!key.monospace && !key.tabular);
        let again = text.build_layout(s, SIZE, key, 120.0);
        assert_eq!(again.size, a.size);
        assert_eq!(again.glyphs.len(), a.glyphs.len());
        for ((x, _), (y, _)) in again.glyphs.iter().zip(&a.glyphs) {
            assert_eq!((x.x, x.w, x.glyph_id), (y.x, y.w, y.glyph_id));
        }
    }
}

#[test]
fn unwrapped_monospace_lines_are_never_cut_by_layout() {
    let mut text = system();
    let cell = cells(&mut text);
    let long = "x".repeat(400);
    let layout = text.layout(&long, SIZE, MONO, f32::INFINITY);
    assert_eq!(layout.lines.len(), 1);
    assert!((layout.size.x / cell - 400.0).abs() < 0.01);
    // A wrapping width breaks the same text into several lines; carets stay on the grid.
    let wrapped = text.layout("alpha beta gamma delta", SIZE, MONO, 8.0 * cell);
    assert!(wrapped.lines.len() > 1);
    for line in &wrapped.lines {
        for c in &line.clusters {
            let n = c.x0 / cell;
            assert!(near(n, n.round()));
        }
    }
}

#[test]
fn caret_and_hit_positions_use_the_same_cells() {
    let mut text = system();
    let cell = cells(&mut text);
    let carets = text.carets("ab日c", SIZE, MONO);
    assert_eq!(carets.first().copied(), Some((0, 0.0)));
    let mut previous = 0.0;
    for (_, x) in &carets {
        assert!(*x >= previous);
        previous = *x;
    }
    // Before the fallback glyph the carets are the cells of the primary font.
    assert!(near(carets[1].1, cell) && near(carets[2].1, 2.0 * cell));
    let layout = text.layout("abcdef", SIZE, MONO, f32::INFINITY);
    let line = &layout.lines[0];
    // Halfway through a cell picks the nearer boundary; the cell under x starts at its glyph.
    let (nearest, cell_start) = line.hit(2.0 * cell + 0.3 * cell);
    assert_eq!((nearest, cell_start), (2, 2));
    let (nearest, cell_start) = line.hit(2.0 * cell + 0.7 * cell);
    assert_eq!((nearest, cell_start), (3, 2));
    let spans = line.spans(1..4);
    assert_eq!(spans.len(), 1);
    assert!(near(spans[0].0, cell) && near(spans[0].1, 4.0 * cell));
}
