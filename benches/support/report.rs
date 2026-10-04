use serde::Serialize;
use std::time::Duration;
use zaxis::{CacheStats, DrawData, Rect, RendererStats, Vec2};

#[derive(Default, Serialize)]
pub struct Distribution {
    pub samples: usize,
    pub mean_ms: f64,
    pub min_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

impl Distribution {
    pub fn new(values: &[Duration]) -> Self {
        if values.is_empty() {
            return Self::default();
        }
        let mut sorted: Vec<_> = values.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
        sorted.sort_by(f64::total_cmp);
        let percentile =
            |p: f64| sorted[((sorted.len() as f64 * p).ceil() as usize).saturating_sub(1)];
        Self {
            samples: sorted.len(),
            mean_ms: sorted.iter().sum::<f64>() / sorted.len() as f64,
            min_ms: sorted[0],
            p50_ms: percentile(0.50),
            p95_ms: percentile(0.95),
            p99_ms: percentile(0.99),
            max_ms: *sorted.last().unwrap(),
        }
    }
}

#[derive(Default)]
pub struct Samples {
    pub process_memory: Vec<ProcessMemory>,
    memory_system: sysinfo::System,
    pub input: Vec<Duration>,
    pub ui: Vec<Duration>,
    pub response: Vec<Duration>,
    pub render: Vec<Duration>,
    pub completion: Vec<Duration>,
    pub total: Vec<Duration>,
    pub cadence: Vec<Duration>,
    pub stages: std::collections::BTreeMap<String, Vec<Duration>>,
}

impl Samples {
    /// Outside measured UI work. Windows virtual_memory is PROCESS_MEMORY_COUNTERS_EX.PrivateUsage.
    pub fn memory(&mut self) {
        let Ok(pid) = sysinfo::get_current_pid() else {
            return;
        };
        self.memory_system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[pid]),
            false,
            sysinfo::ProcessRefreshKind::nothing().with_memory(),
        );
        if let Some(process) = self.memory_system.process(pid) {
            self.process_memory.push(ProcessMemory {
                sample: self.total.len(),
                resident_bytes: process.memory(),
                private_commit_bytes: cfg!(windows).then(|| process.virtual_memory()),
            });
        }
    }
    pub fn stage(&mut self, name: impl Into<String>, duration: Duration) {
        self.stages.entry(name.into()).or_default().push(duration);
    }
    pub fn images(&mut self, context: &mut zaxis::Context) {
        for t in context.take_image_timings() {
            self.stage(format!("image_{:?}", t.stage), t.duration);
        }
    }
    pub fn renderer(&mut self, renderer: &mut zaxis::Renderer) {
        for t in renderer.take_renderer_timings() {
            self.stage(format!("backend_{:?}", t.stage), t.duration);
        }
    }
    pub fn with_capacity(n: usize) -> Self {
        Self {
            process_memory: Vec::new(),
            memory_system: sysinfo::System::new(),
            input: Vec::with_capacity(n),
            ui: Vec::with_capacity(n),
            response: Vec::with_capacity(n),
            render: Vec::with_capacity(n),
            completion: Vec::with_capacity(n),
            total: Vec::with_capacity(n),
            cadence: Vec::with_capacity(n),
            stages: Default::default(),
        }
    }
}

#[derive(Serialize)]
pub struct ProcessMemory {
    pub sample: usize,
    pub resident_bytes: u64,
    pub private_commit_bytes: Option<u64>,
}

#[derive(Default, Serialize)]
pub struct Counters {
    pub ui_passes: u64,
    pub tessellated_elements: u64,
    pub reused_elements: u64,
    pub geometry_rebuilds: u64,
    pub geometry_full_rebuilds: u64,
    pub geometry_partial_updates: u64,
    pub geometry_bytes_copied: u64,
    pub text_layouts_built: u64,
    pub presents: u64,
    pub geometry_uploads: u64,
    pub texture_uploads: u64,
    pub geometry_upload_bytes: u64,
    pub decodes: u64,
    pub svg_parses: u64,
    pub rasterizations: u64,
    pub downsamplings: u64,
    pub image_cache_hits: u64,
    pub image_cache_misses: u64,
    pub cpu_evictions: u64,
    pub stale_completions: u64,
    pub cpu_image_resident_bytes: usize,
    pub gpu_image_resident_bytes_estimate: u64,
    pub texture_creations: u64,
    pub texture_reuploads: u64,
    pub texture_upload_bytes: u64,
    pub texture_evictions: u64,
    pub draw_calls: u64,
}

