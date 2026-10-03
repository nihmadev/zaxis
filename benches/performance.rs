//! Run with `cargo bench --bench performance -- --help`.
//! Timings use real public APIs; functional assertions are outside timed regions.
#![forbid(unsafe_code)]

#[path = "support/combo_box.rs"]
mod combo_box;
#[path = "support/images.rs"]
mod images;
#[path = "support/report.rs"]
mod report;
#[path = "support/scene.rs"]
mod scene;
#[path = "support/scroll.rs"]
mod scroll;

use report::{Counters, FailedCase, Geometry, Report, ResultRow, Samples};
use scene::{Case, Scene};
use std::{
    error::Error,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use zaxis::winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use zaxis::{CacheStats, PresentationMode, RenderStatus, Renderer, RendererStats};

type BenchResult<T> = Result<T, Box<dyn Error>>;
const GPU_TIMEOUT: Duration = Duration::from_secs(10);

struct Options {
    iterations: usize,
    warmup: usize,
    sizes: Vec<usize>,
    size: PhysicalSize<u32>,
    cpu: bool,
    gpu: bool,
    gpu_wait: bool,
    gpu_timestamps: bool,
    filter: String,
    output: PathBuf,
    max_p95: Option<f64>,
}

impl Options {
    fn parse() -> BenchResult<Option<Self>> {
        let mut options = Self {
            iterations: 120,
            warmup: 30,
            sizes: vec![32, 128, 512],
            size: PhysicalSize::new(1280, 800),
            cpu: true,
            gpu: true,
            gpu_wait: false,
            gpu_timestamps: false,
            filter: String::new(),
            output: PathBuf::from("target/benchmark.json"),
            max_p95: None,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bench" => {} // Cargo may append the libtest flag to custom harnesses.
                "--help" | "-h" => {
                    println!("zaxis performance benchmark (release, real desktop GPU by default)\n\
                        --quick                  12 samples, 3 warmup frames, 32 objects\n\
                        --hard                   600 samples, 120 warmup frames, 128/512/2048 objects\n\
                        --cpu-only / --gpu-only   select suite (CPU needs no display)\n\
                        --iterations N           measured frames per case/size/mode (default 120)\n\
                        --warmup N               untimed warmup frames (default 30, minimum 1)\n\
                        --sizes N,N,...          objects per stress scene (default 32,128,512)\n\
                        --resolution WxH         physical window size (default 1280x800)\n\
                        --filter TEXT            comma-separated case substrings; --list for names\n\
                        --gpu-wait               serialize each frame until GPU work completes\n\
                        --gpu-timestamps         diagnostic pass/copy timestamps and serialized readback\n\
                        --output PATH            JSON with all distributions and counters\n\
                        --max-p95-ms N           fail if total frame p95 exceeds this budget\n\
                        --list                   list cases and exit\n\n\
                        Vsync and Immediate both run. Immediate can fall back; capabilities are reported.\n\
                        Render includes acquisition, encoding, submission and present, not GPU-only time.\n\
                        Synthetic input measures dispatcher-to-model/frame latency, not OS or display latency.");
                    return Ok(None);
                }
                "--list" => {
                    for case in Case::ALL {
                        println!("{}", case.name());
                    }
                    return Ok(None);
                }
                "--quick" => {
                    options.iterations = 12;
                    options.warmup = 3;
                    options.sizes = vec![32];
                }
                "--hard" => {
                    options.iterations = 600;
                    options.warmup = 120;
                    options.sizes = vec![128, 512, 2048];
                }
                "--cpu-only" => {
                    options.cpu = true;
                    options.gpu = false;
                }
                "--gpu-only" => {
                    options.cpu = false;
                    options.gpu = true;
                }
                "--gpu-wait" => options.gpu_wait = true,
                "--gpu-timestamps" => options.gpu_timestamps = true,
                "--iterations" => {
                    options.iterations = args.next().ok_or("missing iterations")?.parse()?
                }
                "--warmup" => options.warmup = args.next().ok_or("missing warmup")?.parse()?,
                "--sizes" => {
                    options.sizes = args
                        .next()
                        .ok_or("missing sizes")?
                        .split(',')
                        .map(str::parse)
                        .collect::<Result<_, _>>()?
                }
                "--filter" => options.filter = args.next().ok_or("missing filter")?,
                "--output" => options.output = args.next().ok_or("missing output")?.into(),
                "--max-p95-ms" => {
                    options.max_p95 = Some(args.next().ok_or("missing p95 budget")?.parse()?)
                }
                "--resolution" => {
                    let value = args.next().ok_or("missing resolution")?;
                    let (w, h) = value.split_once('x').ok_or("expected WIDTHxHEIGHT")?;
                    options.size = PhysicalSize::new(w.parse()?, h.parse()?);
                }
                _ => return Err(format!("unknown option {arg}; use --help").into()),
            }
        }
        if cfg!(debug_assertions) {
            return Err("run benchmarks in release mode with cargo bench".into());
        }
        if options.iterations < 2
            || options.warmup == 0
            || options.sizes.is_empty()
            || options.sizes.contains(&0)
        {
            return Err("iterations must be >= 2, warmup and every size must be >= 1".into());
        }
        if options.size.width < 640 || options.size.height < 480 {
            return Err("resolution must be at least 640x480 for the interaction probe".into());
        }
        if options.max_p95.is_some_and(|p| !p.is_finite() || p <= 0.0) {
            return Err("p95 budget must be finite and positive".into());
        }
        Ok(Some(options))
    }
    fn cases(&self) -> Vec<Case> {
        Case::ALL
            .into_iter()
            .filter(|c| {
                self.filter
                    .split(',')
                    .any(|filter| c.name().contains(filter))
            })
            .collect()
    }
}

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

