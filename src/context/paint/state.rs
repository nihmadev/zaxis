//! What painting keeps: this pass's elements and order, and meshes cached across passes.

use super::{visual::VisualMesh, Paint};
use crate::{
    context::{geometry::Element, Id},
    shapes::Mesh,
    Rect,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub struct CachedElement {
    pub paint: Vec<Paint>,
    pub scale: f32,
    pub mesh: Arc<Mesh>,
    pub bounds: Option<Rect>,
    pub last_frame: u64,
}

/// Paint data of one context. The cache and visual meshes outlive passes; elements,
/// order, the ids painted and the ids whose mesh changed belong to the current pass.
pub(crate) struct PaintState {
    pub(in crate::context) cache: HashMap<Id, CachedElement>,
    pub(in crate::context) visual_meshes: HashMap<Id, VisualMesh>,
    /// The inner paint of a visual is being drawn: its element is about to be wrapped,
    /// so the visual mesh it had is not stale.
    pub(super) materializing: bool,
    /// Scale of the visuals around the element being drawn, for image requests.
    pub(super) image_visual_scale: f32,
    pub(in crate::context) elements: Vec<Element>,
    pub(in crate::context) order: HashMap<Id, usize>,
    pub(in crate::context) seen: HashSet<Id>,
    pub(in crate::context) modified: HashSet<Id>,
}

impl Default for PaintState {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            visual_meshes: HashMap::new(),
            materializing: false,
            image_visual_scale: 1.0,
            elements: Vec::new(),
            order: HashMap::new(),
            seen: HashSet::new(),
            modified: HashSet::new(),
        }
    }
}

impl PaintState {
    pub(in crate::context) fn begin_pass(&mut self) {
        self.elements.clear();
        self.order.clear();
        self.seen.clear();
        self.modified.clear();
    }

    /// Visual meshes of elements not painted in this pass and cache entries not used by
    /// it are dropped. The two rules differ: a culled element keeps its cache entry alive
    /// without being painted into an element.
    pub(in crate::context) fn retire(&mut self, frame: u64) {
        let seen = &self.seen;
        self.visual_meshes.retain(|id, _| seen.contains(id));
        self.cache.retain(|_, element| element.last_frame == frame);
    }

    /// Order of `id` among this pass's paints: its first record.
    pub(in crate::context) fn record_order(&mut self, id: Id) {
        let order = self.order.len();
        self.order.entry(id).or_insert(order);
    }

    /// Scale of the visuals around the element being painted.
    pub(in crate::context) fn image_visual_scale(&self) -> f32 {
        self.image_visual_scale
    }

    /// Whether `id` was painted in this pass.
    pub(in crate::context) fn painted(&self, id: Id) -> bool {
        self.seen.contains(&id)
    }

    /// Claim `id` for this pass. A repeated id gets a deterministic alias, recorded in
    /// paint order, so neither element disappears and the cache stays consistent.
    pub(super) fn claim(&mut self, id: Id) -> Result<Id, Id> {
        if self.seen.insert(id) {
            return Ok(id);
        }
        let mut n = 1_u32;
        let alias = loop {
            let alias = id.with(("duplicate", n));
            if self.seen.insert(alias) {
                break alias;
            }
            n += 1;
        };
        self.record_order(alias);
        Err(alias)
    }

    /// The element just painted under `id`, if the last paint produced one.
    pub(in crate::context) fn last_element_mut(&mut self, id: Id) -> Option<&mut Element> {
        self.elements.last_mut().filter(|element| element.id == id)
    }

    /// Order elements by layer rank, then by paint order within a layer.
    pub(in crate::context) fn sort(&mut self, ranks: &HashMap<Id, usize>) {
        let order = &self.order;
        self.elements.sort_by_key(|e| {
            (
                ranks.get(&e.layer).copied().unwrap_or(0),
                order.get(&e.id).copied().unwrap_or(0),
            )
        });
    }

    /// A plain element replaces the visual it was: its wrapped mesh is gone.
    pub(super) fn drop_visual(&mut self, id: Id) {
        if !self.materializing && self.visual_meshes.remove(&id).is_some() {
            self.modified.insert(id);
        }
    }

    /// Append the cached mesh of `id` to this pass's elements, unless `cull` and its
    /// bounds miss `clip`.
    pub(super) fn emit(&mut self, id: Id, layer: Id, clip: Rect, cull: bool) {
        let cached = &self.cache[&id];
        if cull
            && cached
                .bounds
                .is_some_and(|bounds| bounds.intersect(clip).is_empty())
        {
            return;
        }
        self.elements.push(Element {
            id,
            layer,
            clip,
            mesh: Arc::clone(&cached.mesh),
            blur: None,
            scroll_hint: super::data::is_scroll_hint(&cached.paint),
        });
    }
}
