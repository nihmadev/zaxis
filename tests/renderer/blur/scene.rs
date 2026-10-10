//! Canvases the blur tests filter: steps with exact fractional coverage, flat colors and
//! fine detail, painted on the CPU so the GPU filter and the reference see the same texels.

use super::cpu::{decode, encode};

pub type Canvas = Vec<[u8; 4]>;

/// Linear mix of two opaque sRGB colors by `t`, encoded back (what AA coverage produces).
pub fn mix(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let ch = |i: usize| encode(decode(a[i]) * (1.0 - t) + decode(b[i]) * t);
    [ch(0), ch(1), ch(2), 255]
}

/// `left` for x < `edge`, `right` after it; the column holding the (fractional) edge mixes by
/// the covered area, as an antialiased rasterizer does.
pub fn step(size: [u32; 2], edge: f32, left: [u8; 4], right: [u8; 4]) -> Canvas {
    (0..size[1])
        .flat_map(|_| {
            (0..size[0]).map(move |x| {
                let covered = (x as f32 + 1.0 - edge).clamp(0.0, 1.0);
                mix(left, right, covered)
            })
        })
        .collect()
}

pub fn flat(size: [u32; 2], color: [u8; 4]) -> Canvas {
    vec![color; (size[0] * size[1]) as usize]
}

/// Deterministic 1-2 px dark strokes on a light ground: stand-in for small text.
pub fn detail(size: [u32; 2]) -> Canvas {
    let mut state = 0x2545_f491_u32;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    let mut pixels = flat(size, [235, 235, 235, 255]);
    for _ in 0..(size[0] * size[1] / 40) {
        let (x, y) = (next() % size[0], next() % size[1]);
        let (w, h) = if next() % 2 == 0 {
            (1 + next() % 2, 5)
        } else {
            (5, 1 + next() % 2)
        };
        for dy in 0..h.min(size[1] - y) {
            for dx in 0..w.min(size[0] - x) {
                pixels[((y + dy) * size[0] + x + dx) as usize] = [30, 30, 36, 255];
            }
        }
    }
    pixels
}

/// A dark vertical line one pixel wide whose left edge sits at `x` (fractional), on `LIGHT`.
pub fn line(size: [u32; 2], x: f32, ground: [u8; 4], ink: [u8; 4]) -> Canvas {
    (0..size[1])
        .flat_map(|_| {
            (0..size[0]).map(move |col| {
                let left = col as f32;
                let covered = ((x + 1.0).min(left + 1.0) - x.max(left)).clamp(0.0, 1.0);
                mix(ground, ink, covered)
            })
        })
        .collect()
}
