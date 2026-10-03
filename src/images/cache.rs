use super::{
    decode::Document,
    source::Source,
    worker::{Job, Work, Workers},
    DecodedImage, ImageDecoder, ImageError, ImageHandle, ImageSource, TextureIds,
};
use crate::{TextureFilter, TextureId, TextureImage, TextureOptions, Vec2};
use std::{
    collections::{HashMap, VecDeque},
    path::{Component, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
mod lifecycle;

mod types;
pub use types::*;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum Key {
    Path(PathBuf),
    Static(usize, usize),
    Shared(usize, [u32; 2]),
    Handle(u64),
    Version(Box<Key>, u64),
}

struct Entry {
    source: Source,
    generation: u64,
    ids: [TextureId; 2],
    revisions: [u64; 2],
    document: Option<Document>,
    displayed: Option<DecodedImage>,
    variants: VecDeque<DecodedImage>,
    error: Option<ImageError>,
    pending: bool,
    pinned: bool,
    last_frame: u64,
    wanted: [u32; 2],
    linear_wanted: [u32; 2],
    target: [u32; 2],
    changed_at: Instant,
    requested_at: Instant,
    observed_ready: bool,
    intrinsic: Option<[u32; 2]>,
}
impl Entry {
    fn state(&self) -> ImageState {
        if let Some(e) = &self.error {
            return ImageState::Error(e.clone());
        }
        let size = self
            .document
            .as_ref()
            .map(Document::size)
            .or(self.intrinsic)
            .or_else(|| match self.source {
                Source::Rgba(s, _) => Some(s),
                _ => None,
            })
            .map(|[w, h]| Vec2::new(w as f32, h as f32));
        if self.displayed.is_some() {
            ImageState::Ready {
                size: size.unwrap(),
            }
        } else {
            ImageState::Loading { size }
        }
    }
    fn bytes(&self) -> usize {
        let input = match &self.source {
            Source::Encoded(p) | Source::Rgba(_, p) => p.len(),
            _ => 0,
        };
        let doc = self.document.as_ref().map_or(0, Document::bytes);
        let base = match &self.document {
            Some(Document::Raster(r)) => Some(&r.pixels),
            _ => None,
        };
        // Count shared original/variant/display buffers once.
        let mut seen = Vec::new();
        if let Some(p) = base {
            seen.push(Arc::as_ptr(p));
        }
        let mut extra = 0;
        for r in self.variants.iter().chain(self.displayed.iter()) {
            if !seen.contains(&Arc::as_ptr(&r.pixels)) {
                seen.push(Arc::as_ptr(&r.pixels));
                extra += r.pixels.len();
            }
        }
        let shared_input =
            matches!((&self.source, base), (Source::Rgba(_,p), Some(b)) if Arc::ptr_eq(p,b));
        input + doc + extra - if shared_input { input } else { 0 }
    }
}

pub(crate) struct ImageCache {
    owner: u64,
    ids: TextureIds,
    next_entry: u64,
    next_generation: u64,
    keys: HashMap<Key, u64>,
    entries: HashMap<u64, Entry>,
    base_dir: PathBuf,
    pub(crate) limits: ImageLimits,
    workers: Workers,
    decoders: Vec<Arc<dyn ImageDecoder>>,
    frame: u64,
    now: Instant,
    metrics: ImageMetrics,
    timings: VecDeque<ImageTiming>,
    inflight_bytes: usize,
    placeholder: Arc<Vec<u8>>,
    pub(crate) state_changed: bool,
}
impl ImageCache {
    pub fn new(owner: u64, ids: TextureIds) -> Self {
        Self {
            owner,
            ids,
            next_entry: 1,
            next_generation: 1,
            keys: HashMap::new(),
            entries: HashMap::new(),
            base_dir: std::env::current_dir().unwrap_or_default(),
            limits: ImageLimits::default(),
            workers: Workers::new(),
            decoders: Vec::new(),
            frame: 0,
            now: Instant::now(),
            metrics: ImageMetrics::default(),
            timings: VecDeque::new(),
            inflight_bytes: 0,
            placeholder: Arc::new(vec![180, 180, 180, 32]),
            state_changed: false,
        }
    }
    fn key(&self, source: &Source) -> Key {
        match source {
            Source::Path(p) => {
                let absolute = if p.is_absolute() {
                    p.as_ref().clone()
                } else {
                    self.base_dir.join(p.as_ref())
                };
                let mut path = PathBuf::new();
                for c in absolute.components() {
                    match c {
                        Component::CurDir => {}
                        Component::ParentDir => {
                            path.pop();
                        }
                        _ => path.push(c.as_os_str()),
                    }
                }
                Key::Path(path)
            }
            Source::Static(p) => Key::Static(p.as_ptr() as usize, p.len()),
            Source::Encoded(p) => Key::Shared(Arc::as_ptr(p) as usize, [0, 0]),
            Source::Rgba(s, p) => Key::Shared(Arc::as_ptr(p) as usize, *s),
            Source::Handle(h) => Key::Handle(h.id),
        }
    }
    pub fn resolve(&mut self, source: ImageSource) -> Result<ImageHandle, ImageError> {
        let start = Instant::now();
        let result = self.resolve_inner(source);
        self.record(ImageTiming {
            stage: ImageStage::Lookup,
            duration: start.elapsed(),
        });
        result
    }
    fn resolve_inner(&mut self, source: ImageSource) -> Result<ImageHandle, ImageError> {
        if let Source::Handle(h) = source.0 {
            if h.owner != self.owner || !self.entries.contains_key(&h.id) {
                return Err(ImageError(
                    "image handle belongs to another Context or was released".into(),
                ));
            }
            self.metrics.cache_hits += 1;
            self.entries.get_mut(&h.id).unwrap().last_frame = self.frame;
            return Ok(h);
        }
        let base_key = self.key(&source.0);
        let key = if source.1 == 0 {
            base_key.clone()
        } else {
            Key::Version(Box::new(base_key.clone()), source.1)
        };
        if let Some(&id) = self.keys.get(&key) {
            self.metrics.cache_hits += 1;
            self.entries.get_mut(&id).unwrap().last_frame = self.frame;
            return Ok(ImageHandle {
                owner: self.owner,
                id,
            });
        }
        self.evict_to_fit(0);
        if self.entries.len() >= self.limits.max_entries {
            return Err(ImageError(
                "image cache entry limit reached; release unused handles".into(),
            ));
        }
        let bytes = match &source.0 {
            Source::Encoded(p) | Source::Rgba(_, p) => p.len(),
            _ => 0,
        };
        self.evict_to_fit(bytes);
        if self.resident_bytes().saturating_add(bytes) > self.limits.cpu_cache_bytes {
            return Err(ImageError("image source exceeds CPU cache budget".into()));
        }
        let source = match &base_key {
            Key::Path(p) => Source::Path(Arc::new(p.clone())),
            _ => source.0,
        };
        let id = self.next_entry;
        self.next_entry += 1;
        let generation = self.generation();
        self.entries.insert(
            id,
            Entry {
                source,
                generation,
                ids: [self.ids.next(), self.ids.next()],
                revisions: [0; 2],
                document: None,
                displayed: None,
                variants: VecDeque::new(),
                error: None,
                pending: false,
                pinned: false,
                last_frame: self.frame,
                wanted: [0; 2],
                linear_wanted: [0; 2],
                target: [0; 2],
                changed_at: self.now,
                requested_at: Instant::now(),
                observed_ready: false,
                intrinsic: None,
            },
        );
        self.keys.insert(key, id);
        self.metrics.cache_misses += 1;
        Ok(ImageHandle {
            owner: self.owner,
            id,
        })
    }
    fn generation(&mut self) -> u64 {
        let n = self.next_generation;
        self.next_generation += 1;
        n
    }
    pub fn state(&self, source: ImageSource) -> ImageState {
        let id = if let Source::Handle(h) = source.0 {
            if h.owner != self.owner {
                return ImageState::Error(ImageError("foreign image handle".into()));
            }
            Some(h.id)
        } else {
            let key = self.key(&source.0);
            self.keys
                .get(&if source.1 == 0 {
                    key
                } else {
                    Key::Version(Box::new(key), source.1)
                })
                .copied()
        };
        id.and_then(|id| self.entries.get(&id))
            .map_or(ImageState::Loading { size: None }, Entry::state)
    }
    pub fn texture(&self, h: ImageHandle, filter: TextureFilter) -> Option<TextureId> {
        let e = self.entries.get(&h.id)?;
        Some(e.ids[usize::from(filter == TextureFilter::Nearest)])
    }
    pub fn request(&mut self, h: ImageHandle, size: Vec2, texture: TextureId) {
        if let Some(e) = self.entries.get_mut(&h.id) {
            if !size.is_finite() || size.min_element() <= 0.0 {
                return;
            }
            let size = [
                size.x.ceil().min(u32::MAX as f32) as u32,
                size.y.ceil().min(u32::MAX as f32) as u32,
            ];
            e.wanted = [e.wanted[0].max(size[0]), e.wanted[1].max(size[1])];
            if texture == e.ids[0] {
                e.linear_wanted = [
                    e.linear_wanted[0].max(size[0]),
                    e.linear_wanted[1].max(size[1]),
                ];
            }
            e.last_frame = self.frame;
        }
    }
    pub fn metrics(&self) -> ImageMetrics {
        ImageMetrics {
            cpu_resident_bytes: self.resident_bytes(),
            ..self.metrics
        }
    }
    pub fn take_timings(&mut self) -> Vec<ImageTiming> {
        self.timings.drain(..).collect()
    }
    fn record(&mut self, timing: ImageTiming) {
        if self.timings.len() == 1024 {
            self.timings.pop_front();
        }
        self.timings.push_back(timing);
    }
    fn resident_bytes(&self) -> usize {
        self.entries.values().map(Entry::bytes).sum()
    }
    pub fn add_decoder(&mut self, decoder: Arc<dyn ImageDecoder>) {
        self.decoders.push(decoder);
    }
    pub fn set_waker(&mut self, waker: Option<Arc<dyn Fn() + Send + Sync>>) {
        self.workers.set_waker(waker);
    }
    pub fn has_results(&self) -> bool {
        self.workers.has_results()
    }
    pub fn payloads(
        &mut self,
        active: &std::collections::HashSet<TextureId>,
    ) -> (Vec<TextureImage>, HashMap<TextureId, TextureOptions>) {
        let mut images = Vec::new();
        let mut options = HashMap::new();
        let mut ready = Vec::new();
        for e in self.entries.values_mut() {
            let placeholder = DecodedImage {
                size: [1, 1],
                pixels: self.placeholder.clone(),
            };
            let displayed = e.displayed.as_ref().unwrap_or(&placeholder);
            for (i, &id) in e.ids.iter().enumerate() {
                if !active.contains(&id) {
                    continue;
                }
                let pixels = if i == 1 {
                    match &e.document {
                        Some(Document::Raster(r)) => r,
                        _ => displayed,
                    }
                } else {
                    displayed
                };
                images.push(TextureImage {
                    id,
                    size: pixels.size,
                    pixels: Arc::clone(&pixels.pixels),
                    revision: e.revisions[i],
                });
                options.insert(
                    id,
                    TextureOptions {
                        filter: if i == 1 {
                            TextureFilter::Nearest
                        } else {
                            TextureFilter::Linear
                        },
                        managed: true,
                    },
                );
                if e.displayed.is_some() && !e.observed_ready {
                    ready.push(e.requested_at.elapsed());
                    e.observed_ready = true;
                }
            }
        }
        for duration in ready {
            self.record(ImageTiming {
                stage: ImageStage::FirstReady,
                duration,
            });
        }
        (images, options)
    }
}
