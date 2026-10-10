//! The measures of `metrics` applied to the pre-pyramid blur. Run with `--nocapture` to see the
//! numbers; they are the baseline the new filter is compared with (see CHANGELOG).

use super::{
    gpu::gpu,
    legacy,
    metrics::{self, BlurFn},
};

fn legacy_blur(gpu: &super::gpu::Gpu) -> Box<BlurFn<'static>> {
    let legacy = legacy::Legacy::new(gpu);
    Box::new(move |gpu, size, canvas, sigma, scale, rect, rounding| {
        legacy.blur(gpu, size, canvas, sigma, scale, rect, rounding)
    })
}

/// Print the measures of `blur` under `label`.
pub fn report(label: &str, gpu: &super::gpu::Gpu, blur: &BlurFn) {
    for sigma in [1.0, 4.0, 8.0, 16.0, 32.0, 64.0] {
        let (max, mean) = metrics::profile(&gpu, blur, sigma, 1.0);
        let (dmax, dmean) = metrics::detail(&gpu, blur, sigma);
        println!("{label} profile sigma {sigma}: step max {max:.1} mean {mean:.3}; detail max {dmax:.1} mean {dmean:.3}");
    }
    for sigma in [2.0, 6.0, 12.0] {
        let (ripple, err) = metrics::ripple(&gpu, blur, sigma, 17);
        println!("{label} ripple sigma {sigma}: frame-diff deviation {ripple:.1}, error {err:.1}");
    }
    for (from, to) in [(3.0, 5.0), (7.0, 9.0), (15.0, 17.0)] {
        let jump = metrics::sweep(&gpu, blur, from, to, 0.1);
        println!("{label} sweep {from}..{to}: jump {jump:.1}");
    }
    for sigma in [2.0, 3.0, 6.0, 10.0] {
        let (got, want) = metrics::peak_spread(&gpu, blur, sigma, 17);
        println!("{label} line peak spread sigma {sigma}: {got:.3} (exact Gaussian {want:.3})");
    }
    for sigma in [4.0, 16.0, 48.0] {
        for (scale, gap) in [(1.0, 0.0), (1.0, 3.0), (1.5, 7.0), (1.25, 13.0)] {
            let e = metrics::edge_error(&gpu, blur, sigma, scale, gap);
            println!("{label} detail error near panel edge, sigma {sigma} scale {scale} gap {gap}: {e:.1}");
        }
    }
    for sigma in [4.0, 16.0, 48.0] {
        for scale in [1.0, 1.5] {
            let inner = metrics::rim(&gpu, blur, sigma, scale, true);
            let edge = metrics::rim(&gpu, blur, sigma, scale, false);
            println!("{label} rim sigma {sigma} scale {scale}: interior panel {inner:.1}, full screen {edge:.1}");
        }
    }
}

#[test]
fn baseline_measures() {
    let Some(gpu) = gpu("legacy blur baseline") else {
        return;
    };
    let blur = legacy_blur(&gpu);
    report("LEGACY", &gpu, &*blur);
}

#[test]
fn new_measures() {
    let Some(gpu) = gpu("blur measures") else {
        return;
    };
    let blur = super::engine::blur_fn(super::engine::Engine::new(&gpu));
    report("NEW", &gpu, &*blur);
}

#[test]
#[ignore = "debugging aid"]
fn debug_detail() {
    use super::{cpu, metrics, scene};
    let Some(gpu) = gpu("debug") else { return };
    let blur = super::engine::blur_fn(super::engine::Engine::new(&gpu));
    let canvas = scene::detail(metrics::SIZE);
    let got = blur(
        &gpu,
        metrics::SIZE,
        &canvas,
        4.0,
        1.0,
        metrics::panel(),
        0.0,
    );
    let want = metrics::reference(metrics::SIZE, &canvas, 4.0);
    let w = metrics::SIZE[0] as usize;
    let mut worst = (0.0, 0, 0);
    for y in 17..239 {
        for x in 17..495 {
            let d = (f32::from(got[y * w + x][0]) - f32::from(want[y * w + x][0])).abs();
            if d > worst.0 {
                worst = (d, x, y);
            }
        }
    }
    let mut bad = 0;
    for y in 17..239 {
        for x in 17..495 {
            if (f32::from(got[y * w + x][0]) - f32::from(want[y * w + x][0])).abs() > 20.0 {
                if bad < 12 {
                    println!("bad at {x},{y}");
                }
                bad += 1;
            }
        }
    }
    println!("bad count {bad}");
    println!(
        "worst {:?} got {:?} want {:?} canvas {:?}",
        worst,
        got[worst.2 * w + worst.1],
        want[worst.2 * w + worst.1],
        canvas[worst.2 * w + worst.1]
    );
    let _ = cpu::decode(0);
}