impl Counters {
    pub fn images(&mut self, before: zaxis::ImageMetrics, after: zaxis::ImageMetrics) {
        self.decodes += after.decodes - before.decodes;
        self.svg_parses += after.svg_parses - before.svg_parses;
        self.rasterizations += after.rasterizations - before.rasterizations;
        self.downsamplings += after.downsamplings - before.downsamplings;
        self.image_cache_hits += after.cache_hits - before.cache_hits;
        self.image_cache_misses += after.cache_misses - before.cache_misses;
        self.cpu_evictions += after.evictions - before.evictions;
        self.stale_completions += after.stale_completions - before.stale_completions;
        self.cpu_image_resident_bytes = after.cpu_resident_bytes;
    }
    pub fn add_cpu(&mut self, before: CacheStats, after: CacheStats) {
        let delta = Self::cpu(before, after);
        self.ui_passes += delta.ui_passes;
        self.tessellated_elements += delta.tessellated_elements;
        self.reused_elements += delta.reused_elements;
        self.geometry_rebuilds += delta.geometry_rebuilds;
        self.geometry_full_rebuilds += delta.geometry_full_rebuilds;
        self.geometry_partial_updates += delta.geometry_partial_updates;
        self.geometry_bytes_copied += delta.geometry_bytes_copied;
        self.text_layouts_built += delta.text_layouts_built;
    }
    pub fn cpu(before: CacheStats, after: CacheStats) -> Self {
        Self {
            ui_passes: after.ui_passes - before.ui_passes,
            tessellated_elements: after.tessellated_elements - before.tessellated_elements,
            reused_elements: after.reused_elements - before.reused_elements,
            geometry_rebuilds: after.geometry_rebuilds - before.geometry_rebuilds,
            geometry_full_rebuilds: after.geometry_full_rebuilds - before.geometry_full_rebuilds,
            geometry_partial_updates: after.geometry_partial_updates
                - before.geometry_partial_updates,
            geometry_bytes_copied: after.geometry_bytes_copied - before.geometry_bytes_copied,
            text_layouts_built: after.text_layouts_built - before.text_layouts_built,
            ..Self::default()
        }
    }
    pub fn gpu(&mut self, before: RendererStats, after: RendererStats) {
        self.texture_creations = after.texture_creations - before.texture_creations;
        self.texture_reuploads = after.texture_reuploads - before.texture_reuploads;
        self.texture_upload_bytes = after.texture_upload_bytes - before.texture_upload_bytes;
        self.texture_evictions = after.texture_evictions - before.texture_evictions;
        self.draw_calls = after.draw_calls - before.draw_calls;
        self.gpu_image_resident_bytes_estimate = after.image_resident_bytes_estimate;
        self.presents = after.presented_frames - before.presented_frames;
        self.geometry_uploads = after.geometry_uploads - before.geometry_uploads;
        self.texture_uploads = after.texture_uploads - before.texture_uploads;
        self.geometry_upload_bytes = after.geometry_upload_bytes - before.geometry_upload_bytes;
    }
}

#[derive(Default, Serialize)]
pub struct Geometry {
    pub managed_texture_budget_bytes: u64,
    pub managed_texture_binding_budget: usize,
    pub logical_size: [f32; 2],
    pub scale_factor: f32,
    pub vertices: usize,
    pub indices: usize,
    pub draw_commands: usize,
    pub visible_commands: usize,
    pub blur_passes: usize,
    pub texture_bytes: usize,
    pub mesh_bytes: usize,
}

impl Geometry {
    pub fn new(data: &DrawData) -> Self {
        let viewport = Rect::from_min_size(Vec2::ZERO, data.logical_size);
        Self {
            managed_texture_budget_bytes: data.texture_budget_bytes,
            managed_texture_binding_budget: data.texture_binding_budget,
            logical_size: data.logical_size.to_array(),
            scale_factor: data.scale_factor,
            vertices: data.vertices.len(),
            indices: data.indices.len(),
            draw_commands: data.commands.len(),
            visible_commands: data
                .commands
                .iter()
                .filter(|c| !c.clip_rect.intersect(viewport).is_empty())
                .count(),
            blur_passes: data.commands.iter().filter(|c| c.blur.is_some()).count(),
            texture_bytes: data.textures.iter().map(|t| t.pixels.len()).sum(),
            mesh_bytes: data.vertices.len() * std::mem::size_of::<zaxis::Vertex>()
                + data.indices.len() * std::mem::size_of::<u32>(),
        }
    }
}

#[derive(Serialize)]
pub struct ResultRow {
    pub stage_availability: std::collections::BTreeMap<String, String>,
    pub image_asset_format: Option<String>,
    pub image_asset_alpha: Option<String>,
    pub process_memory: Vec<ProcessMemory>,
    pub unsupported: Option<String>,
    pub stages: std::collections::BTreeMap<String, Distribution>,
    pub image_asset_dimensions: Option<[usize; 2]>,
    pub suite: String,
    pub case: String,
    pub objects: usize,
    pub presentation: String,
    pub verified: bool,
    pub elapsed_seconds: f64,
    pub throughput_fps: f64,
    pub frames_over_16_67_ms: usize,
    pub frames_over_33_33_ms: usize,
    pub surface_retries: usize,
    pub input: Distribution,
    pub ui: Distribution,
    pub response: Distribution,
    pub render_present: Distribution,
    pub gpu_wait: Distribution,
    pub total: Distribution,
    pub cadence: Distribution,
    pub counters: Counters,
    pub geometry: Geometry,
}

