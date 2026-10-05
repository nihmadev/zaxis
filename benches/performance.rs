//! Run with `cargo bench --bench performance -- --help`.
//! Timings use real public APIs; functional assertions are outside timed regions.
#![forbid(unsafe_code)]

#[path = "support/access.rs"]
mod access;
#[path = "support/carousel.rs"]
mod carousel;
#[path = "support/combo_box.rs"]
mod combo_box;
#[path = "support/disclosure.rs"]
mod disclosure;
#[path = "support/dnd.rs"]
mod dnd;
#[path = "support/images.rs"]
mod images;
#[path = "support/list_box.rs"]
mod list_box;
#[path = "support/modal.rs"]
mod modal;
#[path = "support/number.rs"]
mod number;
#[path = "support/report.rs"]
mod report;
#[path = "support/scene.rs"]
mod scene;
#[path = "support/scroll.rs"]
mod scroll;
#[path = "support/text_area.rs"]
mod text_area;

use report::{Counters, FailedCase, Geometry, Report, ResultRow, Samples};
use scene::{Case, Scene};
use std::{
    error::Error,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zaxis::winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use zaxis::Instant;
use zaxis::{CacheStats, PresentationMode, RenderStatus, Renderer, RendererStats};

type BenchResult<T> = Result<T, Box<dyn Error>>;
const GPU_TIMEOUT: Duration = Duration::from_secs(10);

fn main() -> BenchResult<()> {
    if cfg!(debug_assertions) {
        println!("performance harness skipped in cargo test; use cargo bench");
        return Ok(());
    }
    let Some(options) = Options::parse()? else {
        return Ok(());
    };
    let cases = options.cases();
    if cases.is_empty() {
        return Err("filter matched no benchmark cases".into());
    }
    let mut report = Report {
        cpu: std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unavailable".into()),
        gpu_timestamps: options.gpu_timestamps,
        schema_version: 1,
        completed: false,
        failure: None,
        failed_case: None,
        max_p95_ms: options.max_p95,
        library_version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        unix_time_seconds: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        release_build: !cfg!(debug_assertions),
        gpu_wait: options.gpu_wait,
        warmup: options.warmup,
        iterations: options.iterations,
        sizes: options.sizes.clone(),
        physical_size: [options.size.width, options.size.height],
        scale_factor: 1.0,
        adapter: None,
        gpu_initialization_ms: None,
        supported_present_modes: Vec::new(),
        timer_pair_p50_ms: report::timer_overhead(),
        results: Vec::new(),
    };
    println!(
        "zaxis: {} samples + {} warmup; objects {:?}; {}x{}; GPU wait {}",
        options.iterations,
        options.warmup,
        options.sizes,
        options.size.width,
        options.size.height,
        options.gpu_wait
    );
    println!(
        "Per row: UI p50, total p50/p95/p99, throughput, mesh rebuilds, geometry/texture uploads."
    );
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_suites(&options, &cases, &mut report)
    }));
    let result = match result {
        Ok(result) => result,
        Err(panic) => {
            let message = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic");
            Err(format!("benchmark panicked: {message}").into())
        }
    };
    report.completed = result.is_ok();
    report.failure = result.as_ref().err().map(ToString::to_string);
    if let Some(parent) = options
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    serde_json::to_writer_pretty(std::fs::File::create(&options.output)?, &report)?;
    println!(
        "Verified {} rows ({} unsupported). JSON: {}",
        report.results.iter().filter(|r| r.verified).count(),
        report
            .results
            .iter()
            .filter(|r| r.unsupported.is_some())
            .count(),
        options.output.display()
    );
    result?;
    if let Some(budget) = options.max_p95 {
        let failures: Vec<_> = report
            .results
            .iter()
            .filter(|r| r.total.p95_ms > budget)
            .collect();
        for row in &failures {
            eprintln!(
                "BUDGET FAIL: {} {} n={} {} p95={:.3} ms > {budget:.3} ms",
                row.suite, row.case, row.objects, row.presentation, row.total.p95_ms
            );
        }
        if !failures.is_empty() {
            return Err(format!("{} rows exceeded p95 budget", failures.len()).into());
        }
    }
    Ok(())
}

#[path = "performance/cpu.rs"]
mod cpu;
#[path = "performance/gpu.rs"]
mod gpu;
#[path = "performance/options.rs"]
mod options;
use cpu::run_suites;
use gpu::{GpuRunner, Job};
use options::Options;
