//! Raw pixel images that change in place.
//!
//! The host hands over rectangles of new pixels ([`Patch`]); the image keeps the complete frame
//! as its source of truth. Patches queue until the next frame is published, then land in a
//! second buffer: the frame before the previous one, caught up by the few regions that
//! differ, so the buffer the last frame's draw data still shares is never written and a
//! frame is never copied whole. A change of revision names the regions, and the renderer
//! writes only those into its texture.
use super::*;
use crate::TextureRegion;

/// Patches queued before the image is brought up to date without waiting for a frame.
pub(super) const MAX_PATCHES: usize = 64;
/// Regions of one change reported to the renderer; more become one bounding box.
const MAX_REGIONS: usize = 8;

/// New pixels for a rectangle: its rows start at `offset` in `data`, `stride` bytes apart.
pub(super) struct Patch {
    region: TextureRegion,
    data: Arc<Vec<u8>>,
    offset: usize,
    stride: usize,
}

/// The change between two revisions of the two textures of an image.
struct Change {
    bases: [u64; 2],
    tos: [u64; 2],
    regions: Vec<TextureRegion>,
}

#[derive(Default)]
pub(super) struct Live {
    /// An older frame, reusable once nothing else holds it.
    spare: Option<Arc<Vec<u8>>>,
    /// Where `spare` differs from the current frame.
    lag: Vec<TextureRegion>,
    pending: Vec<Patch>,
    pending_bytes: usize,
    update: Option<Change>,
}

impl Patch {
    /// The patch is a complete frame, tightly packed.
    fn covers(&self, size: [u32; 2]) -> bool {
        self.region == TextureRegion::whole(size)
            && self.offset == 0
            && self.stride == size[0] as usize * PixelFormat::BYTES_PER_PIXEL
            && self.data.len() == size[0] as usize * size[1] as usize * PixelFormat::BYTES_PER_PIXEL
    }
}

impl Live {
    pub fn held_bytes(&self) -> usize {
        self.spare.as_ref().map_or(0, |s| s.len()) + self.pending_bytes
    }

    /// Forget the change and the spare frame: the next revision is not a patch of this one.
    pub fn reset(&mut self) {
        self.spare = None;
        self.lag.clear();
        self.update = None;
    }

    pub fn release_spare(&mut self) {
        self.spare = None;
        self.lag.clear();
    }

    /// The change that brought texture `index` (0 linear, 1 nearest) from its previous
    /// revision to `revision`, when that is the latest one.
    pub fn update(&self, index: usize, revision: u64) -> Option<(u64, &[TextureRegion])> {
        let change = self.update.as_ref()?;
        (change.tos[index] == revision).then(|| (change.bases[index], &change.regions[..]))
    }
}

impl Entry {
    /// Make the image show its own pixels at full size, without a worker: the document is
    /// the host's buffer. A job in flight is cancelled, its result belongs to an older frame.
    fn go_live(&mut self, limits: &ImageLimits, generation: u64) -> Result<(), ImageError> {
        if self.document.is_none() {
            let mut timings = Vec::new();
            let document = decode::load(&self.source, limits, &[], &mut timings)?;
            self.intrinsic = Some(document.size());
            if let Document::Raster(r) = &document {
                self.displayed = Some(r.clone());
            }
            self.document = Some(document);
            self.revisions[0] += 1;
            self.revisions[1] += 1;
            self.generation = generation;
            self.pending = false;
        }
        self.native = true;
        if let Source::Rgba(size, data) = &self.source {
            self.source = Source::Pixels(Pixels {
                format: PixelFormat::Rgba8Srgb,
                size: *size,
                data: Arc::clone(data),
                live: true,
            });
        }
        if let Some(Document::Raster(r)) = &self.document {
            let same = self
                .displayed
                .as_ref()
                .is_some_and(|d| Arc::ptr_eq(&d.pixels, &r.pixels));
            if !same {
                self.displayed = Some(r.clone());
                self.variants.clear();
                self.revisions[0] += 1;
                self.generation = generation;
                self.pending = false;
            }
        }
        Ok(())
    }
}

