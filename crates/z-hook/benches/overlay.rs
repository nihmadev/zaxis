//! What the overlay costs a host frame.
//!
//! * `driver/*`: the frame driver on the host's render thread with a backend that does nothing
//!   (the interface pass, geometry, input routing), CPU only, in microseconds.
//! * `layer/*` (Linux, needs a Vulkan device and the built examples): a host's frame time with
//!   and without the Vulkan layer, in a headless swapchain.
//!
//! `cargo bench -p z-hook --bench overlay -- [--iterations N] [--output path.json] [--no-layer]`.
//! The JSON says, per case, whether it completed, was verified (its state checked outside the
//! timed section) or is unsupported on this machine.

use std::{path::PathBuf, process::Command, time::Instant};
use z_hook::{testing::MockBackend, FrameOutcome, Overlay, OverlayOptions};
use zaxis::{Context, Window};

struct Case {
    name: String,
    completed: bool,
    verified: bool,
    unsupported: Option<String>,
    p50: f64,
    p95: f64,
    p99: f64,
    unit: &'static str,
    note: String,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn summary(
    name: &str,
    mut samples: Vec<f64>,
    unit: &'static str,
    verified: bool,
    note: String,
) -> Case {
    samples.sort_by(|a, b| a.total_cmp(b));
    Case {
        name: name.into(),
        completed: true,
        verified,
        unsupported: None,
        p50: percentile(&samples, 0.5),
        p95: percentile(&samples, 0.95),
        p99: percentile(&samples, 0.99),
        unit,
        note,
    }
}

/// A window of `rows` labels, one of which changes every frame when `busy`.
fn overlay(rows: usize, busy: bool) -> Overlay {
    let mut frame = 0u64;
    Overlay::detached(
        OverlayOptions::default().frame_budget(None),
        move |ctx: &mut Context| {
            frame += 1;
            Window::new("Bench").show(ctx, |ui| {
                for row in 0..rows {
                    if busy && row == 0 {
                        ui.label(format!("frame {frame}"));
                    } else {
                        ui.label(format!("row {row}"));
                    }
                }
            });
            if busy {
                ctx.request_repaint();
            }
        },
    )
}

fn driver_case(name: &str, rows: usize, busy: bool, iterations: usize) -> Case {
    let overlay = overlay(rows, busy);
    let mut backend = MockBackend::new([1280, 720]);
    for _ in 0..30 {
        overlay.present(&mut backend);
    }
    let passes_before = overlay.stats().ui_passes;
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        let outcome = overlay.present(&mut backend);
        samples.push(started.elapsed().as_secs_f64() * 1e6);
        assert!(matches!(outcome, FrameOutcome::Drawn { .. }));
    }
    let passes = overlay.stats().ui_passes - passes_before;
    let expected = if busy { iterations as u64 } else { 0 };
    summary(
        name,
        samples,
        "us",
        passes == expected,
        format!("{passes} interface passes in {iterations} frames, expected {expected}"),
    )
}

fn input_case(iterations: usize) -> Case {
    let overlay = overlay(100, false);
    let mut backend = MockBackend::new([1280, 720]);
    overlay.present(&mut backend);
    let sink = overlay.input();
    let mut samples = Vec::with_capacity(iterations);
    for i in 0..iterations {
        let started = Instant::now();
        sink.send(zaxis::InputEvent::PointerMoved {
            x: 20.0 + (i % 200) as f64,
            y: 60.0 + (i % 100) as f64,
        });
        samples.push(started.elapsed().as_secs_f64() * 1e6);
    }
    summary(
        "driver/pointer-move-routing",
        samples,
        "us",
        true,
        "handled on the render thread".into(),
    )
}

fn target_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

