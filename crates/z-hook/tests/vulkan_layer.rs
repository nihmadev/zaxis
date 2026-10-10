//! The Vulkan layer in a real process: an application of its own (`vk_host`, headless) runs with
//! the layer (`vk_overlay`) enabled through an implicit-layer manifest, and a frame is dumped
//! from its swapchain image after the overlay drew into it.
//!
//! Needs a Vulkan driver and the loader. Skipped, with a message, when `ZAXIS_SKIP_GPU_TESTS` is
//! set or no device is found.
#![cfg(all(feature = "vulkan", target_os = "linux"))]

use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn skipped(name: &str) -> bool {
    if std::env::var_os("ZAXIS_SKIP_GPU_TESTS").is_some() {
        eprintln!("SKIPPED {name}: ZAXIS_SKIP_GPU_TESTS");
        return true;
    }
    false
}

fn target() -> PathBuf {
    let exe = std::env::current_exe().expect("test executable path");
    exe.parent()
        .and_then(Path::parent)
        .expect("target directory")
        .to_path_buf()
}

struct Run {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
    dump: PathBuf,
    log: String,
}

/// Run the host with the layer; `dump_frame` is the overlay frame to write out.
fn run(name: &str, host_args: &[&str], dump_frame: u32, extra_env: &[(&str, &str)]) -> Option<Run> {
    let directory = std::env::temp_dir().join(format!("z-hook-test-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let library = target().join("examples/libvk_overlay.so");
    let manifest = format!(
        r#"{{"file_format_version":"1.2.0","layer":{{"name":"VK_LAYER_ZAXIS_overlay","type":"GLOBAL","library_path":"{}","api_version":"1.3.0","implementation_version":"1","description":"test","functions":{{"vkNegotiateLoaderLayerInterfaceVersion":"vkNegotiateLoaderLayerInterfaceVersion"}},"enable_environment":{{"ZAXIS_HOOK_ENABLE":"1"}},"disable_environment":{{"ZAXIS_HOOK_DISABLE":"1"}}}}}}"#,
        library.display()
    );
    std::fs::write(directory.join("VK_LAYER_ZAXIS_overlay.json"), manifest).unwrap();
    let dump = directory.join("frame.png");
    let log_path = directory.join("hook.log");
    let host = target().join("examples/vk_host");
    let mut command = Command::new(host);
    command
        .args(host_args)
        .args(["--delay", "8"])
        .env("VK_ADD_IMPLICIT_LAYER_PATH", &directory)
        .env("ZAXIS_HOOK_ENABLE", "1")
        .env("ZAXIS_HOOK_DUMP", &dump)
        .env("ZAXIS_HOOK_DUMP_FRAME", dump_frame.to_string())
        .env("ZAXIS_HOOK_DUMP_EVERY", "1")
        .env("ZAXIS_HOOK_LOG", &log_path)
        .env("ZAXIS_HOOK_LOG_LEVEL", "info");
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let output = command.output().expect("vk_host starts");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if stderr.contains("ERROR_INCOMPATIBLE_DRIVER")
        || stderr.contains("no graphics queue")
        || stderr.contains("Unable to find")
        || stderr.contains("instance: ")
    {
        eprintln!(
            "SKIPPED {name}: no usable Vulkan device ({})",
            stderr.lines().next().unwrap_or("")
        );
        return None;
    }
    Some(Run {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        log: std::fs::read_to_string(&log_path).unwrap_or_default(),
        stderr,
        dump,
    })
}

/// What a dump shows: the host's clear color (taken in the corner the overlay never reaches),
/// how much of the frame still has it, and the color inside the overlay's window.
struct Shot {
    size: (u32, u32),
    host: [u8; 3],
    host_share: f64,
    panel: [u8; 3],
}

fn analyze(path: &Path) -> Shot {
    let image = image::open(path)
        .expect("the frame dump is a PNG")
        .to_rgb8();
    let (width, height) = image.dimensions();
    let host = image.get_pixel(width - 2, height - 2).0;
    let same = image.pixels().filter(|pixel| pixel.0 == host).count();
    Shot {
        size: (width, height),
        host,
        host_share: same as f64 / f64::from(width * height),
        panel: image.get_pixel(200, 200).0,
    }
}

fn distance(a: [u8; 3], b: [u8; 3]) -> i32 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (i32::from(*a) - i32::from(b)).abs())
        .sum()
}

fn assert_drawn(run: &Run, size: (u32, u32)) {
    assert!(
        run.status.success(),
        "host failed: {}\n{}",
        run.stdout,
        run.stderr
    );
    assert!(
        run.dump.exists(),
        "no frame was dumped; hook log:\n{}\nhost stderr:\n{}",
        run.log,
        run.stderr
    );
    let shot = analyze(&run.dump);
    assert_eq!(shot.size, size);
    assert!(
        shot.host_share > 0.4,
        "most of the frame is still the host's, got {:.2}",
        shot.host_share
    );
    assert!(
        distance(shot.panel, shot.host) > 100,
        "the overlay's window should differ from the host's color {:?}, got {:?}",
        shot.host,
        shot.panel
    );
    assert!(
        !run.log.contains("ERROR"),
        "the layer reported an error:\n{}",
        run.log
    );
}

#[test]
fn the_overlay_is_drawn_over_the_hosts_frame() {
    if skipped("the_overlay_is_drawn_over_the_hosts_frame") {
        return;
    }
    let Some(run) = run("plain", &["--frames", "300"], 10, &[]) else {
        return;
    };
    assert_drawn(&run, (640, 480));
    assert!(
        run.stdout.contains("presented 300 frames"),
        "{}",
        run.stdout
    );
}

#[test]
fn an_srgb_swapchain_gets_the_overlay_too() {
    if skipped("an_srgb_swapchain_gets_the_overlay_too") {
        return;
    }
    let Some(run) = run("srgb", &["--frames", "300", "--format", "srgb"], 10, &[]) else {
        return;
    };
    assert_drawn(&run, (640, 480));
}

#[test]
fn a_host_that_asked_for_vulkan_1_0_still_works() {
    if skipped("a_host_that_asked_for_vulkan_1_0_still_works") {
        return;
    }
    let Some(run) = run("v10", &["--frames", "300", "--api", "1.0"], 10, &[]) else {
        return;
    };
    assert_drawn(&run, (640, 480));
}

#[test]
fn recreating_the_swapchain_keeps_the_overlay_on_the_new_one() {
    if skipped("recreating_the_swapchain_keeps_the_overlay_on_the_new_one") {
        return;
    }
    let Some(run) = run(
        "resize",
        &["--frames", "300", "--resize", "800x600"],
        10,
        &[],
    ) else {
        return;
    };
    assert_drawn(&run, (800, 600));
}

#[test]
fn a_disabled_layer_leaves_the_host_alone() {
    if skipped("a_disabled_layer_leaves_the_host_alone") {
        return;
    }
    let Some(run) = run(
        "disabled",
        &["--frames", "60"],
        10,
        &[("ZAXIS_HOOK_DISABLE", "1")],
    ) else {
        return;
    };
    assert!(run.status.success(), "{}", run.stderr);
    assert!(
        !run.dump.exists(),
        "the layer was loaded although it is disabled"
    );
}