impl Entry {
    /// Raw pixels shown at their own size need no worker: their document is the host's
    /// buffer. Open it now, so that the first frame already draws the image. A source that
    /// breaks a limit is an error of the image.
    pub(super) fn open_native(&mut self, limits: &ImageLimits, generation: u64) {
        if self.native && self.document.is_none() && self.source.pixels().is_some() {
            if let Err(error) = self.go_live(limits, generation) {
                self.error = Some(error);
            }
        }
    }
}

impl ImageCache {
    /// Whether `len` bytes of pixels hold `region` of the image: its rows start at `offset`,
    /// `stride` bytes apart. The error says which part is wrong.
    pub fn check_region(
        &self,
        h: ImageHandle,
        region: TextureRegion,
        len: usize,
        offset: usize,
        stride: usize,
    ) -> Result<(), ImageError> {
        if h.owner != self.owner {
            return Err(ImageError("foreign image handle".into()));
        }
        let e = self
            .entries
            .get(&h.id)
            .ok_or_else(|| ImageError("released image handle".into()))?;
        let Some((_, size, _, _)) = e.source.pixels() else {
            return Err(ImageError(
                "region updates need a raw pixel image (ImageSource::rgba, bgra, ...)".into(),
            ));
        };
        if let Some(error) = &e.error {
            return Err(error.clone());
        }
        if !region.fits(size) {
            return Err(ImageError(format!(
                "region {}x{} at ({}, {}) is empty or outside the {}x{} image",
                region.width, region.height, region.x, region.y, size[0], size[1]
            )));
        }
        let row = region.width as usize * PixelFormat::BYTES_PER_PIXEL;
        if stride < row {
            return Err(ImageError(format!(
                "stride {stride} is shorter than a row of the region ({row} bytes)"
            )));
        }
        let needed = (region.height as usize - 1)
            .checked_mul(stride)
            .and_then(|n| n.checked_add(row));
        match needed.and_then(|n| n.checked_add(offset)) {
            Some(end) if end <= len => Ok(()),
            _ => Err(ImageError(format!(
                "pixels hold {len} bytes from offset {offset}, the region needs {} with stride {stride}",
                needed.map_or("more than the address space".into(), |n| n.to_string()),
            ))),
        }
    }

    /// Queue new pixels for `region` of the image. The rows of the region are in `data`
    /// from `offset`, `stride` bytes apart. Applied in order, the last wins.
    pub fn update_region(
        &mut self,
        h: ImageHandle,
        region: TextureRegion,
        data: Arc<Vec<u8>>,
        offset: usize,
        stride: usize,
    ) -> Result<(), ImageError> {
        self.check_region(h, region, data.len(), offset, stride)?;
        let generation = self.generation();
        let limits = &self.limits;
        let e = self.entries.get_mut(&h.id).expect("checked entry");
        e.go_live(limits, generation)?;
        let bytes = region.width as usize * PixelFormat::BYTES_PER_PIXEL * region.height as usize;
        let live = e.live.get_or_insert_with(Default::default);
        live.pending_bytes += bytes;
        live.pending.push(Patch {
            region,
            data,
            offset,
            stride,
        });
        self.metrics.region_updates += 1;
        self.metrics.region_bytes += bytes as u64;
        if live.pending.len() >= MAX_PATCHES {
            flush(e, &mut self.metrics);
        }
        self.epoch += 1;
        Ok(())
    }

    /// Bring every image with queued patches up to date: the next revision is published.
    pub fn flush_live(&mut self, ids: &[u64]) {
        for id in ids {
            if let Some(e) = self.entries.get_mut(id) {
                flush(e, &mut self.metrics);
            }
        }
    }

    /// Entries with patches waiting.
    pub fn with_patches(&self) -> Vec<u64> {
        self.entries
            .iter()
            .filter(|(_, e)| e.live.as_ref().is_some_and(|l| !l.pending.is_empty()))
            .map(|(&id, _)| id)
            .collect()
    }
}

