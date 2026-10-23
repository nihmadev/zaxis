//! Releasing image textures: least-recently-used eviction within the budget, and clearing.
use super::*;
use std::collections::HashSet;

impl Gpu {
    pub(super) fn evict_images(
        &mut self,
        store: &mut TextureStore,
        budget: u64,
        active: &HashSet<TextureId>,
    ) {
        loop {
            let is_idle = |t: &GpuTexture| {
                t.managed
                    && !store
                        .textures
                        .iter()
                        .any(|(id, o)| active.contains(id) && o.allocation == t.allocation)
            };
            let mut seen = HashSet::new();
            let idle_bytes: u64 = store
                .textures
                .values()
                .filter(|t| is_idle(t) && seen.insert(t.allocation))
                .map(|t| u64::from(t.size[0]) * u64::from(t.size[1]) * 4)
                .sum();
            if idle_bytes <= budget {
                break;
            }
            let allocation = store
                .textures
                .iter()
                .filter(|(_, t)| is_idle(t))
                .min_by_key(|(id, t)| (t.last_used, id.0))
                .map(|(_, t)| t.allocation);
            let Some(allocation) = allocation else {
                break;
            };
            store
                .textures
                .retain(|_, t| !t.managed || t.allocation != allocation);
            self.stats.texture_evictions += 1;
        }
    }

    /// Drop every managed image texture, including those used by sibling renderers.
    pub(in crate::renderer) fn clear_image_textures(&mut self) {
        let mut allocations = HashSet::new();
        self.store
            .lock()
            .expect("texture store mutex")
            .textures
            .retain(|_, t| {
                if t.managed {
                    allocations.insert(t.allocation);
                    false
                } else {
                    true
                }
            });
        self.stats.texture_evictions += allocations.len() as u64;
        self.stats.image_resident_bytes_estimate = 0;
    }
}

/// Estimated bytes of the managed textures, each allocation once.
pub(super) fn image_bytes(store: &TextureStore) -> u64 {
    let mut seen = HashSet::new();
    store
        .textures
        .values()
        .filter(|t| t.managed && seen.insert(t.allocation))
        .map(|t| u64::from(t.size[0]) * u64::from(t.size[1]) * 4)
        .sum()
}
