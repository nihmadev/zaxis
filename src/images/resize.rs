use super::{DecodedImage, ImageError, ImageLimits};
use std::sync::Arc;

/// Area downsampling in linear light with premultiplied accumulation. No full-size
/// float intermediate, and transparent RGB cannot contaminate a visible edge.
pub(super) fn downsample(
    input: &DecodedImage,
    size: [u32; 2],
    limits: &ImageLimits,
) -> Result<DecodedImage, ImageError> {
    let bytes = limits.check_size(size)?;
    if size[0] >= input.size[0] || size[1] >= input.size[1] {
        return Ok(input.clone());
    }
    let linear: [f32; 256] = std::array::from_fn(|i| {
        let c = i as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|e| ImageError(e.to_string()))?;
    output.resize(bytes, 0);
    let sx = input.size[0] as f32 / size[0] as f32;
    let sy = input.size[1] as f32 / size[1] as f32;
    for y in 0..size[1] {
        let y0 = y as f32 * sy;
        let y1 = (y + 1) as f32 * sy;
        for x in 0..size[0] {
            let x0 = x as f32 * sx;
            let x1 = (x + 1) as f32 * sx;
            let mut sum = [0.0; 4];
            for iy in y0.floor() as u32..(y1.ceil() as u32).min(input.size[1]) {
                let wy = (y1.min((iy + 1) as f32) - y0.max(iy as f32)).max(0.0);
                for ix in x0.floor() as u32..(x1.ceil() as u32).min(input.size[0]) {
                    let w = wy * (x1.min((ix + 1) as f32) - x0.max(ix as f32)).max(0.0);
                    let p = &input.pixels
                        [((iy as usize * input.size[0] as usize + ix as usize) * 4)..][..4];
                    let a = p[3] as f32 / 255.0 * w;
                    sum[3] += a;
                    for c in 0..3 {
                        sum[c] += linear[p[c] as usize] * a;
                    }
                }
            }
            let p = &mut output[((y as usize * size[0] as usize + x as usize) * 4)..][..4];
            p[3] = (sum[3] / (sx * sy) * 255.0).round().clamp(0.0, 255.0) as u8;
            for c in 0..3 {
                let v = if sum[3] > 0.0 { sum[c] / sum[3] } else { 0.0 };
                let s = if v <= 0.0031308 {
                    v * 12.92
                } else {
                    1.055 * v.powf(1.0 / 2.4) - 0.055
                };
                p[c] = (s * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Ok(DecodedImage {
        size,
        pixels: Arc::new(output),
    })
}
