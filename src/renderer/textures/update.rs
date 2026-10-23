//! Bringing a texture to a new revision by writing only the regions that changed.
use super::{upload::write_region, *};
use crate::{TextureImage, TextureRegion, TextureUpdate};
use std::collections::HashSet;

/// Regions of one update a backend accepts; more than this is uploaded whole.
pub(super) const MAX_REGIONS: usize = 256;

/// Writes per texture and update; beyond it the regions become one bounding box.
const MAX_WRITES: usize = 8;

/// What a frame's updates and active IDs say about a texture's neighbours.
pub(super) struct Frame<'a> {
    pub updates: &'a HashMap<TextureId, &'a TextureUpdate>,
    pub active: &'a HashSet<TextureId>,
}

/// Updates of `data` that name a payload at their revision and stay inside it. Others are
/// ignored: the payload is complete, so the texture is uploaded whole instead.
pub(super) fn usable<'a>(
    images: &'a [TextureImage],
    updates: &'a [TextureUpdate],
) -> HashMap<TextureId, &'a TextureUpdate> {
    let mut usable = HashMap::new();
    for update in updates {
        let Some(image) = images.iter().find(|i| i.id == update.id) else {
            continue;
        };
        if update.base_revision < update.to_revision
            && update.to_revision == image.revision
            && !update.regions.is_empty()
            && update.regions.len() <= MAX_REGIONS
            && update.regions.iter().all(|r| r.fits(image.size))
        {
            usable.insert(update.id, update);
        }
    }
    usable
}

impl Gpu {
    /// Write the regions of `update` into the texture of `image`, which must hold exactly
    /// the base revision in the same layout and size, and keep its bind group. False leaves
    /// everything as it was.
    ///
    /// An allocation shared with other IDs is written once for all of them when they are
    /// updated by the same change. A sharer that is idle and managed is dropped, since its
    /// next appearance uploads it anyway; any other sharer would show the new pixels
    /// without asking for them, so the texture is uploaded whole into a fresh allocation.
    pub(super) fn apply_update(
        &mut self,
        store: &mut TextureStore,
        image: &TextureImage,
        texels: Texels,
        update: &TextureUpdate,
        frame: &Frame<'_>,
    ) -> bool {
        let Some(t) = store.textures.get(&image.id) else {
            return false;
        };
        if t.size != image.size || t.texels != texels || t.revision != update.base_revision {
            return false;
        }
        let mut followers = Vec::new();
        let mut dropped = Vec::new();
        if t.allocation != 0 {
            for (id, other) in &store.textures {
                if *id == image.id || other.allocation != t.allocation {
                    continue;
                }
                let same_change = frame.updates.get(id).is_some_and(|u| {
                    u.base_revision == update.base_revision
                        && u.to_revision == update.to_revision
                        && u.regions == update.regions
                });
                if same_change && other.revision == update.base_revision {
                    followers.push(*id);
                } else if !frame.active.contains(id) && other.managed {
                    dropped.push(*id);
                } else {
                    return false;
                }
            }
        }
        let start = self.diagnostics.start();
        for region in TextureRegion::coalesce(&update.regions, image.size, MAX_WRITES) {
            write_region(
                &self.queue,
                &t.texture,
                image.size,
                &image.pixels,
                region,
                texels.convert,
            );
            self.stats.texture_writes += 1;
            self.stats.texture_region_writes += 1;
            self.stats.texture_region_bytes += region.area() * 4;
        }
        self.diagnostics.end(RendererStage::WriteTextureCpu, start);
        self.stats.texture_region_updates += 1;
        for id in dropped {
            store.textures.remove(&id);
        }
        for id in followers.into_iter().chain([image.id]) {
            if let Some(t) = store.textures.get_mut(&id) {
                t.revision = update.to_revision;
                t.pixels = Arc::downgrade(&image.pixels);
            }
        }
        true
    }
}