impl ResultRow {
    pub fn new(
        suite: &str,
        case: &str,
        objects: usize,
        presentation: &str,
        samples: Samples,
        elapsed: Duration,
        counters: Counters,
        geometry: Geometry,
    ) -> Self {
        let budget_samples = if samples.cadence.is_empty() {
            &samples.total
        } else {
            &samples.cadence
        };
        Self {
            stage_availability:[
                ("gpu_allocation","unavailable: driver allocation has no timestamp boundary"),
                ("production_gpu_transfer","unavailable: queue.write_texture does not expose a timed transfer pass"),
                ("gpu_render_pass",if samples.stages.contains_key("gpu_render_pass_timestamp") { "adapter timestamp queries after completion" } else { "unavailable: CPU suite or timestamps disabled/unsupported" }),
                ("diagnostic_gpu_transfer",if samples.stages.contains_key("explicit_copy_gpu_transfer_timestamp") { "adapter timestamps around explicit buffer-to-texture copy; separate diagnostic path" } else { "unavailable: diagnostic not run or encoder timestamps unsupported" }),
            ].into_iter().map(|(k,v)|(k.into(),v.into())).collect(),
            image_asset_format:case.starts_with("image_").then(|| if case.contains("svg") { "SVG" } else if case.contains("jpeg") { "JPEG" } else if case.contains("webp") { "WebP" } else if case.contains("pixels") || case.contains("dimensions") { "RGBA8" } else if case.contains("empty") { "none" } else { "PNG" }.into()),
            image_asset_alpha:case.starts_with("image_").then(|| if case.contains("jpeg") { "opaque" } else { "opaque and translucent pixels" }.into()),
            process_memory: samples.process_memory,
            unsupported: None,
            stages: samples
                .stages
                .iter()
                .map(|(key, durations)| (key.clone(), Distribution::new(durations)))
                .collect(),
            image_asset_dimensions: case.starts_with("image_").then_some([objects; 2]),
            suite: suite.into(),
            case: case.into(),
            objects,
            presentation: presentation.into(),
            verified: true,
            elapsed_seconds: elapsed.as_secs_f64(),
            throughput_fps: samples.total.len() as f64 / elapsed.as_secs_f64(),
            frames_over_16_67_ms: budget_samples
                .iter()
                .filter(|d| d.as_secs_f64() > 1.0 / 60.0)
                .count(),
            frames_over_33_33_ms: budget_samples
                .iter()
                .filter(|d| d.as_secs_f64() > 1.0 / 30.0)
                .count(),
            surface_retries: 0,
            input: Distribution::new(&samples.input),
            ui: Distribution::new(&samples.ui),
            response: Distribution::new(&samples.response),
            render_present: Distribution::new(&samples.render),
            gpu_wait: Distribution::new(&samples.completion),
            total: Distribution::new(&samples.total),
            cadence: Distribution::new(&samples.cadence),
            counters,
            geometry,
        }
    }
    pub fn print(&self) {
        println!("{:<4} {:<21} {:>5} {:<9} UI {:>8.3}  frame {:>8.3}/{:>8.3}/{:>8.3} ms  {:>8.1} fps  mesh {:>5} uploads {:>5}/{:>4}",
            self.suite, self.case, self.objects, self.presentation, self.ui.p50_ms,
            self.total.p50_ms, self.total.p95_ms, self.total.p99_ms, self.throughput_fps,
            self.counters.geometry_rebuilds, self.counters.geometry_uploads, self.counters.texture_uploads);
    }
}

#[derive(Serialize)]
pub struct FailedCase {
    pub suite: String,
    pub case: String,
    pub objects: usize,
    pub presentation: String,
}

impl FailedCase {
    pub fn new(suite: &str, case: &str, objects: usize, presentation: &str) -> Self {
        Self {
            suite: suite.into(),
            case: case.into(),
            objects,
            presentation: presentation.into(),
        }
    }
}

#[derive(Serialize)]
pub struct Report {
    pub cpu: String,
    pub gpu_timestamps: bool,
    pub schema_version: u32,
    pub completed: bool,
    pub failure: Option<String>,
    pub failed_case: Option<FailedCase>,
    pub max_p95_ms: Option<f64>,
    pub library_version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub unix_time_seconds: u64,
    pub release_build: bool,
    pub gpu_wait: bool,
    pub warmup: usize,
    pub iterations: usize,
    pub sizes: Vec<usize>,
    pub physical_size: [u32; 2],
    pub scale_factor: f64,
    pub adapter: Option<String>,
    pub gpu_initialization_ms: Option<f64>,
    pub supported_present_modes: Vec<String>,
    pub timer_pair_p50_ms: f64,
    pub results: Vec<ResultRow>,
}

pub fn timer_overhead() -> f64 {
    let durations: Vec<_> = (0..1000)
        .map(|_| std::time::Instant::now().elapsed())
        .collect();
    Distribution::new(&durations).p50_ms
}
