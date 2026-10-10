#![cfg(feature = "catalog")]

use resvg::{tiny_skia, usvg};

#[test]
fn catalog_is_complete_sorted_and_every_name_resolves_to_its_static_icon() {
    let all = z_icons::all();
    assert_eq!(all.len(), 1866);
    assert!(all.windows(2).all(|pair| pair[0].name() < pair[1].name()));
    for icon in all {
        let found = z_icons::get(icon.name()).unwrap();
        assert!(core::ptr::eq(found, *icon));
    }
    assert!(core::ptr::eq(z_icons::get("user").unwrap(), &z_icons::USER));
    for missing in ["", "USER", "arrow_left", "missing-icon"] {
        assert!(z_icons::get(missing).is_none());
    }
}

#[test]
fn every_generated_icon_rasterizes_with_configured_root_attributes() {
    for icon in z_icons::all() {
        let svg = icon.render().size(32, 32).color("#0000ff").to_string();
        let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).unwrap();
        assert_eq!((tree.size().width(), tree.size().height()), (32.0, 32.0));
        let mut pixels = tiny_skia::Pixmap::new(32, 32).unwrap();
        resvg::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        assert!(
            pixels.pixels().iter().any(|p| p.alpha() > 0),
            "{}",
            icon.name()
        );
        assert!(
            pixels
                .pixels()
                .iter()
                .all(|p| p.red() == 0 && p.green() == 0),
            "{} contains pixels outside the configured color",
            icon.name()
        );
    }
}
