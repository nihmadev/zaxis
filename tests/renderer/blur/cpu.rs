//! A CPU reference in linear light: decoded canvases, an exact separable Gaussian with the
//! edge clamped, and the sRGB code-value error measures every blur test shares.

pub fn decode(v: u8) -> f32 {
    let v = f32::from(v) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

pub fn encode(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

/// Linear premultiplied texels, the way the GPU samples the canvas.
#[derive(Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub px: Vec<[f32; 4]>,
}

impl Image {
    pub fn from_srgb8(width: usize, height: usize, pixels: &[[u8; 4]]) -> Self {
        let px = pixels
            .iter()
            .map(|p| {
                [
                    decode(p[0]),
                    decode(p[1]),
                    decode(p[2]),
                    f32::from(p[3]) / 255.0,
                ]
            })
            .collect();
        Self { width, height, px }
    }

    pub fn to_srgb8(&self) -> Vec<[u8; 4]> {
        self.px
            .iter()
            .map(|p| {
                [
                    encode(p[0]),
                    encode(p[1]),
                    encode(p[2]),
                    (p[3].clamp(0.0, 1.0) * 255.0).round() as u8,
                ]
            })
            .collect()
    }

    /// An exact Gaussian of `sigma` pixels, truncated at five sigma, edge texels repeated.
    pub fn blur(&self, sigma: f32) -> Self {
        if sigma <= 0.0 {
            return self.clone();
        }
        let radius = (sigma * 5.0).ceil() as i32;
        let mut kernel: Vec<f32> = (-radius..=radius)
            .map(|i| (-0.5 * (i as f32 / sigma).powi(2)).exp())
            .collect();
        let total: f32 = kernel.iter().sum();
        kernel.iter_mut().for_each(|w| *w /= total);
        let pass = |source: &Self, horizontal: bool| {
            let mut out = source.clone();
            let width = source.width;
            let rows_per_thread = source.height.div_ceil(8).max(1);
            std::thread::scope(|scope| {
                for (chunk, rows) in out.px.chunks_mut(rows_per_thread * width).enumerate() {
                    let (kernel, source) = (&kernel, &source);
                    scope.spawn(move || {
                        for (r, line) in rows.chunks_mut(width).enumerate() {
                            let y = chunk * rows_per_thread + r;
                            for (x, texel) in line.iter_mut().enumerate() {
                                let mut sum = [0.0_f32; 4];
                                for (k, w) in kernel.iter().enumerate() {
                                    let d = k as i32 - radius;
                                    let (sx, sy) = if horizontal {
                                        ((x as i32 + d).clamp(0, width as i32 - 1) as usize, y)
                                    } else {
                                        (
                                            x,
                                            (y as i32 + d).clamp(0, source.height as i32 - 1)
                                                as usize,
                                        )
                                    };
                                    let p = source.px[sy * width + sx];
                                    for c in 0..4 {
                                        sum[c] += p[c] * w;
                                    }
                                }
                                *texel = sum;
                            }
                        }
                    });
                }
            });
            out
        };
        pass(&pass(self, true), false)
    }
}

/// Largest and mean absolute difference of two RGBA8 images over `region` (x, y, w, h), in
/// sRGB code values (1/255). Every channel counts.
pub fn error(width: usize, a: &[[u8; 4]], b: &[[u8; 4]], [x0, y0, w, h]: [usize; 4]) -> (f32, f32) {
    let (mut max, mut sum, mut n) = (0.0_f32, 0.0_f32, 0.0_f32);
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            for c in 0..4 {
                let d = (f32::from(a[y * width + x][c]) - f32::from(b[y * width + x][c])).abs();
                max = max.max(d);
                sum += d;
                n += 1.0;
            }
        }
    }
    (max, sum / n)
}