/// The host's frame time with the layer (or without): p50, p95 and maximum in milliseconds,
/// from the host's own report.
fn host_frames(with_layer: bool) -> Option<(f64, f64, f64)> {
    let target = target_dir();
    let host = target.join("examples/vk_host");
    let library = target.join("examples/libvk_overlay.so");
    if !host.exists() || !library.exists() {
        return None;
    }
    let directory = std::env::temp_dir().join(format!("z-hook-bench-{}", std::process::id()));
    std::fs::create_dir_all(&directory).ok()?;
    let manifest = format!(
        r#"{{"file_format_version":"1.2.0","layer":{{"name":"VK_LAYER_ZAXIS_overlay","type":"GLOBAL","library_path":"{}","api_version":"1.3.0","implementation_version":"1","description":"bench","functions":{{"vkNegotiateLoaderLayerInterfaceVersion":"vkNegotiateLoaderLayerInterfaceVersion"}},"enable_environment":{{"ZAXIS_HOOK_ENABLE":"1"}},"disable_environment":{{"ZAXIS_HOOK_DISABLE":"1"}}}}}}"#,
        library.display()
    );
    std::fs::write(directory.join("layer.json"), manifest).ok()?;
    let mut command = Command::new(host);
    command
        .args(["--frames", "900", "--delay", "2"])
        .env("VK_ADD_IMPLICIT_LAYER_PATH", &directory);
    if with_layer {
        command.env("ZAXIS_HOOK_ENABLE", "1");
    }
    let output = command.output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().find(|l| l.contains("frame ms"))?;
    let numbers: Vec<f64> = line
        .split_whitespace()
        .filter_map(|w| w.parse().ok())
        .collect();
    (numbers.len() == 3).then(|| (numbers[0], numbers[1], numbers[2]))
}

fn layer_cases() -> Vec<Case> {
    let unsupported = |reason: &str| Case {
        name: "layer/host-frame".into(),
        completed: false,
        verified: false,
        unsupported: Some(reason.into()),
        p50: 0.0,
        p95: 0.0,
        p99: 0.0,
        unit: "ms",
        note: String::new(),
    };
    if !cfg!(target_os = "linux") || std::env::var_os("ZAXIS_SKIP_GPU_TESTS").is_some() {
        return vec![unsupported(
            "needs Linux and a Vulkan device (ZAXIS_SKIP_GPU_TESTS skips it)",
        )];
    }
    let (Some(without), Some(with)) = (host_frames(false), host_frames(true)) else {
        return vec![unsupported(
            "no Vulkan device, or the examples are not built (cargo build -p z-hook --examples)",
        )];
    };
    let case = |name: &str, (p50, p95, max): (f64, f64, f64)| Case {
        name: name.into(),
        completed: true,
        verified: true,
        unsupported: None,
        p50,
        p95,
        p99: max,
        unit: "ms",
        note: "p99 column is the maximum; one host frame in a headless swapchain, overlay at rest"
            .into(),
    };
    vec![
        case("layer/host-frame-without", without),
        case("layer/host-frame-with", with),
    ]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |key: &str| args.windows(2).find(|a| a[0] == key).map(|a| a[1].clone());
    let iterations: usize = value("--iterations").map_or(2000, |s| s.parse().unwrap());
    let output = PathBuf::from(value("--output").unwrap_or_else(|| {
        format!(
            "{}/../../target/z-hook-benchmark.json",
            env!("CARGO_MANIFEST_DIR")
        )
    }));
    let mut cases = Vec::new();
    for rows in [10, 100, 1000] {
        cases.push(driver_case(
            &format!("driver/idle/{rows}-rows"),
            rows,
            false,
            iterations,
        ));
        cases.push(driver_case(
            &format!("driver/ui-pass/{rows}-rows"),
            rows,
            true,
            iterations.min(500),
        ));
    }
    cases.push(input_case(iterations));
    if !args.iter().any(|a| a == "--no-layer") {
        cases.extend(layer_cases());
    }
    let mut json = String::from("[\n");
    for (index, case) in cases.iter().enumerate() {
        match &case.unsupported {
            Some(reason) => println!("{:<34} unsupported: {reason}", case.name),
            None => println!(
                "{:<34} p50 {:>9.3}{u} p95 {:>9.3}{u} p99 {:>9.3}{u}  verified={} {}",
                case.name,
                case.p50,
                case.p95,
                case.p99,
                case.verified,
                case.note,
                u = case.unit
            ),
        }
        json.push_str(&format!(
            "  {{\"name\":\"{}\",\"completed\":{},\"failure\":null,\"verified\":{},\"unsupported\":{},\"unit\":\"{}\",\"p50\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"note\":\"{}\"}}{}\n",
            case.name,
            case.completed,
            case.verified,
            case.unsupported.as_ref().map_or("null".to_string(), |r| format!("\"{r}\"")),
            case.unit,
            case.p50,
            case.p95,
            case.p99,
            case.note,
            if index + 1 < cases.len() { "," } else { "" }
        ));
    }
    json.push(']');
    std::fs::write(&output, json).unwrap();
    println!("wrote {}", output.display());
    assert!(
        cases
            .iter()
            .filter(|c| c.unsupported.is_none())
            .all(|c| c.completed && c.verified),
        "a case did not verify"
    );
}
