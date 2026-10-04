#![cfg(feature = "bundled-icons")]

use resvg::{tiny_skia, usvg};
use zaxis::{icons, ImageSource};

fn render(svg: &[u8]) -> tiny_skia::Pixmap {
    let tree = usvg::Tree::from_data(svg, &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(48, 48).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(2.0, 2.0),
        &mut pixmap.as_mut(),
    );
    pixmap
}

#[test]
fn icon_is_a_white_stroke_ready_for_tinting() {
    assert_eq!(icons::USER.name(), "user");
    let pixmap = render(icons::USER.svg());
    let drawn: Vec<_> = pixmap.pixels().iter().filter(|p| p.alpha() > 0).collect();
    assert!(!drawn.is_empty());
    assert!(drawn
        .iter()
        .all(|p| p.red() == p.alpha() && p.blue() == p.alpha()));
    let _ = ImageSource::from(&icons::USER);
}

#[test]
fn every_bundled_svg_parses_and_draws() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/z-icons/icons");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let pixmap = render(&std::fs::read(&path).unwrap());
        assert!(
            pixmap.pixels().iter().any(|p| p.alpha() > 0),
            "{path:?} is blank"
        );
        count += 1;
    }
    assert!(count > 1500);
}
