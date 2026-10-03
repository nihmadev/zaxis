use super::*;
#[derive(Clone, Copy)]
pub(super) struct Job {
    pub(super) mode: PresentationMode,
    pub(super) count: usize,
    pub(super) case: Case,
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

pub(super) struct GpuState {
    window: Arc<Window>,
    renderer: Renderer,
    active: Option<ActiveCase>,
    retry_at: Option<Instant>,
}

pub(super) struct GpuRunner<'a> {
    pub(super) options: &'a Options,
    pub(super) report: &'a mut Report,
    pub(super) jobs: Vec<Job>,
    pub(super) next_job: usize,
    pub(super) state: Option<GpuState>,
    pub(super) error: Option<Box<dyn Error>>,
    pub(super) finished: bool,
    pub(super) image_waker: Arc<dyn Fn() + Send + Sync>,
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

#[path = "gpu/events.rs"]
mod events;
