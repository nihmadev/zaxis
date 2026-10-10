//! Pipelines and layouts that every renderer of a device shares: the built-in pipelines per
//! [`Target`] and the backdrop filter per canvas format.

use super::{
    blur::{BlurDevice, BlurPipelines},
    pipeline::{PipelineSet, Target},
};
use std::{collections::HashMap, sync::Arc};

/// Target kinds kept at once. A target whose pipelines a renderer still holds is never
/// evicted, so the cache can exceed this only while that many are in use.
const MAX_TARGETS: usize = 16;

/// The built-in pipelines by [`Target`], created on first use.
pub(super) struct PipelineCache {
    device: wgpu::Device,
    viewport_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    entries: Vec<(Arc<PipelineSet>, u64)>,
    clock: u64,
}

impl PipelineCache {
    pub fn new(
        device: &wgpu::Device,
        viewport_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        Self {
            device: device.clone(),
            viewport_layout: viewport_layout.clone(),
            texture_layout: texture_layout.clone(),
            entries: Vec::new(),
            clock: 0,
        }
    }

    /// The pipelines for `target`, built now if no renderer asked for them before.
    pub fn get(&mut self, target: Target) -> Arc<PipelineSet> {
        self.clock += 1;
        if let Some((set, used)) = self.entries.iter_mut().find(|(s, _)| s.target == target) {
            *used = self.clock;
            return Arc::clone(set);
        }
        while self.entries.len() >= MAX_TARGETS {
            let idle = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, (set, _))| Arc::strong_count(set) == 1)
                .min_by_key(|(_, (_, used))| *used)
                .map(|(index, _)| index);
            match idle {
                Some(index) => self.entries.swap_remove(index),
                None => break,
            };
        }
        let set = Arc::new(PipelineSet::new(
            &self.device,
            &self.viewport_layout,
            &self.texture_layout,
            target,
        ));
        self.entries.push((Arc::clone(&set), self.clock));
        set
    }

    /// Targets cached now.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// The backdrop filter's pipelines by canvas and pyramid format.
#[derive(Default)]
pub(super) struct BlurCache {
    entries: HashMap<(wgpu::TextureFormat, wgpu::TextureFormat), Arc<BlurPipelines>>,
}

impl BlurCache {
    pub fn get(&mut self, dev: &BlurDevice) -> Arc<BlurPipelines> {
        Arc::clone(
            self.entries
                .entry((dev.format, dev.intermediate))
                .or_insert_with(|| Arc::new(BlurPipelines::new(dev))),
        )
    }
}
