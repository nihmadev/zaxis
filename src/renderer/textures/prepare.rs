//! Bringing the store up to date with a frame's payloads.
use super::{
    evict::image_bytes,
    update::{usable, Frame},
    upload::{create_texture_profiled, write_texture},
    *,
};
use crate::{DrawData, TextureImage};
use std::collections::HashSet;

use super::super::RenderError;

impl Gpu {
    pub(in crate::renderer) fn prepare_textures(
        &mut self,
        store: &mut TextureStore,
        data: &DrawData,
    ) -> Result<(), RenderError> {
        if !store.shared && store.source != Some(data.source) {
            store
                .textures
                .retain(|id, t| *id == TextureId::WHITE || t.external);
        }
        store.source = Some(data.source);
        let active: HashSet<_> = data.commands.iter().map(|c| c.texture).collect();
        check_externals(store, data, &active)?;
        let active_bytes = self.validate_payloads(data, &active)?;
        self.evict_images(
            store,
            data.texture_budget_bytes.saturating_sub(active_bytes),
            &active,
        );
        let updates = usable(&data.textures, &data.texture_updates);
        let frame = Frame {
            updates: &updates,
            active: &active,
        };
        for image in &data.textures {
            let options = data
                .texture_options
                .get(&image.id)
                .copied()
                .unwrap_or_default();
            if options.managed && !active.contains(&image.id) {
                continue;
            }
            let texels = self.texels(
                data.texture_formats
                    .get(&image.id)
                    .copied()
                    .unwrap_or_default(),
            );
            let lookup = self.diagnostics.start();
            let mut current = store.textures.get(&image.id).is_some_and(|t| {
                t.size == image.size && t.revision == image.revision && t.texels == texels
            });
            self.diagnostics.end(RendererStage::TextureLookup, lookup);
            if !current {
                if let Some(update) = updates.get(&image.id) {
                    current = self.apply_update(store, image, texels, update, &frame);
                    if !current {
                        self.stats.texture_update_fallbacks += 1;
                    }
                }
            }
            if current {
                let t = store.textures.get_mut(&image.id).unwrap();
                if t.filter != options.filter {
                    let bind_group = self.bind(&t.texture.clone(), options.filter);
                    let t = store.textures.get_mut(&image.id).unwrap();
                    t.bind_group = bind_group;
                    t.filter = options.filter;
                }
                let t = store.textures.get_mut(&image.id).unwrap();
                t.managed = options.managed;
                t.last_used = store.clock;
                continue;
            }
            let mut gpu = self.upload(store, image, texels, options.filter);
            gpu.pixels = Arc::downgrade(&image.pixels);
            gpu.filter = options.filter;
            gpu.managed = options.managed;
            gpu.last_used = store.clock;
            store.textures.insert(image.id, gpu);
        }
        while store.textures.values().filter(|t| t.managed).count() > data.texture_binding_budget {
            let id = store
                .textures
                .iter()
                .filter(|(id, t)| t.managed && !active.contains(id))
                .min_by_key(|(id, t)| (t.last_used, id.0))
                .map(|(&id, _)| id);
            let Some(id) = id else {
                break;
            };
            store.textures.remove(&id);
            self.stats.texture_evictions += 1;
        }
        self.stats.image_resident_bytes_estimate = image_bytes(store);
        self.stats.external_textures =
            store.textures.values().filter(|t| t.external).count() as u64;
        Ok(())
    }

    /// Check every payload and the budgets of the active ones; the bytes the active managed
    /// textures occupy.
    fn validate_payloads(
        &self,
        data: &DrawData,
        active: &HashSet<TextureId>,
    ) -> Result<u64, RenderError> {
        let limit = self.device.limits().max_texture_dimension_2d;
        let mut payloads = HashSet::new();
        let mut allocations = HashSet::new();
        let mut active_bytes = 0u64;
        for image in &data.textures {
            if image.id == TextureId::WHITE
                || !payloads.insert(image.id)
                || image.size.contains(&0)
                || image.size.iter().any(|s| *s > limit)
                || image.pixels.len() as u64
                    != u64::from(image.size[0]) * u64::from(image.size[1]) * 4
            {
                return Err(RenderError::Texture(format!(
                    "invalid texture {:?}, dimensions {:?}, device limit {limit}",
                    image.id, image.size
                )));
            }
            if active.contains(&image.id)
                && data
                    .texture_options
                    .get(&image.id)
                    .is_some_and(|o| o.managed)
                && allocations.insert((Arc::as_ptr(&image.pixels), image.size, image.revision))
            {
                active_bytes += image.pixels.len() as u64;
            }
        }
        if active.iter().any(|id| {
            *id != TextureId::WHITE
                && !payloads.contains(id)
                && !data.external_textures.iter().any(|e| e.id == *id)
        }) {
            return Err(RenderError::InvalidDrawData(
                "complete texture payload required for every referenced ID",
            ));
        }
        if active_bytes > data.texture_budget_bytes {
            return Err(RenderError::Texture(format!(
                "visible image set needs {active_bytes} bytes, GPU budget is {}",
                data.texture_budget_bytes
            )));
        }
        if active
            .iter()
            .filter(|id| data.texture_options.get(id).is_some_and(|o| o.managed))
            .count()
            > data.texture_binding_budget
        {
            return Err(RenderError::Texture(
                "visible image bindings exceed binding budget".into(),
            ));
        }
        Ok(active_bytes)
    }