#[test]
#[ignore = "debugging aid"]
fn debug_sigma() {
    use super::{cpu, metrics, scene};
    let Some(gpu) = gpu("debug") else { return };
    let blur = super::engine::blur_fn(super::engine::Engine::new(&gpu));
    for sigma in [3.0_f32, 5.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0] {
        // impulse response along x of a bright line on black
        let canvas = scene::line(metrics::SIZE, 256.0, [0, 0, 0, 255], [255, 255, 255, 255]);
        let got = blur(
            &gpu,
            metrics::SIZE,
            &canvas,
            sigma,
            1.0,
            metrics::panel(),
            0.0,
        );
        let row = 128 * 512;
        let lin: Vec<f32> = (0..512).map(|x| cpu::decode(got[row + x][0])).collect();
        let total: f32 = lin.iter().sum();
        let mean: f32 = lin
            .iter()
            .enumerate()
            .map(|(x, v)| (x as f32 + 0.5) * v)
            .sum::<f32>()
            / total;
        let var: f32 = lin
            .iter()
            .enumerate()
            .map(|(x, v)| ((x as f32 + 0.5) - mean).powi(2) * v)
            .sum::<f32>()
            / total;
        // 1px wide line has variance 1/12
        println!(
            "sigma {sigma}: mean {mean:.3} effective sigma {:.3} (ratio {:.4}) sum {:.4}",
            (var - 1.0 / 12.0).sqrt(),
            (var - 1.0 / 12.0).sqrt() / sigma,
            total
        );
    }
}

#[test]
#[ignore = "debugging aid"]
fn debug_edges() {
    use super::{metrics, scene};
    let Some(gpu) = gpu("debug") else { return };
    let blur = super::engine::blur_fn(super::engine::Engine::new(&gpu));
    let canvas = scene::detail(metrics::SIZE);
    let sigma = 48.0;
    let got = blur(
        &gpu,
        metrics::SIZE,
        &canvas,
        sigma,
        1.0,
        metrics::panel(),
        0.0,
    );
    let want = metrics::reference(metrics::SIZE, &canvas, sigma);
    let w = 512;
    for (name, f) in [("x", true), ("y", false)] {
        println!("{name}: distance from left/top edge -> mean signed error, max abs");
        for d in [17usize, 18, 20, 24, 32, 48, 64, 96, 128] {
            let (mut s, mut m, mut n) = (0.0, 0.0_f32, 0.0);
            let range = if f { 17..239usize } else { 17..495 };
            for t in range {
                let (x, y) = if f { (d, t) } else { (t, d) };
                let e = f32::from(got[y * w + x][0]) - f32::from(want[y * w + x][0]);
                s += e;
                m = m.max(e.abs());
                n += 1.0;
            }
            println!("  {d}: {:.2} {m}", s / n);
        }
    }
}

#[test]
#[ignore = "debugging aid"]
fn debug_bias() {
    use super::{metrics, scene};
    let Some(gpu) = gpu("debug") else { return };
    let blur = super::engine::blur_fn(super::engine::Engine::new(&gpu));
    let canvas = scene::detail(metrics::SIZE);
    for sigma in [
        2.0, 3.5, 4.0, 5.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0,
    ] {
        let got = blur(
            &gpu,
            metrics::SIZE,
            &canvas,
            sigma,
            1.0,
            metrics::panel(),
            0.0,
        );
        let want = metrics::reference(metrics::SIZE, &canvas, sigma);
        let (mut s, mut n, mut sq) = (0.0, 0.0, 0.0);
        for y in 120..136usize {
            for x in 200..312usize {
                let e = f32::from(got[y * 512 + x][0]) - f32::from(want[y * 512 + x][0]);
                s += e;
                sq += e * e;
                n += 1.0;
            }
        }
        let f = zaxis::renderer::blur::params::filter(sigma);
        println!(
            "sigma {sigma}: level {} g {:.2} center bias {:.3} rms {:.3}",
            f.level,
            f.sigma,
            s / n,
            (sq / n as f32).sqrt()
        );
    }
}

#[test]
#[ignore = "debugging aid"]
fn debug_edges2() {
    use super::{metrics, scene};
    use zaxis::{vec2, Rect};
    let Some(gpu) = gpu("debug") else { return };
    let blur = super::engine::blur_fn(super::engine::Engine::new(&gpu));
    let canvas = scene::detail(metrics::SIZE);
    for (sigma, gap) in [
        (16.0_f32, 70.0_f32),
        (16.0, 8.0),
        (16.0, 0.0),
        (4.0, 8.0),
        (4.0, 0.0),
    ] {
        let rect = Rect::from_min_max(vec2(gap, gap), vec2(512.0 - gap, 256.0 - gap));
        let got = blur(&gpu, metrics::SIZE, &canvas, sigma, 1.0, rect, 0.0);
        let want = metrics::reference(metrics::SIZE, &canvas, sigma);
        let g = gap as usize;
        println!(
            "sigma {sigma} gap {gap}: mean signed error by distance from the panel's left edge"
        );
        for d in [1usize, 2, 4, 8, 16, 32, 64] {
            let (mut s, mut n) = (0.0, 0.0);
            for y in (g + 10)..(256 - g - 10) {
                let e = f32::from(got[y * 512 + g + d][0]) - f32::from(want[y * 512 + g + d][0]);
                s += e;
                n += 1.0;
            }
            print!(" {d}:{:.2}", s / n);
        }
        println!();
    }
}