/// Apply the queued patches and publish the next revision.
pub(super) fn flush(e: &mut Entry, metrics: &mut ImageMetrics) {
    let Some(mut live) = e.live.take() else {
        return;
    };
    let (format, size) = match e.source.pixels() {
        Some((format, size, _, _)) if !live.pending.is_empty() => (format, size),
        _ => {
            e.live = Some(live);
            return;
        }
    };
    // Let go of every reference the entry itself holds, so that a count of one means nobody
    // outside does.
    let held = std::mem::replace(
        &mut e.source,
        Source::Pixels(Pixels {
            format,
            size,
            data: Arc::default(),
            live: true,
        }),
    );
    e.document = None;
    e.displayed = None;
    e.variants.clear();
    let (Source::Rgba(_, front) | Source::Pixels(Pixels { data: front, .. })) = held else {
        unreachable!("a live image keeps raw pixels")
    };
    let patches = std::mem::take(&mut live.pending);
    live.pending_bytes = 0;
    let regions: Vec<_> = patches.iter().map(|p| p.region).collect();
    let whole = patches.iter().rposition(|p| p.covers(size));
    let data: Arc<Vec<u8>> = if let Some(at) = whole {
        // A patch that is the whole frame replaces it: its buffer is the next frame, with the
        // patches after it applied, and nothing is copied when there are none.
        let tail = &patches[at + 1..];
        let data = Arc::clone(&patches[at].data);
        live.spare = Some(front);
        live.lag = vec![TextureRegion::whole(size)];
        if tail.is_empty() {
            data
        } else {
            let mut buffer = Arc::try_unwrap(data).unwrap_or_else(|shared| {
                metrics.live_full_copies += 1;
                shared.as_ref().clone()
            });
            apply(&mut buffer, size, tail);
            Arc::new(buffer)
        }
    } else {
        let current = match Arc::try_unwrap(front) {
            Ok(mut buffer) => {
                apply(&mut buffer, size, &patches);
                live.lag.extend(&regions);
                live.lag = TextureRegion::coalesce(&live.lag, size, MAX_REGIONS);
                buffer
            }
            Err(front) => {
                let mut buffer = match live.spare.take().map(Arc::try_unwrap) {
                    Some(Ok(mut spare)) => {
                        let lag = std::mem::take(&mut live.lag);
                        let patch_from_front: Vec<_> = lag
                            .iter()
                            .map(|&region| Patch {
                                region,
                                data: Arc::clone(&front),
                                offset: region.y as usize * size[0] as usize * 4
                                    + region.x as usize * 4,
                                stride: size[0] as usize * 4,
                            })
                            .collect();
                        apply(&mut spare, size, &patch_from_front);
                        spare
                    }
                    _ => {
                        metrics.live_full_copies += 1;
                        front.as_ref().clone()
                    }
                };
                apply(&mut buffer, size, &patches);
                live.lag = TextureRegion::coalesce(&regions, size, MAX_REGIONS);
                live.spare = Some(front);
                buffer
            }
        };
        Arc::new(current)
    };
    let bases = e.revisions;
    e.revisions[0] += 1;
    e.revisions[1] += 1;
    live.update = Some(Change {
        bases,
        tos: e.revisions,
        regions: TextureRegion::coalesce(&regions, size, MAX_REGIONS),
    });
    let image = DecodedImage {
        size,
        pixels: Arc::clone(&data),
    };
    e.source = Source::Pixels(Pixels {
        format,
        size,
        data,
        live: true,
    });
    e.document = Some(Document::Raster(image.clone()));
    e.displayed = Some(image);
    e.live = Some(live);
    metrics.live_flushes += 1;
}

/// Copy the rows of each patch into the frame, in order.
fn apply(frame: &mut [u8], size: [u32; 2], patches: &[Patch]) {
    let stride = size[0] as usize * PixelFormat::BYTES_PER_PIXEL;
    for patch in patches {
        let row = patch.region.width as usize * PixelFormat::BYTES_PER_PIXEL;
        for y in 0..patch.region.height as usize {
            let from = patch.offset + y * patch.stride;
            let to = (patch.region.y as usize + y) * stride
                + patch.region.x as usize * PixelFormat::BYTES_PER_PIXEL;
            frame[to..to + row].copy_from_slice(&patch.data[from..from + row]);
        }
    }
}
