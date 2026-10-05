//! Image jobs and where they run: on worker threads, or on the drawing thread in small
//! per-frame slices where threads are unavailable (`wasm32`, or by request).

#[cfg(not(target_arch = "wasm32"))]
mod threads;

use super::{
    decode::{self, Document},
    source::Source,
    DecodedImage, ImageDecoder, ImageError, ImageLimits, ImageStage, ImageTiming,
};
use crate::time::Instant;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};

/// Time one frame may spend decoding when jobs run on the drawing thread. At least one job
/// runs per frame, so a single large image can still exceed it.
pub(super) const INLINE_FRAME_BUDGET: Duration = Duration::from_millis(6);

pub(super) enum Work {
    Load(Source, Vec<Arc<dyn ImageDecoder>>),
    Variant(Document),
}
pub(super) struct Job {
    pub id: u64,
    pub generation: u64,
    pub size: [u32; 2],
    pub linear: bool,
    pub raster_size: [u32; 2],
    pub limits: ImageLimits,
    pub work: Work,
    pub queued_at: Instant,
    pub reserved: usize,
}
pub(super) struct Completion {
    pub id: u64,
    pub generation: u64,
    pub loaded: bool,
    pub reserved: usize,
    pub output: Result<(Document, DecodedImage), ImageError>,
    pub timings: Vec<ImageTiming>,
}
pub(super) struct Shared {
    jobs: Mutex<VecDeque<Job>>,
    results: Mutex<Vec<Completion>>,
    changed: Condvar,
    alive: AtomicBool,
    ready: AtomicBool,
    waker: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}
pub(super) struct Workers {
    shared: Arc<Shared>,
    #[cfg(not(target_arch = "wasm32"))]
    threads: Vec<std::thread::JoinHandle<()>>,
    /// `Some(budget)`: decode on the thread that drains results instead of on workers.
    inline: Option<Duration>,
}
impl Workers {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                jobs: Mutex::new(VecDeque::new()),
                results: Mutex::new(Vec::new()),
                changed: Condvar::new(),
                alive: AtomicBool::new(true),
                ready: AtomicBool::new(false),
                waker: Mutex::new(None),
            }),
            #[cfg(not(target_arch = "wasm32"))]
            threads: Vec::new(),
            inline: cfg!(target_arch = "wasm32").then_some(INLINE_FRAME_BUDGET),
        }
    }
    /// Decode at most `frame_budget` per frame on the drawing thread (`Some`), or on
    /// worker threads (`None`; on `wasm32`, which has none, the default budget stays).
    pub fn set_inline(&mut self, frame_budget: Option<Duration>) {
        self.inline = frame_budget.or(cfg!(target_arch = "wasm32").then_some(INLINE_FRAME_BUDGET));
    }
    pub fn set_waker(&self, callback: Option<Arc<dyn Fn() + Send + Sync>>) {
        *self.shared.waker.lock().unwrap() = callback;
        if self.has_results() {
            wake(&self.shared);
        }
    }
    /// Finished work is waiting, or queued work still needs a frame to run in.
    pub fn has_results(&self) -> bool {
        self.shared.ready.load(Ordering::Acquire)
            || (self.inline.is_some() && !self.shared.jobs.lock().unwrap().is_empty())
    }
    pub fn submit(&mut self, job: Job) -> Result<(), ImageError> {
        if self.inline.is_none() {
            self.spawn_threads()?;
        }
        self.shared.jobs.lock().unwrap().push_back(job);
        self.shared.changed.notify_one();
        if self.inline.is_some() {
            wake(&self.shared);
        }
        Ok(())
    }
    pub fn drain(&self) -> Vec<Completion> {
        if let Some(budget) = self.inline {
            self.run_inline(budget);
        }
        let mut results = self.shared.results.lock().unwrap();
        self.shared.ready.store(false, Ordering::Release);
        std::mem::take(&mut *results)
    }
    /// Decode queued jobs until the budget is spent; the first job always runs.
    fn run_inline(&self, budget: Duration) {
        let start = Instant::now();
        loop {
            let Some(job) = self.shared.jobs.lock().unwrap().pop_front() else {
                return;
            };
            let completion = process(job);
            self.shared.results.lock().unwrap().push(completion);
            self.shared.ready.store(true, Ordering::Release);
            if start.elapsed() >= budget {
                return;
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn spawn_threads(&mut self) -> Result<(), ImageError> {
        Err(ImageError(
            "image worker threads are unavailable on this target".into(),
        ))
    }
}
fn wake(shared: &Shared) {
    // Serialize removal of the callback with invocation: after Drop returns no
    // worker can call a host waker. Callbacks must only enqueue an event.
    let callback = shared.waker.lock().unwrap();
    if shared.alive.load(Ordering::Acquire) {
        if let Some(w) = callback.as_ref() {
            w();
        }
    }
}
/// Run one job to completion: decode or rasterize, then combine the stage timings.
fn process(job: Job) -> Completion {
    let service = Instant::now();
    let loaded = matches!(job.work, Work::Load(..));
    let mut timings = vec![ImageTiming {
        stage: ImageStage::Queue,
        duration: service - job.queued_at,
    }];
    let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let document = match job.work {
            Work::Load(source, decoders) => {
                decode::load(&source, &job.limits, &decoders, &mut timings)?
            }
            Work::Variant(doc) => doc,
        };
        let pixels = match &document {
            Document::Svg { tree, .. } => {
                decode::rasterize(tree, job.size, &job.limits, &mut timings)?
            }
            Document::Raster(r) => {
                let ratio = (job.raster_size[0] as f32 / r.size[0] as f32)
                    .max(job.raster_size[1] as f32 / r.size[1] as f32);
                if job.linear && ratio < 0.5 {
                    let size = [
                        (r.size[0] as f32 * ratio).ceil().max(1.0) as u32,
                        (r.size[1] as f32 * ratio).ceil().max(1.0) as u32,
                    ];
                    decode::timed(&mut timings, ImageStage::Downsample, || {
                        super::resize::downsample(r, size, &job.limits)
                    })?
                } else {
                    r.clone()
                }
            }
        };
        Ok((document, pixels))
    }))
    .unwrap_or_else(|_| {
        Err(ImageError(
            "image decoder panicked; source was rejected".into(),
        ))
    });
    timings.push(ImageTiming {
        stage: ImageStage::Service,
        duration: service.elapsed(),
    });
    let mut combined: Vec<ImageTiming> = Vec::new();
    for timing in timings {
        if let Some(t) = combined.iter_mut().find(|t| t.stage == timing.stage) {
            t.duration += timing.duration;
        } else {
            combined.push(timing);
        }
    }
    Completion {
        id: job.id,
        generation: job.generation,
        loaded,
        reserved: job.reserved,
        output,
        timings: combined,
    }
}
impl Drop for Workers {
    fn drop(&mut self) {
        self.shared.alive.store(false, Ordering::Release);
        *self.shared.waker.lock().unwrap() = None;
        self.shared.jobs.lock().unwrap().clear();
        self.shared.changed.notify_all();
        // Never join a codec while dropping the UI. Idle workers exit immediately;
        // bounded in-flight work exits after its current decode without publication.
        #[cfg(not(target_arch = "wasm32"))]
        self.threads.clear();
    }
}
