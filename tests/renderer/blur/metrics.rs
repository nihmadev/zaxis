//! Measures shared by the baseline and the new filter: accuracy against the CPU Gaussian,
//! frame-to-frame ripple under sub-pixel motion, continuity in sigma and rims at edges.
//! Errors are in sRGB code values (1/255).

use super::{
    cpu::{error, Image},
    gpu::Gpu,
    scene::{self, Canvas},
};
use zaxis::{vec2, Rect};

/// Blur `canvas` (`size` texels) behind `rect` (logical) at `scale`; return the canvas.
pub type BlurFn<'a> = dyn Fn(&Gpu, [u32; 2], &Canvas, f32, f32, Rect, f32) -> Canvas + 'a;

pub const SIZE: [u32; 2] = [512, 256];
pub const DARK: [u8; 4] = [20, 22, 30, 255];
pub const LIGHT: [u8; 4] = [235, 232, 220, 255];

pub fn panel() -> Rect {
    Rect::from_min_max(vec2(16.0, 16.0), vec2(496.0, 240.0))
}

/// The panel in physical texels, for the error region.
pub fn region(rect: Rect, scale: f32) -> [usize; 4] {
    let x = (rect.min.x * scale).ceil() as usize + 1;
    let y = (rect.min.y * scale).ceil() as usize + 1;
    let w = (rect.max.x * scale).floor() as usize - 1 - x;
    let h = (rect.max.y * scale).floor() as usize - 1 - y;
    [x, y, w, h]
}

pub fn reference(size: [u32; 2], canvas: &Canvas, sigma_px: f32) -> Canvas {
    Image::from_srgb8(size[0] as usize, size[1] as usize, canvas)
        .blur(sigma_px)
        .to_srgb8()
}

/// (max, mean) error of one blur of a step against the exact Gaussian.
pub fn profile(gpu: &Gpu, blur: &BlurFn, sigma: f32, scale: f32) -> (f32, f32) {
    let size = [
        (SIZE[0] as f32 * scale) as u32,
        (SIZE[1] as f32 * scale) as u32,
    ];
    let canvas = scene::step(size, (SIZE[0] / 2) as f32 * scale, DARK, LIGHT);
    let got = blur(gpu, size, &canvas, sigma, scale, panel(), 0.0);
    let want = reference(size, &canvas, sigma * scale);
    error(size[0] as usize, &got, &want, region(panel(), scale))
}

/// (max, mean) error of the blur of fine detail against the exact Gaussian.
pub fn detail(gpu: &Gpu, blur: &BlurFn, sigma: f32) -> (f32, f32) {
    let canvas = scene::detail(SIZE);
    let got = blur(gpu, SIZE, &canvas, sigma, 1.0, panel(), 0.0);
    let want = reference(SIZE, &canvas, sigma);
    error(SIZE[0] as usize, &got, &want, region(panel(), 1.0))
}

/// Move an edge by a quarter pixel per frame. Returns (largest deviation of a frame-to-frame
/// difference from the exact Gaussian's difference, largest error against the Gaussian).
pub fn ripple(gpu: &Gpu, blur: &BlurFn, sigma: f32, frames: usize) -> (f32, f32) {
    let mut previous: Option<(Canvas, Canvas)> = None;
    let (mut worst_ripple, mut worst_error) = (0.0_f32, 0.0_f32);
    let [x, y, w, h] = region(panel(), 1.0);
    for k in 0..frames {
        let edge = 200.25 + 0.25 * k as f32;
        let canvas = scene::step(SIZE, edge, DARK, LIGHT);
        let got = blur(gpu, SIZE, &canvas, sigma, 1.0, panel(), 0.0);
        let want = reference(SIZE, &canvas, sigma);
        worst_error = worst_error.max(error(SIZE[0] as usize, &got, &want, [x, y, w, h]).0);
        if let Some((got_before, want_before)) = &previous {
            for row in y..y + h {
                for col in x..x + w {
                    let i = row * SIZE[0] as usize + col;
                    for c in 0..3 {
                        let g = f32::from(got[i][c]) - f32::from(got_before[i][c]);
                        let r = f32::from(want[i][c]) - f32::from(want_before[i][c]);
                        worst_ripple = worst_ripple.max((g - r).abs());
                    }
                }
            }
        }
        previous = Some((got, want));
    }
    (worst_ripple, worst_error)
}

