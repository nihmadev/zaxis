use super::{
    decode::{self, Document},
    source::Source,
    DecodedImage, ImageDecoder, ImageError, ImageLimits, ImageStage, ImageTiming,
};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};

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
struct Shared {
    jobs: Mutex<VecDeque<Job>>,
    results: Mutex<Vec<Completion>>,
    changed: Condvar,
    alive: AtomicBool,
    ready: AtomicBool,
    waker: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}
pub(super) struct Workers {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
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
            threads: Vec::new(),
        }
    }
    pub fn set_waker(&self, callback: Option<Arc<dyn Fn() + Send + Sync>>) {
        *self.shared.waker.lock().unwrap() = callback;
        if self.has_results() {
            wake(&self.shared);
        }
    }
    pub fn has_results(&self) -> bool {
        self.shared.ready.load(Ordering::Acquire)
    }
    pub fn submit(&mut self, job: Job) -> Result<(), ImageError> {
        if self.threads.is_empty() {
            for i in 0..2 {
                let shared = self.shared.clone();
                self.threads.push(
                    std::thread::Builder::new()
                        .name(format!("zaxis image {i}"))
                        .spawn(move || run(shared))
                        .map_err(|e| ImageError(format!("image worker: {e}")))?,
                );
            }
        }
        self.shared.jobs.lock().unwrap().push_back(job);
        self.shared.changed.notify_one();
        Ok(())
    }
    pub fn drain(&self) -> Vec<Completion> {
        let mut results = self.shared.results.lock().unwrap();
        self.shared.ready.store(false, Ordering::Release);
        std::mem::take(&mut *results)
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
fn run(shared: Arc<Shared>) {
    loop {
        let job = {
            let mut jobs = shared.jobs.lock().unwrap();
            while jobs.is_empty() && shared.alive.load(Ordering::Acquire) {
                jobs = shared.changed.wait(jobs).unwrap();
            }
            if !shared.alive.load(Ordering::Acquire) {
                return;
            }
            jobs.pop_front().unwrap()
        };
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
        let timings = combined;
        if !shared.alive.load(Ordering::Acquire) {
            return;
        }
        {
            let mut results = shared.results.lock().unwrap();
            results.push(Completion {
                id: job.id,
                generation: job.generation,
                loaded,
                reserved: job.reserved,
                output,
                timings,
            });
            shared.ready.store(true, Ordering::Release);
        }
        wake(&shared);
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
        self.threads.clear();
    }
}
