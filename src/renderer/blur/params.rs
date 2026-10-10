//! How a sigma is filtered: how many pyramid levels, and the Gaussian that remains at the
//! last one. Pure arithmetic, mirrored by the tests.
//!
//! Each pyramid step halves the resolution with a (1 5 10 10 5 1)/32 binomial kernel (variance
//! 1.25 texel^2 of the finer level) and the final cubic B-spline reconstruction adds 1/3 texel^2
//! of the coarsest one. Those variances are known, so the Gaussian at the last level is sized
//! to make the total exactly `sigma^2`: a smooth change of sigma changes the Gaussian's sigma
//! smoothly, and moving to the next level at a threshold swaps one nearly Gaussian kernel for
//! another of the same variance instead of changing the blur strength.

/// Deepest pyramid level: sigma 64 at 4x DPI needs level 6.
pub const MAX_LEVEL: u32 = 8;
/// The widest Gaussian run at any level, in texels of that level. Above it a level is added.
/// At level 0 this is the boundary between the exact full-resolution blur and the pyramid.
pub const GAUSS_MAX: f32 = 4.0;
/// Kernel radius in sigmas; the truncated tail is 0.05% of the weight.
pub const TRUNCATION: f32 = 3.5;
/// Largest kernel radius in texels (`GAUSS_MAX * TRUNCATION` rounded up).
pub const MAX_RADIUS: u32 = 16;
/// Below this physical sigma the blur is not visible and the filter copies.
pub const COPY_BELOW: f32 = 0.2;

/// The filter of one sigma.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Filter {
    /// Pyramid levels below full resolution; 0 filters the backdrop itself.
    pub level: u32,
    /// Sigma of the Gaussian at that level, in its texels.
    pub sigma: f32,
    /// Kernel radius in texels of that level; 0 copies.
    pub radius: u32,
}

/// Variance (texel^2 of the full-resolution image) the pyramid adds by itself at `level`.
pub fn fixed_variance(level: u32) -> f32 {
    if level == 0 {
        return 0.0;
    }
    let steps: f32 = (1..=level).map(|k| 1.25 * 4.0_f32.powi(k as i32 - 1)).sum();
    steps + 4.0_f32.powi(level as i32) / 3.0
}

/// The filter for a Gaussian of `sigma` physical pixels.
pub fn filter(sigma: f32) -> Filter {
    if !(sigma >= COPY_BELOW) {
        return Filter {
            level: 0,
            sigma: 0.0,
            radius: 0,
        };
    }
    let mut level = 0;
    let sigma_at = |level: u32| {
        let left = sigma * sigma - fixed_variance(level);
        (left.max(0.0) / 4.0_f32.powi(level as i32)).sqrt()
    };
    while level < MAX_LEVEL && sigma_at(level) > GAUSS_MAX {
        level += 1;
    }
    let sigma = sigma_at(level);
    let radius = if sigma < COPY_BELOW {
        0
    } else {
        ((sigma * TRUNCATION).ceil() as u32).clamp(1, MAX_RADIUS)
    };
    Filter {
        level,
        sigma,
        radius,
    }
}

/// Size of pyramid level `level` of a `size` texture.
pub fn level_size([width, height]: [u32; 2], level: u32) -> [u32; 2] {
    [
        width.div_ceil(1 << level).max(1),
        height.div_ceil(1 << level).max(1),
    ]
}

/// The normalized one-sided kernel `fs_gaussian` evaluates: `(offset, weight)` of each fetch
/// pair after merging taps `i` and `i + 1`, and the weight of the center fetch (always 1
/// before normalization). Mirrors the shader so tests can check the arithmetic on the CPU.
pub fn gaussian_fetches(sigma: f32, radius: u32) -> Vec<(f32, f32)> {
    let weight = |i: u32| (-0.5 * (i * i) as f32 / (sigma * sigma)).exp();
    (1..=radius)
        .step_by(2)
        .map(|i| {
            let (w1, w2) = (weight(i), if i < radius { weight(i + 1) } else { 0.0 });
            let w = w1 + w2;
            ((i as f32 * w1 + (i + 1) as f32 * w2) / w, w)
        })
        .collect()
}