/// Sweep sigma in small steps. Returns the largest jump of the error between neighbours:
/// how far a step in sigma moves the image beyond what the exact Gaussian moves.
pub fn sweep(gpu: &Gpu, blur: &BlurFn, from: f32, to: f32, step: f32) -> f32 {
    let canvas = scene::step(SIZE, 256.0, DARK, LIGHT);
    let [x, y, w, h] = region(panel(), 1.0);
    let mut previous: Option<(Canvas, Canvas)> = None;
    let mut worst = 0.0_f32;
    let mut sigma = from;
    while sigma <= to + 1e-4 {
        let got = blur(gpu, SIZE, &canvas, sigma, 1.0, panel(), 0.0);
        let want = reference(SIZE, &canvas, sigma);
        if let Some((got_before, want_before)) = &previous {
            for row in (y..y + h).step_by(8) {
                for col in x..x + w {
                    let i = row * SIZE[0] as usize + col;
                    for c in 0..3 {
                        let g = f32::from(got[i][c]) - f32::from(got_before[i][c]);
                        let r = f32::from(want[i][c]) - f32::from(want_before[i][c]);
                        worst = worst.max((g - r).abs());
                    }
                }
            }
        }
        previous = Some((got, want));
        sigma += step;
    }
    worst
}

/// Largest deviation from a flat color anywhere in the canvas after blurring a panel that
/// touches no screen edge (`inside`) or all of them.
pub fn rim(gpu: &Gpu, blur: &BlurFn, sigma: f32, scale: f32, inside: bool) -> f32 {
    let size = [
        (SIZE[0] as f32 * scale) as u32,
        (SIZE[1] as f32 * scale) as u32,
    ];
    let color = [200, 120, 60, 255];
    let canvas = scene::flat(size, color);
    let rect = if inside {
        panel()
    } else {
        Rect::from_min_max(vec2(0.0, 0.0), vec2(SIZE[0] as f32, SIZE[1] as f32))
    };
    let got = blur(gpu, size, &canvas, sigma, scale, rect, 0.0);
    got.iter()
        .flat_map(|p| (0..4).map(|c| f32::from(p[c].abs_diff(color[c]))))
        .fold(0.0, f32::max)
}

/// Slide a one pixel line by a quarter pixel per frame and follow how dark its blurred peak
/// is. Returns the relative spread (max - min) / mean of the peak over the frames, and the
/// same spread for the exact Gaussian: the difference is flicker the filter adds.
pub fn peak_spread(gpu: &Gpu, blur: &BlurFn, sigma: f32, frames: usize) -> (f32, f32) {
    let row = 128 * SIZE[0] as usize;
    let (mut got_peaks, mut want_peaks) = (Vec::new(), Vec::new());
    for k in 0..frames {
        let canvas = scene::line(SIZE, 200.0 + 0.25 * k as f32, LIGHT, DARK);
        let got = blur(gpu, SIZE, &canvas, sigma, 1.0, panel(), 0.0);
        let want = reference(SIZE, &canvas, sigma);
        let depth = |row_pixels: &[[u8; 4]]| {
            // Linear darkness at the centre of the line, the signal the eye follows.
            row_pixels[180..230]
                .iter()
                .map(|p| LIGHT[0].abs_diff(p[0]))
                .max()
                .unwrap() as f32
        };
        got_peaks.push(depth(&got[row..row + SIZE[0] as usize]));
        want_peaks.push(depth(&want[row..row + SIZE[0] as usize]));
    }
    let spread = |v: &[f32]| {
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        let (lo, hi) = v
            .iter()
            .fold((f32::MAX, 0.0_f32), |(a, b), x| (a.min(*x), b.max(*x)));
        (hi - lo) / mean.max(1.0)
    };
    (spread(&got_peaks), spread(&want_peaks))
}

/// Largest deviation from the exact Gaussian over the whole canvas of a detailed scene when
/// the panel sits `gap` texels from the screen edge (`gap` 0: touching it).
pub fn edge_error(gpu: &Gpu, blur: &BlurFn, sigma: f32, scale: f32, gap: f32) -> f32 {
    let size = [
        (SIZE[0] as f32 * scale) as u32,
        (SIZE[1] as f32 * scale) as u32,
    ];
    let canvas = scene::detail(size);
    let rect = Rect::from_min_max(
        vec2(gap, gap),
        vec2(SIZE[0] as f32 - gap, SIZE[1] as f32 - gap),
    );
    let got = blur(gpu, size, &canvas, sigma, scale, rect, 0.0);
    let want = reference(size, &canvas, sigma * scale);
    let r = [
        (gap * scale).ceil() as usize + 1,
        (gap * scale).ceil() as usize + 1,
        size[0] as usize - 2 * ((gap * scale).ceil() as usize + 1),
        size[1] as usize - 2 * ((gap * scale).ceil() as usize + 1),
    ];
    error(size[0] as usize, &got, &want, r).0
}