fn run_suites(options: &Options, cases: &[Case], report: &mut Report) -> BenchResult<()> {
    if options.cpu {
        for &count in &options.sizes {
            for &case in cases {
                if let Case::Images(image) = case {
                    if let Some(reason) = image.unsupported(count) {
                        report
                            .results
                            .push(unsupported("CPU", case, count, "none", reason));
                        continue;
                    }
                }
                // Protocol rows exercise the GPU backend rather than Context.
                if case.protocol() {
                    continue;
                }
                report.failed_case = Some(FailedCase::new("CPU", case.name(), count, "none"));
                let row = cpu_case(options, case, count);
                row.print();
                report.results.push(row);
                report.failed_case = None;
            }
        }
    }
    if options.gpu {
        let event_loop = EventLoop::<()>::with_user_event().build()?;
        let proxy = event_loop.create_proxy();
        let image_waker: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = proxy.send_event(());
        });
        let mut jobs = Vec::new();
        for mode in [PresentationMode::Vsync, PresentationMode::Immediate] {
            for &count in &options.sizes {
                for &case in cases {
                    if case != Case::Cold {
                        if let Case::Images(image) = case {
                            if let Some(reason) = image.unsupported(count) {
                                report.results.push(unsupported(
                                    "GPU",
                                    case,
                                    count,
                                    &format!("{mode:?}"),
                                    reason,
                                ));
                                continue;
                            }
                        }
                        jobs.push(Job { mode, count, case });
                    }
                }
            }
        }
        let mut runner = GpuRunner {
            options,
            report,
            jobs,
            next_job: 0,
            state: None,
            error: None,
            finished: false,
            image_waker,
        };
        event_loop.run_app(&mut runner)?;
        if let Some(error) = runner.error.take() {
            return Err(error);
        }
        if !runner.finished {
            return Err("GPU benchmark closed before all cases completed".into());
        }
    }
    if report.results.is_empty() {
        return Err(
            "no cases in the selected suite (cold_ui is CPU-only, protocol is GPU-only)".into(),
        );
    }
    Ok(())
}

