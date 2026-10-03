use super::*;
pub(super) fn run_suites(
    options: &Options,
    cases: &[Case],
    report: &mut Report,
) -> BenchResult<()> {
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

pub(super) fn unsupported(
    suite: &str,
    case: Case,
    count: usize,
    mode: &str,
    reason: String,
) -> ResultRow {
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
