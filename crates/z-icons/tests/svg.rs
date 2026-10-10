use core::fmt::{self, Write};
use resvg::{tiny_skia, usvg};
use z_icons::{Icon, USER};

fn rasterize(svg: &[u8]) -> tiny_skia::Pixmap {
    let tree = usvg::Tree::from_data(svg, &usvg::Options::default()).unwrap();
    let mut pixels = tiny_skia::Pixmap::new(
        tree.size().width().ceil() as u32,
        tree.size().height().ceil() as u32,
    )
    .unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    pixels
}

#[test]
fn raw_data_remains_static_and_tintable() {
    const BYTES: &[u8] = USER.svg();
    const TEXT: &str = USER.svg_str();
    assert_eq!(USER.name(), "user");
    assert_eq!(BYTES, TEXT.as_bytes());
    assert_eq!(BYTES.as_ptr(), TEXT.as_ptr());
    let pixels = rasterize(BYTES);
    let drawn: Vec<_> = pixels.pixels().iter().filter(|p| p.alpha() > 0).collect();
    assert!(!drawn.is_empty());
    assert!(drawn
        .iter()
        .all(|p| p.red() == p.alpha() && p.green() == p.alpha() && p.blue() == p.alpha()));
}

#[test]
fn configured_svg_changes_real_pixels_and_intrinsic_size() {
    let svg = USER
        .render()
        .size(48, 32)
        .color("#ff0000")
        .stroke_width(3.0)
        .to_string();
    let doc = usvg::roxmltree::Document::parse(&svg).unwrap();
    let root = doc.root_element();
    assert_eq!(root.attribute("viewBox"), Some("0 0 24 24"));
    assert_eq!(root.attribute("stroke-width"), Some("3"));
    let pixels = rasterize(svg.as_bytes());
    assert_eq!((pixels.width(), pixels.height()), (48, 32));
    let drawn: Vec<_> = pixels.pixels().iter().filter(|p| p.alpha() > 0).collect();
    assert!(!drawn.is_empty());
    assert!(drawn
        .iter()
        .all(|p| p.red() == p.alpha() && p.green() == 0 && p.blue() == 0));
    let invisible = USER.render().stroke_width(0.0).to_string();
    assert!(rasterize(invisible.as_bytes())
        .pixels()
        .iter()
        .all(|p| p.alpha() == 0));
}

#[test]
fn current_color_inherits_from_inline_svg_css() {
    let svg = USER.render().to_string();
    let svg = svg.replacen("<svg ", "<svg color=\"#00ff00\" ", 1);
    let pixels = rasterize(svg.as_bytes());
    let drawn: Vec<_> = pixels.pixels().iter().filter(|p| p.alpha() > 0).collect();
    assert!(!drawn.is_empty());
    assert!(drawn
        .iter()
        .all(|p| p.green() == p.alpha() && p.red() == 0 && p.blue() == 0));
}

#[test]
fn bundled_filled_dots_use_the_selected_color_and_custom_fills_are_preserved() {
    let svg = z_icons::PALETTE.render().color("#0000ff").to_string();
    let doc = usvg::roxmltree::Document::parse(&svg).unwrap();
    let dots: Vec<_> = doc
        .descendants()
        .filter(|node| node.has_tag_name("circle"))
        .collect();
    assert_eq!(dots.len(), 4);
    assert!(dots
        .iter()
        .all(|node| node.attribute("fill") == Some("#0000ff")));
    let pixels = rasterize(svg.as_bytes());
    assert!(pixels
        .pixels()
        .iter()
        .all(|p| p.red() == 0 && p.green() == 0));

    static CUSTOM: Icon = Icon::new(
        "custom-fill",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4" fill="#fff" stroke="red"/></svg>"##,
    );
    let svg = CUSTOM.render().color("blue").to_string();
    let doc = usvg::roxmltree::Document::parse(&svg).unwrap();
    let circle = doc.root_element().first_element_child().unwrap();
    assert_eq!(circle.attribute("fill"), Some("#fff"));
    assert_eq!(circle.attribute("stroke"), Some("red"));
}

#[test]
fn colors_cannot_inject_xml_attributes_or_nodes() {
    let color = "red\" onload=\"evil()'><script>& Привет";
    let svg = USER.render().color(color).to_string();
    let doc = usvg::roxmltree::Document::parse(&svg).unwrap();
    assert_eq!(doc.root_element().attribute("stroke"), Some(color));
    assert!(doc.root_element().attribute("onload").is_none());
    assert!(!doc.descendants().any(|node| node.has_tag_name("script")));
}

#[test]
fn custom_svg_preserves_geometry_and_unrelated_root_attributes() {
    static CUSTOM: Icon = Icon::new(
        "custom",
        "  <svg xmlns='http://www.w3.org/2000/svg' width = '10' height='20' viewBox='0 0 10 20' stroke='#fff' stroke-width='1.5' data-note='a > b'><path d='M1 1L9 19'/></svg>",
    );
    let svg = CUSTOM.render().size(20, 40).color("blue").to_string();
    let doc = usvg::roxmltree::Document::parse(&svg).unwrap();
    let root = doc.root_element();
    assert_eq!(root.attribute("data-note"), Some("a > b"));
    assert_eq!(root.attribute("viewBox"), Some("0 0 10 20"));
    assert_eq!(root.attribute("stroke-width"), Some("1.5"));
    assert_eq!(root.attribute("width"), Some("20"));
    assert_eq!(
        root.first_element_child().unwrap().attribute("d"),
        Some("M1 1L9 19")
    );
    let pixels = rasterize(svg.as_bytes());
    assert!(pixels.pixels().iter().any(|p| p.blue() > 0));
    let original_size = CUSTOM.render().to_string();
    let doc = usvg::roxmltree::Document::parse(&original_size).unwrap();
    assert_eq!(doc.root_element().attribute("width"), Some("10"));
    assert_eq!(doc.root_element().attribute("height"), Some("20"));
}

struct Buffer {
    bytes: [u8; 1024],
    len: usize,
    capacity: usize,
}

impl Write for Buffer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.len + text.len();
        if end > self.capacity {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[test]
fn supports_allocation_free_output_and_propagates_writer_failure() {
    let svg = USER.render().color("#123456");
    let mut buffer = Buffer {
        bytes: [0; 1024],
        len: 0,
        capacity: 1024,
    };
    svg.write_to(&mut buffer).unwrap();
    assert_eq!(&buffer.bytes[..buffer.len], svg.to_string().as_bytes());
    buffer.len = 0;
    buffer.capacity = 10;
    assert_eq!(svg.write_to(&mut buffer), Err(fmt::Error));
    assert_eq!(
        Icon::new("invalid", "<svgx />")
            .render()
            .write_to(&mut String::new()),
        Err(fmt::Error)
    );
}

#[test]
fn every_bundled_asset_parses_and_rasterizes() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("icons");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let source = std::fs::read(&path).unwrap();
        assert!(
            rasterize(&source).pixels().iter().any(|p| p.alpha() > 0),
            "{path:?} is blank"
        );
        count += 1;
    }
    assert_eq!(count, 1866);
}

#[test]
#[should_panic(expected = "SVG dimensions must be positive")]
fn rejects_zero_size() {
    let _ = USER.render().size(0, 24);
}

#[test]
fn rejects_invalid_stroke_widths() {
    for width in [-1.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        assert!(std::panic::catch_unwind(|| USER.render().stroke_width(width)).is_err());
    }
}