fn unsupported(suite: &str, case: Case, count: usize, mode: &str, reason: String) -> ResultRow {
    let mut row = ResultRow::new(
        suite,
        case.name(),
        count,
        mode,
        Samples::default(),
        Duration::from_nanos(1),
        Counters::default(),
        Geometry::default(),
    );
    row.verified = false;
    row.unsupported = Some(reason);
    row
}
fn cpu_case(options: &Options, case: Case, count: usize) -> ResultRow {
    let mut scene = Scene::new(case, count, options.size, 1.0);
    for step in 0..options.warmup {
        scene.input(step);
        scene.build();
        if let Some(images) = &mut scene.images {
            images.await_ready(&mut scene.context);
        }
        scene.verify();
    }
    let mut samples = Samples::with_capacity(options.iterations);
    if scene.images.is_some() {
        samples.memory();
    }
    let mut counters = Counters::default();
    scene.context.take_image_timings();
    for step in options.warmup..options.warmup + options.iterations {
        let before = if case == Case::Cold {
            CacheStats::default()
        } else {
            scene.context.cache_stats()
        };
        let start = Instant::now();
        let image_before = scene.context.image_metrics();
        scene.input(step);
        let input = start.elapsed();
        let ui_start = Instant::now();
        scene.build();
        let mut ui = ui_start.elapsed();
        if let Some(images) = &mut scene.images {
            ui += images.await_ready(&mut scene.context);
        }
        let total = start.elapsed();
        samples.input.push(input);
        samples.ui.push(ui);
        samples.total.push(total);
        if case.interactive() {
            samples.response.push(total);
        }
        scene.verify();
        counters.add_cpu(before, scene.context.cache_stats());
        counters.images(image_before, scene.context.image_metrics());
        samples.images(&mut scene.context);
        if scene.images.is_some()
            && (step % 16 == 0 || step + 1 == options.warmup + options.iterations)
        {
            samples.memory();
        }
    }
    let measured_elapsed: Duration = samples.total.iter().copied().sum();
    ResultRow::new(
        "CPU",
        case.name(),
        count,
        "none",
        samples,
        measured_elapsed,
        counters,
        Geometry::new(scene.draw_data()),
    )
}

#[derive(Clone, Copy)]
struct Job {
    mode: PresentationMode,
    count: usize,
    case: Case,
}

struct ActiveCase {
    scene: Scene,
    job: Job,
    frames: usize,
    samples: Samples,
    counters: Counters,
    gpu_before: RendererStats,
    start: Option<Instant>,
    previous_present: Option<Instant>,
    attempts: usize,
    last_progress: Instant,
    image_transaction: Option<(CacheStats, zaxis::ImageMetrics, Instant, Duration)>,
}

struct GpuState {
    window: Arc<Window>,
    renderer: Renderer,
    active: Option<ActiveCase>,
    retry_at: Option<Instant>,
}

struct GpuRunner<'a> {
    options: &'a Options,
    report: &'a mut Report,
    jobs: Vec<Job>,
    next_job: usize,
    state: Option<GpuState>,
    error: Option<Box<dyn Error>>,
    finished: bool,
    image_waker: Arc<dyn Fn() + Send + Sync>,
}