    /// The texture for the complete payload `image`: another ID's allocation of the same
    /// buffer, this ID's own texture written over when nothing else uses it, or a new one.
    fn upload(
        &mut self,
        store: &mut TextureStore,
        image: &TextureImage,
        texels: Texels,
        filter: TextureFilter,
    ) -> GpuTexture {
        let shared = store
            .textures
            .iter()
            .find(|(id, t)| {
                **id != image.id
                    && t.size == image.size
                    && t.revision == image.revision
                    && t.texels == texels
                    && same_buffer(&t.pixels, &image.pixels)
            })
            .map(|(_, t)| (t.texture.clone(), t.allocation));
        if let Some((texture, allocation)) = shared {
            return GpuTexture {
                bind_group: self.bind(&texture, filter),
                texture,
                size: image.size,
                texels,
                revision: image.revision,
                filter,
                managed: false,
                last_used: store.clock,
                allocation,
                pixels: Weak::new(),
                external: false,
            };
        }
        let reusable = store
            .textures
            .get(&image.id)
            .filter(|t| t.size == image.size && t.texels == texels)
            .is_some_and(|t| {
                store
                    .textures
                    .values()
                    .filter(|o| o.allocation == t.allocation)
                    .count()
                    == 1
            });
        let gpu = if reusable {
            let mut t = store.textures.remove(&image.id).unwrap();
            let start = self.diagnostics.start();
            write_texture(
                &self.queue,
                &t.texture,
                image.size,
                &image.pixels,
                texels.convert,
            );
            self.diagnostics.end(RendererStage::WriteTextureCpu, start);
            if t.filter != filter {
                t.bind_group = self.bind(&t.texture.clone(), filter);
            }
            t.revision = image.revision;
            self.stats.texture_reuploads += 1;
            t
        } else {
            self.stats.texture_creations += 1;
            self.stats.texture_bind_group_creations += 1;
            store.allocations += 1;
            let sampler = self.sampler_for(filter).clone();
            let mut t = create_texture_profiled(
                &self.device,
                &self.queue,
                &self.texture_layout,
                &sampler,
                image.size,
                &image.pixels,
                image.revision,
                texels,
                &mut self.diagnostics,
            );
            t.allocation = store.allocations;
            t
        };
        self.stats.texture_uploads += 1;
        self.stats.texture_writes += 1;
        self.stats.texture_upload_bytes += image.pixels.len() as u64;
        gpu
    }
}

/// Every external texture the frame draws is registered, with the size the frame was built
/// for; it brings no payload, so a missing one cannot be uploaded instead.
fn check_externals(
    store: &TextureStore,
    data: &DrawData,
    active: &HashSet<TextureId>,
) -> Result<(), RenderError> {
    for id in active {
        let listed = data.external_textures.iter().find(|e| e.id == *id);
        let registered = store.textures.get(id).filter(|t| t.external);
        match (listed, registered) {
            (Some(image), Some(t)) if t.size != image.size => {
                return Err(RenderError::Texture(format!(
                    "external texture {id:?} is {}x{} but the frame was built for {}x{}: register it again and show the new ExternalImage",
                    t.size[0], t.size[1], image.size[0], image.size[1]
                )))
            }
            (Some(_), None) => {
                return Err(RenderError::Texture(format!(
                    "external texture {id:?} is not registered with this renderer, or was unregistered"
                )))
            }
            (None, Some(_)) => {
                return Err(RenderError::InvalidDrawData(
                    "a registered external texture is drawn without an entry in external_textures",
                ))
            }
            _ => {}
        }
    }
    Ok(())
}
