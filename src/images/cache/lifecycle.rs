use super::*;

impl ImageCache {
    /// Start a pass of any window. The frame counter is the cache's own and advances once per
    /// pass of any context sharing it, so "used this frame" means "by the running window".
    pub fn begin_frame(&mut self, now: Instant) {
        self.frame += 1;
        self.now = now;
        for e in self.entries.values_mut() {
            e.wanted = [0; 2];
            e.linear_wanted = [0; 2];
        }
        // Results are atomically published only at the start of a UI pass.
        for result in self.workers.drain() {
            let start = Instant::now();
            self.epoch += 1;
            self.metrics.pending_jobs -= 1;
            self.inflight_bytes = self.inflight_bytes.saturating_sub(result.reserved);
            let counted = |stage| result.timings.iter().any(|t| t.stage == stage);
            self.metrics.decodes += u64::from(counted(ImageStage::Decode));
            self.metrics.svg_parses += u64::from(counted(ImageStage::SvgParse));
            self.metrics.rasterizations += u64::from(counted(ImageStage::SvgRaster));
            self.metrics.downsamplings += u64::from(counted(ImageStage::Downsample));
            for t in result.timings {
                self.record(t);
            }
            let Some(e) = self
                .entries
                .get_mut(&result.id)
                .filter(|e| e.generation == result.generation)
            else {
                self.metrics.stale_completions += 1;
                continue;
            };
            e.pending = false;
            match result.output {
                Ok((document, pixels)) => {
                    if result.loaded {
                        e.intrinsic = Some(document.size());
                        e.document = Some(document);
                        e.revisions[1] += 1;
                    }
                    if matches!(e.document, Some(Document::Svg { .. })) && !result.loaded {
                        e.revisions[1] += 1;
                    }
                    e.displayed = Some(pixels.clone());
                    e.revisions[0] += 1;
                    e.variants.push_back(pixels);
                    while e.variants.len() > self.limits.raster_variants.max(1) {
                        e.variants.pop_front();
                    }
                }
                Err(error) => e.error = Some(error),
            }
            self.record(ImageTiming {
                stage: ImageStage::Publication,
                duration: start.elapsed(),
            });
        }
        self.evict_to_fit(0);
    }
    /// Enqueue only visible demand. Returns the next resize-settle deadline.
    pub fn finish_frame(&mut self) -> Option<Instant> {
        let mut deadline: Option<Instant> = None;
        let mut candidates: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| e.wanted != [0; 2] && e.error.is_none())
            .map(|(&id, _)| id)
            .collect();
        candidates.sort_unstable();
        for id in candidates {
            let e = self.entries.get_mut(&id).unwrap();
            let mut target = e.wanted;
            if let Some(Document::Raster(r)) = &e.document {
                target = if e.linear_wanted == [0; 2] {
                    r.size
                } else {
                    e.linear_wanted
                };
                // A single largest visible variant is shared across linear instances.
                let ratio =
                    (target[0] as f32 / r.size[0] as f32).max(target[1] as f32 / r.size[1] as f32);
                target = if ratio >= 0.5 {
                    r.size
                } else {
                    [
                        (r.size[0] as f32 * ratio).ceil().max(1.0) as u32,
                        (r.size[1] as f32 * ratio).ceil().max(1.0) as u32,
                    ]
                };
            }
            if target != e.target {
                e.target = target;
                e.changed_at = self.now;
            }
            if e.pending || e.displayed.as_ref().is_some_and(|r| r.size == target) {
                continue;
            }
            let validation = if e.document.is_some() {
                Some(target)
            } else if let Source::Rgba(size, _) = &e.source {
                Some(*size)
            } else {
                None
            };
            if let Some(Err(error)) = validation.map(|size| self.limits.check_size(size)) {
                e.error = Some(error);
                self.state_changed = true;
                continue;
            }
            if let Some(n) = e.variants.iter().position(|r| r.size == target) {
                let r = e.variants.remove(n).unwrap();
                e.displayed = Some(r.clone());
                e.variants.push_back(r);
                e.revisions[0] += 1;
                if matches!(e.document, Some(Document::Svg { .. })) {
                    e.revisions[1] += 1;
                }
                continue;
            }
            let ready = e.displayed.is_some();
            let at = e.changed_at + self.limits.resize_debounce;
            if ready && at > self.now {
                deadline = Some(deadline.map_or(at, |d| d.min(at)));
                continue;
            }
            let reserve = self
                .limits
                .max_decoded_bytes
                .saturating_mul(2)
                .saturating_add(self.limits.max_encoded_bytes);
            if self.metrics.pending_jobs >= self.limits.max_pending_jobs
                || self.inflight_bytes.saturating_add(reserve) > self.limits.max_inflight_bytes
            {
                self.metrics.backpressure += 1;
                // Completion of an in-flight job wakes us, no polling deadline needed.
                if self.metrics.pending_jobs == 0 {
                    self.state_changed = true;
                    e.error = Some(ImageError(
                        "in-flight image budget cannot hold one job".into(),
                    ));
                }
                continue;
            }
            let work = match &e.document {
                Some(doc) => Work::Variant(doc.clone()),
                None => Work::Load(e.source.clone(), self.decoders.clone()),
            };
            let job = Job {
                id,
                generation: e.generation,
                size: target,
                linear: e.linear_wanted != [0; 2],
                raster_size: if e.linear_wanted == [0; 2] {
                    target
                } else {
                    e.linear_wanted
                },
                limits: self.limits.clone(),
                work,
                queued_at: Instant::now(),
                reserved: reserve,
            };
            match self.workers.submit(job) {
                Ok(()) => {
                    e.pending = true;
                    self.metrics.pending_jobs += 1;
                    self.inflight_bytes += reserve;
                }
                Err(error) => {
                    e.error = Some(error);
                    self.state_changed = true;
                }
            }
        }
        self.evict_to_fit(0);
        deadline
    }
    pub fn pin(&mut self, h: ImageHandle) {
        if let Some(e) = self.entries.get_mut(&h.id) {
            e.pinned = true;
        }
    }
    pub fn reload(
        &mut self,
        h: ImageHandle,
        source: Option<ImageSource>,
    ) -> Result<(), ImageError> {
        if h.owner != self.owner {
            return Err(ImageError("foreign image handle".into()));
        }
        let source = source.map(|s| match self.key(&s.0) {
            Key::Path(p) => ImageSource(Source::Path(Arc::new(p)), s.1),
            _ => s,
        });
        if let Some(s) = &source {
            let old = self
                .entries
                .get(&h.id)
                .ok_or_else(|| ImageError("released image handle".into()))?
                .bytes();
            let input = match &s.0 {
                Source::Encoded(p) | Source::Rgba(_, p) => p.len(),
                _ => 0,
            };
            self.evict_to_fit(input.saturating_sub(old));
            if self
                .resident_bytes()
                .saturating_sub(old)
                .saturating_add(input)
                > self.limits.cpu_cache_bytes
            {
                return Err(ImageError(
                    "replacement source exceeds CPU cache budget".into(),
                ));
            }
            self.keys.retain(|_, id| *id != h.id);
        }
        self.epoch += 1;
        let generation = self.generation();
        let e = self
            .entries
            .get_mut(&h.id)
            .ok_or_else(|| ImageError("released image handle".into()))?;
        if let Some(source) = source {
            if matches!(source.0, Source::Handle(_)) {
                return Err(ImageError("replacement must contain source data".into()));
            }
            let input = match &source.0 {
                Source::Encoded(p) | Source::Rgba(_, p) => p.len(),
                _ => 0,
            };
            if input > self.limits.cpu_cache_bytes {
                return Err(ImageError("replacement source exceeds CPU budget".into()));
            }
            e.source = source.0;
            if let Source::Rgba(size, _) = &e.source {
                e.intrinsic = Some(*size);
            }
        }
        e.generation = generation;
        e.pending = false;
        e.document = None;
        e.displayed = None;
        e.variants.clear();
        e.error = None;
        e.revisions[0] += 1;
        e.revisions[1] += 1;
        e.target = [0; 2];
        e.observed_ready = false;
        e.requested_at = Instant::now();
        Ok(())
    }
    pub fn invalidate(&mut self, source: ImageSource) -> Result<(), ImageError> {
        let h = self.resolve(source)?;
        self.reload(h, None)
    }
    pub fn release(&mut self, h: ImageHandle) {
        if h.owner != self.owner {
            return;
        }
        self.epoch += 1;
        self.entries.remove(&h.id);
        self.keys.retain(|_, id| *id != h.id);
    }
    pub fn clear_decoded(&mut self) {
        let ids: Vec<_> = self.entries.keys().copied().collect();
        for id in ids {
            let _ = self.reload(
                ImageHandle {
                    owner: self.owner,
                    id,
                },
                None,
            );
        }
        self.metrics.evictions += self.entries.len() as u64;
    }
    pub(super) fn evict_to_fit(&mut self, additional: usize) {
        while self.resident_bytes().saturating_add(additional) > self.limits.cpu_cache_bytes
            || self.entries.len() >= self.limits.max_entries
        {
            let candidate = self
                .entries
                .iter()
                .filter(|(_, e)| {
                    !e.pending && e.last_frame != self.frame && (!e.pinned || e.document.is_some())
                })
                .min_by_key(|(id, e)| (e.last_frame, **id))
                .map(|(&id, _)| id);
            let Some(id) = candidate else {
                break;
            };
            if self.entries[&id].pinned {
                let _ = self.reload(
                    ImageHandle {
                        owner: self.owner,
                        id,
                    },
                    None,
                );
            } else {
                self.entries.remove(&id);
                self.keys.retain(|_, v| *v != id);
            }
            self.metrics.evictions += 1;
        }
        // Active set cannot silently exceed the budget; retain source and an error.
        if self.resident_bytes() > self.limits.cpu_cache_bytes {
            let ids: Vec<_> = self
                .entries
                .iter()
                .filter(|(_, e)| e.document.is_some())
                .map(|(&id, _)| id)
                .collect();
            for id in ids {
                if self.resident_bytes() <= self.limits.cpu_cache_bytes {
                    break;
                }
                let e = self.entries.get_mut(&id).unwrap();
                e.document = None;
                e.displayed = None;
                e.variants.clear();
                e.error = Some(ImageError("visible images exceed CPU cache budget".into()));
                self.state_changed = true;
                self.metrics.evictions += 1;
            }
        }
    }
}

impl ImageCache {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Whether a failure or limit change was recorded since the last call; counts as a change.
    pub fn take_state_changed(&mut self) -> bool {
        let changed = std::mem::take(&mut self.state_changed);
        self.epoch += u64::from(changed);
        changed
    }
}