impl GpuRunner<'_> {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: Box<dyn Error>) {
        self.error = Some(error);
        event_loop.exit();
    }

    fn start_job(&mut self) -> BenchResult<bool> {
        let state = self.state.as_mut().unwrap();
        state.renderer.wait_idle(GPU_TIMEOUT)?;
        let Some(&job) = self.jobs.get(self.next_job) else {
            return Ok(false);
        };
        self.next_job += 1;
        self.report.failed_case = Some(FailedCase::new(
            "GPU",
            job.case.name(),
            job.count,
            match job.mode {
                PresentationMode::Vsync => "Vsync",
                PresentationMode::Immediate => "Immediate",
            },
        ));
        state.renderer.set_presentation_mode(job.mode);
        state
            .renderer
            .set_image_diagnostics(self.options.gpu_timestamps);
        let mut scene = Scene::new(
            job.case,
            job.count,
            state.window.inner_size(),
            state.window.scale_factor(),
        );
        let wake = self.image_waker.clone();
        scene.context.set_image_waker(move || wake());
        scene.context.take_image_timings();
        if matches!(job.case, Case::Images(images::ImageCase::GpuRecovery)) {
            // Keep the live Context and its CPU resources across a fresh backend/device.
            state.renderer = pollster::block_on(Renderer::new_with_presentation_mode(
                state.window.clone(),
                job.mode,
            ))?;
            state
                .renderer
                .set_image_diagnostics(self.options.gpu_timestamps);
            state
                .renderer
                .render(scene.draw_data(), zaxis::Color::BLACK)?;
            state.renderer.wait_idle(GPU_TIMEOUT)?;
            scene.verify();
        }
        state.window.set_title(&format!(
            "zaxis benchmark — {} / {} objects / {:?} ({}/{})",
            job.case.name(),
            job.count,
            job.mode,
            self.next_job,
            self.jobs.len()
        ));
        state.active = Some(ActiveCase {
            scene,
            job,
            frames: 0,
            samples: Samples::with_capacity(self.options.iterations),
            counters: Counters::default(),
            gpu_before: state.renderer.stats(),
            start: None,
            previous_present: None,
            attempts: 0,
            last_progress: Instant::now(),
            image_transaction: None,
        });
        Ok(true)
    }

    fn redraw(&mut self) -> BenchResult<bool> {
        if self.state.as_ref().unwrap().active.is_none() && !self.start_job()? {
            return Ok(false);
        }
        let state = self.state.as_mut().unwrap();
        state.retry_at = None;
        let active = state.active.as_mut().unwrap();
        if active.last_progress.elapsed() > Duration::from_secs(30) {
            return Err("surface did not present for 30 seconds".into());
        }
        let measured = active.frames >= self.options.warmup;
        if measured && active.start.is_none() {
            state.renderer.wait_idle(GPU_TIMEOUT)?;
            active.gpu_before = state.renderer.stats();
            active.start = Some(Instant::now());
            active.previous_present = None;
        }
        let (before, image_before, start, accumulated_ui) =
            active.image_transaction.unwrap_or_else(|| {
                (
                    active.scene.context.cache_stats(),
                    active.scene.context.image_metrics(),
                    Instant::now(),
                    Duration::ZERO,
                )
            });
        let input_start = Instant::now();
        if active.image_transaction.is_none() {
            if let Some(images) = &active.scene.images {
                if matches!(
                    images.case,
                    images::ImageCase::FirstShow | images::ImageCase::GpuRecovery
                ) {
                    state.renderer.clear_image_textures();
                }
            }
            active.scene.input(active.attempts);
            active.attempts += 1;
        }
        let input = input_start.elapsed();
        let ui_start = Instant::now();
        active.scene.build();
        let ui = ui_start.elapsed();
        let response = start.elapsed();
        let render_start = Instant::now();
        let status = state.renderer.render(
            active.scene.draw_data(),
            active.scene.context.style().background,
        )?;
        let render = render_start.elapsed();
        let wait_start = Instant::now();
        if self.options.gpu_wait && status == RenderStatus::Presented {
            state.renderer.wait_idle(GPU_TIMEOUT)?;
        }
        let completion = wait_start.elapsed();
        if self.options.gpu_timestamps && status == RenderStatus::Presented {
            if let Some(gpu) = state.renderer.read_render_gpu_time()? {
                if measured {
                    active.samples.stage("gpu_render_pass_timestamp", gpu);
                }
            }
        }
        let total = start.elapsed();
        let presented_at = Instant::now();
        active.scene.verify();
        if active
            .scene
            .images
            .as_ref()
            .is_some_and(|p| !p.ready(&active.scene.context))
        {
            active.image_transaction = Some((before, image_before, start, accumulated_ui + ui));
            active.last_progress = presented_at;
            return Ok(true);
        }
        active.image_transaction = None;
        if measured {
            active
                .counters
                .add_cpu(before, active.scene.context.cache_stats());
            active
                .counters
                .images(image_before, active.scene.context.image_metrics());
            active.samples.images(&mut active.scene.context);
            active.samples.renderer(&mut state.renderer);
            if active.scene.images.is_some() && active.samples.total.len().is_multiple_of(16) {
                active.samples.memory();
            }
        } else {
            active.scene.context.take_image_timings();
            state.renderer.take_renderer_timings();
        }
        match status {
            RenderStatus::Presented => {
                active.last_progress = presented_at;
                if measured {
                    active.samples.input.push(input);
                    active.samples.ui.push(ui + accumulated_ui);
                    active.samples.render.push(render);
                    active.samples.total.push(total);
                    if active.scene.images.is_some() && accumulated_ui > Duration::ZERO {
                        active.samples.stage("first_ready_present_wall", total);
                    }
                    if self.options.gpu_timestamps
                        && matches!(active.job.case, Case::Images(images::ImageCase::FirstShow))
                    {
                        if let Some(texture) =
                            active.scene.draw_data().textures.iter().find(|t| {
                                active.scene.draw_data().texture_options.contains_key(&t.id)
                            })
                        {
                            let upload = state.renderer.diagnostic_image_upload(texture)?;
                            for timing in upload.cpu_stages {
                                active.samples.stage(
                                    format!("explicit_copy_{:?}", timing.stage),
                                    timing.duration,
                                );
                            }
                            if let Some(gpu) = upload.gpu_transfer {
                                active
                                    .samples
                                    .stage("explicit_copy_gpu_transfer_timestamp", gpu);
                            }
                            active.samples.stage(
                                "explicit_copy_completion_inclusive",
                                upload.completion_inclusive,
                            );
                        }
                    }
                    if active.job.case.interactive() {
                        active.samples.response.push(response);
                    }
                    if self.options.gpu_wait {
                        active.samples.completion.push(completion);
                    }
                    if let Some(previous) = active.previous_present {
                        active.samples.cadence.push(presented_at - previous);
                    }
                }
                active.previous_present = Some(presented_at);
                active.frames += 1;
            }
            RenderStatus::Retry => {
                // Never report an unpresented frame as a successful timing sample.
                state.retry_at = Some(Instant::now() + Duration::from_millis(16));
            }
            RenderStatus::Dormant => {
                return Err("benchmark window is occluded or dormant; keep it visible".into())
            }
        }
        if active.frames == self.options.warmup + self.options.iterations {
            // Drain the final submissions so throughput does not hide a queued GPU tail.
            state.renderer.wait_idle(GPU_TIMEOUT)?;
            let active = state.active.take().unwrap();
            let elapsed = active.start.unwrap().elapsed();
            let mut counters = active.counters;
            counters.gpu(active.gpu_before, state.renderer.stats());
            if active.scene.images.is_none() {
                assert_eq!(counters.presents as usize, self.options.iterations);
            } else {
                assert!(counters.presents as usize >= self.options.iterations);
            }
            if let Case::Images(image) = active.job.case {
                if matches!(
                    image,
                    images::ImageCase::Warm | images::ImageCase::Shared | images::ImageCase::Unique
                ) {
                    assert_eq!(counters.texture_uploads, 0);
                    assert_eq!(counters.decodes, 0);
                    assert_eq!(counters.rasterizations, 0);
                    assert_eq!(counters.geometry_uploads, 0);
                }
                if image == images::ImageCase::Pixels {
                    assert_eq!(counters.geometry_uploads, 0);
                }
            }
            if matches!(
                active.job.case,
                Case::Cached
                    | Case::Repaint
                    | Case::Blur0
                    | Case::Blur8
                    | Case::Blur24
                    | Case::Blur64
                    | Case::BlurStack
                    | Case::ControlBlur
                    | Case::ProtocolCached
                    | Case::ColorPickerClosed
                    | Case::ColorPickerInternal
                    | Case::ColorPickerFloating
            ) {
                assert_eq!(
                    counters.geometry_uploads, 0,
                    "cached scene uploaded geometry"
                );
                assert_eq!(
                    counters.texture_uploads, 0,
                    "cached scene uploaded textures"
                );
            }
            if active.job.case == Case::ProtocolTexture {
                assert_eq!(counters.geometry_uploads, 0);
                assert_eq!(counters.texture_uploads as usize, self.options.iterations);
            }
            if active.job.case == Case::ProtocolGeometry {
                assert_eq!(counters.geometry_uploads as usize, self.options.iterations);
                assert_eq!(counters.texture_uploads, 0);
            }
            let mut row = ResultRow::new(
                "GPU",
                active.job.case.name(),
                active.job.count,
                match active.job.mode {
                    PresentationMode::Vsync => "Vsync",
                    PresentationMode::Immediate => "Immediate",
                },
                active.samples,
                elapsed,
                counters,
                Geometry::new(active.scene.draw_data()),
            );
            row.surface_retries = active.attempts.saturating_sub(active.frames);
            row.print();
            self.report.results.push(row);
            self.report.failed_case = None;
        }
        Ok(true)
    }
}

