use super::*;
pub(super) fn rgba(side: usize, seed: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(side * side * 4);
    for y in 0..side {
        for x in 0..side {
            data.extend([
                ((x % 64) * 4) as u8 ^ seed,
                ((y % 64) * 4) as u8,
                ((x % 64).wrapping_mul(17) ^ (y % 64).wrapping_mul(13)) as u8,
                if (x + y) % 8 == 0 { 96 } else { 255 },
            ]);
        }
    }
    data
}
pub(super) fn encode(side: usize, rgba: &[u8], format: image::ImageFormat) -> Vec<u8> {
    if format == image::ImageFormat::Png {
        use image::{
            codecs::png::{CompressionType, FilterType, PngEncoder},
            ImageEncoder,
        };
        let mut output = Vec::new();
        PngEncoder::new_with_quality(&mut output, CompressionType::Best, FilterType::Adaptive)
            .write_image(
                rgba,
                side as u32,
                side as u32,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        assert!(
            output.len() <= 16 << 20,
            "benchmark PNG exceeds encoded budget"
        );
        return output;
    }
    let image = image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(side as u32, side as u32, rgba.to_vec()).unwrap(),
    );
    let image = if format == image::ImageFormat::Jpeg {
        image::DynamicImage::ImageRgb8(image.into_rgb8())
    } else {
        image
    };
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, format).unwrap();
    output.into_inner()
}
pub(super) fn svg(side: usize, paths: usize) -> String {
    let mut svg=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{side}\" height=\"{side}\" viewBox=\"0 0 100 100\">");
    for i in 0..paths {
        svg.push_str(&format!(
            "<path d=\"M{} {}q20 -15 15 15t-15 15Z\" fill=\"#{:06x}\" fill-opacity=\".5\"/>",
            i % 80,
            (i * 17) % 80,
            (i * 7919) % 0xffffff
        ));
    }
    svg.push_str("</svg>");
    svg
}