impl ApplicationHandler for GpuRunner<'_> {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _: ()) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let result = (|| -> BenchResult<GpuState> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("zaxis performance benchmark")
                        .with_inner_size(self.options.size)
                        .with_resizable(false),
                )?,
            );
            let start = Instant::now();
            let renderer = pollster::block_on(Renderer::new(Arc::clone(&window)))?;
            self.report.gpu_initialization_ms = Some(start.elapsed().as_secs_f64() * 1000.0);
            self.report.adapter = Some(format!("{:?}", renderer.adapter_info()));
            self.report.supported_present_modes = renderer
                .supported_present_modes()
                .iter()
                .map(|mode| format!("{mode:?}"))
                .collect();
            let size = window.inner_size();
            self.report.physical_size = [size.width, size.height];
            self.report.scale_factor = window.scale_factor();
            println!(
                "GPU: {:?}; modes {:?}; initialization {:.1} ms; native DPI {}",
                renderer.adapter_info(),
                self.report.supported_present_modes,
                start.elapsed().as_secs_f64() * 1000.0,
                window.scale_factor()
            );
            window.request_redraw();
            Ok(GpuState {
                window,
                renderer,
                active: None,
                retry_at: None,
            })
        })();
        match result {
            Ok(state) => self.state = Some(state),
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.fail(
            event_loop,
            "benchmark interrupted by window suspension".into(),
        );
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if !self.state.as_ref().is_some_and(|s| s.window.id() == id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Occluded(true) => {
                self.fail(event_loop, "benchmark window was occluded".into())
            }
            WindowEvent::Resized(size) => {
                let state = self.state.as_mut().unwrap();
                if size.width == 0 || size.height == 0 {
                    self.fail(event_loop, "benchmark window was minimized".into());
                } else if let Err(error) = state.renderer.resize(size) {
                    self.fail(event_loop, Box::new(error));
                } else if state.active.is_some() {
                    self.fail(
                        event_loop,
                        "viewport changed during a benchmark case".into(),
                    );
                }
            }
            WindowEvent::RedrawRequested => match self.redraw() {
                Ok(true) => {}
                Ok(false) => {
                    self.finished = true;
                    event_loop.exit();
                }
                Err(error) => self.fail(event_loop, error),
            },
            _ => {} // Live mouse/keyboard input must not contaminate scripted scenes.
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &self.state else {
            return;
        };
        if let Some(deadline) = state.retry_at.filter(|t| *t > Instant::now()) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        } else {
            if state
                .active
                .as_ref()
                .is_some_and(|a| a.image_transaction.is_some() && !a.scene.context.needs_repaint())
            {
                event_loop.set_control_flow(
                    state
                        .active
                        .as_ref()
                        .and_then(|a| a.scene.context.next_repaint())
                        .map_or(ControlFlow::Wait, ControlFlow::WaitUntil),
                );
                return;
            }
            state.window.request_redraw();
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
