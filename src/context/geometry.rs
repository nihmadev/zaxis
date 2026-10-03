//! Incremental frame meshes, ordered batching, and draw-data revisions.

use super::{Context, Id};
use crate::{shapes::Mesh, DrawCommand, Rect, TextureId, Vertex};
use std::{ops::Range, sync::Arc};

pub(super) struct Element {
    pub(super) id: Id,
    pub(super) layer: Id,
    pub(super) clip: Rect,
    pub(super) mesh: Arc<Mesh>,
    pub(super) blur: Option<f32>,
    pub(super) scroll_hint: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ElementKey {
    pub(super) id: Id,
    pub(super) layer: Id,
    pub(super) clip: Rect,
    pub(super) blur: Option<f32>,
    pub(super) scroll_hint: bool,
}

/// Stable buffer locations. Growing an element relocates only that element;
/// unused old ranges are reclaimed on structural changes or excessive slack.
pub(super) struct MeshSlot {
    vertices: Range<usize>,
    indices: Range<usize>,
    batches: Vec<(Range<u32>, TextureId)>,
}

const EMPTY_VERTEX: Vertex = Vertex {
    position: [0.0; 2],
    uv: [0.0; 2],
    color: [0.0; 4],
};

impl Context {
    pub(super) fn rebuild_geometry(&mut self) {
        let keys: Vec<_> = self
            .elements
            .iter()
            .map(|e| ElementKey {
                id: e.id,
                layer: e.layer,
                clip: e.clip,
                blur: e.blur,
                scroll_hint: e.scroll_hint,
            })
            .collect();
        let metadata_changed = keys != self.previous_elements;
        let viewport_changed = self.draw_data.logical_size != self.logical_size
            || self.draw_data.scale_factor != self.scale;
        if metadata_changed || !self.modified.is_empty() || viewport_changed {
            let same_order = keys.len() == self.previous_elements.len()
                && keys
                    .iter()
                    .zip(&self.previous_elements)
                    .all(|(a, b)| a.id == b.id);
            let live_vertices: usize = self.elements.iter().map(|e| e.mesh.vertices.len()).sum();
            let live_indices: usize = self.elements.iter().map(|e| e.mesh.indices.len()).sum();
            let compact = !same_order
                // A global invalidation (such as DPI) gains nothing from slots.
                // Repack once instead of relocating and sorting every element.
                || self.modified.len() == self.elements.len()
                || self.draw_data.vertices.len() > live_vertices.saturating_mul(2) + 1024
                || self.draw_data.indices.len() > live_indices.saturating_mul(2) + 1536;
            let mut update = self.draw_data.geometry_update.take().unwrap_or_default();
            update.from_revision = self.draw_data.revision;
            update.to_revision = self.draw_data.revision + 1;
            update.vertices.clear();
            update.indices.clear();
            let mut commands_changed = metadata_changed || compact;
            if compact {
                self.draw_data.vertices.clear();
                self.draw_data.indices.clear();
                self.mesh_slots.clear();
                self.stats.geometry_full_rebuilds += 1;
            } else {
                self.stats.geometry_partial_updates += 1;
            }
            for (n, element) in self.elements.iter().enumerate() {
                if !compact && !self.modified.contains(&element.id) {
                    continue;
                }
                let mesh = &element.mesh;
                if compact {
                    let vertex_start = self.draw_data.vertices.len();
                    let index_start = self.draw_data.indices.len();
                    self.draw_data.vertices.extend_from_slice(&mesh.vertices);
                    self.draw_data
                        .indices
                        .extend(mesh.indices.iter().map(|i| i + vertex_start as u32));
                    self.mesh_slots.push(MeshSlot {
                        vertices: vertex_start..self.draw_data.vertices.len(),
                        indices: index_start..self.draw_data.indices.len(),
                        batches: mesh.batches.clone(),
                    });
                } else {
                    let slot = &mut self.mesh_slots[n];
                    let grow_vertices = mesh.vertices.len() > slot.vertices.len();
                    let grow_indices = mesh.indices.len() > slot.indices.len();
                    if grow_vertices {
                        let start = self.draw_data.vertices.len();
                        let end = start + mesh.vertices.len().next_power_of_two();
                        self.draw_data.vertices.resize(end, EMPTY_VERTEX);
                        slot.vertices = start..end;
                        commands_changed = true;
                    }
                    if grow_indices {
                        let start = self.draw_data.indices.len();
                        let end = start + mesh.indices.len().next_power_of_two();
                        self.draw_data.indices.resize(end, 0);
                        slot.indices = start..end;
                        commands_changed = true;
                    }
                    self.draw_data.vertices
                        [slot.vertices.start..slot.vertices.start + mesh.vertices.len()]
                        .copy_from_slice(&mesh.vertices);
                    for (target, index) in self.draw_data.indices
                        [slot.indices.start..slot.indices.start + mesh.indices.len()]
                        .iter_mut()
                        .zip(&mesh.indices)
                    {
                        *target = index + slot.vertices.start as u32;
                    }
                    if slot.batches != mesh.batches {
                        slot.batches.clone_from(&mesh.batches);
                        commands_changed = true;
                    }
                    if !mesh.vertices.is_empty() {
                        // Newly reserved slack is zero-filled too. Include it so
                        // the complete GPU buffer matches DrawData after growth.
                        let end = if grow_vertices {
                            slot.vertices.end
                        } else {
                            slot.vertices.start + mesh.vertices.len()
                        };
                        update.vertices.push(slot.vertices.start..end);
                    }
                    if !mesh.indices.is_empty() {
                        let end = if grow_indices {
                            slot.indices.end
                        } else {
                            slot.indices.start + mesh.indices.len()
                        };
                        update.indices.push(slot.indices.start..end);
                    }
                }
                self.stats.geometry_bytes_copied += (mesh.vertices.len()
                    * std::mem::size_of::<Vertex>()
                    + mesh.indices.len() * std::mem::size_of::<u32>())
                    as u64;
            }
            if commands_changed {
                self.draw_data.commands.clear();
                for (element, slot) in self.elements.iter().zip(&self.mesh_slots) {
                    for (range, texture) in &slot.batches {
                        let indices = (range.start + slot.indices.start as u32)
                            ..(range.end + slot.indices.start as u32);
                        if let Some(last) = self.draw_data.commands.last_mut() {
                            if last.texture == *texture
                                && last.blur.is_none()
                                && element.blur.is_none()
                                && last.scroll_hint == element.scroll_hint
                                && last.clip_rect == element.clip
                                && last.indices.end == indices.start
                            {
                                last.indices.end = indices.end;
                                continue;
                            }
                        }
                        self.draw_data.commands.push(DrawCommand {
                            indices,
                            clip_rect: element.clip,
                            texture: *texture,
                            blur: element.blur,
                            scroll_hint: element.scroll_hint,
                        });
                    }
                }
            }
            self.draw_data.revision += 1;
            self.stats.geometry_rebuilds += 1;
            if compact {
                self.draw_data.geometry_update = None;
            } else {
                merge_ranges(&mut update.vertices);
                merge_ranges(&mut update.indices);
                self.draw_data.geometry_update = Some(update);
            }
        }
        self.draw_data.logical_size = self.logical_size;
        self.draw_data.scale_factor = self.scale;
        self.draw_data.textures = self.text.textures();
        let active = self.draw_data.commands.iter().map(|c| c.texture).collect();
        let (images, options) = self.images.payloads(&active);
        self.draw_data.textures.extend(images);
        self.draw_data.texture_options = options;
        self.draw_data.texture_budget_bytes = self.images.limits.gpu_cache_bytes;
        self.draw_data.texture_binding_budget = self.images.limits.max_gpu_bindings;
        self.previous_elements = keys;
    }
}

fn merge_ranges(ranges: &mut Vec<Range<usize>>) {
    ranges.sort_unstable_by_key(|range| range.start);
    let mut used = 0;
    for n in 0..ranges.len() {
        if used > 0 && ranges[n].start <= ranges[used - 1].end {
            ranges[used - 1].end = ranges[used - 1].end.max(ranges[n].end);
        } else {
            if used != n {
                ranges[used] = ranges[n].clone();
            }
            used += 1;
        }
    }
    ranges.truncate(used);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{vec2, DrawData};

    fn mesh(triangles: usize, x: f32) -> Arc<Mesh> {
        let mut mesh = Mesh::default();
        for n in 0..triangles {
            for p in [[x, n as f32], [x + 1.0, n as f32], [x, n as f32 + 1.0]] {
                mesh.vertices.push(Vertex {
                    position: p,
                    uv: [0.5; 2],
                    color: [0.5, 0.25, 0.75, 1.0],
                });
                mesh.indices.push(mesh.indices.len() as u32);
            }
        }
        if !mesh.indices.is_empty() {
            mesh.batches
                .push((0..mesh.indices.len() as u32, TextureId::WHITE));
        }
        Arc::new(mesh)
    }

    fn assert_paint_order(context: &Context) {
        let data = context.draw_data();
        let actual: Vec<_> = data
            .commands
            .iter()
            .flat_map(|c| {
                data.indices[c.indices.start as usize..c.indices.end as usize]
                    .iter()
                    .map(move |&i| (data.vertices[i as usize], c.texture, c.clip_rect, c.blur))
            })
            .collect();
        let expected: Vec<_> = context
            .elements
            .iter()
            .flat_map(|e| {
                e.mesh.batches.iter().flat_map(move |(r, texture)| {
                    e.mesh.indices[r.start as usize..r.end as usize]
                        .iter()
                        .map(move |&i| (e.mesh.vertices[i as usize], *texture, e.clip, e.blur))
                })
            })
            .collect();
        assert_eq!(actual, expected);
        assert!(data
            .indices
            .iter()
            .all(|&i| (i as usize) < data.vertices.len()));
        assert!(data
            .vertices
            .iter()
            .flat_map(|v| v.position.iter().chain(&v.uv).chain(&v.color))
            .all(|f| f.is_finite()));
    }

    #[test]
    fn local_growth_shrink_empty_and_reorder_preserve_complete_paint_stream() {
        let mut context = Context::new();
        context.logical_size = vec2(800.0, 600.0);
        context.elements = (0..40)
            .map(|i| Element {
                id: Id::new(i),
                layer: Id::new("panel"),
                clip: Rect::from_min_size(vec2(0.0, 0.0), vec2(800.0, 600.0)),
                mesh: mesh(8, i as f32),
                blur: None,
                scroll_hint: false,
            })
            .collect();
        context.rebuild_geometry();
        assert_paint_order(&context);
        let initial_full = context.stats.geometry_full_rebuilds;
        for triangles in [12, 3, 0, 20, 4, 32, 8, 1, 0, 2] {
            context.modified.clear();
            let id = context.elements[0].id;
            context.modified.insert(id);
            context.elements[0].mesh = mesh(triangles, triangles as f32);
            let previous_revision = context.draw_data.revision;
            let bytes = context.stats.geometry_bytes_copied;
            context.rebuild_geometry();
            let update = context.draw_data.geometry_update.as_ref().unwrap();
            assert_eq!(
                (update.from_revision, update.to_revision),
                (previous_revision, previous_revision + 1)
            );
            assert_eq!(
                context.stats.geometry_bytes_copied - bytes,
                (triangles * 3 * (32 + 4)) as u64
            );
            assert_eq!(context.stats.geometry_full_rebuilds, initial_full);
            assert_paint_order(&context);
        }
        context.modified.clear();
        context.elements[1].clip = context.elements[1].clip.shrink(15.0);
        context.elements[1].blur = Some(8.0);
        let bytes = context.stats.geometry_bytes_copied;
        context.rebuild_geometry();
        assert_eq!(context.stats.geometry_bytes_copied, bytes);
        assert!(context
            .draw_data
            .geometry_update
            .as_ref()
            .unwrap()
            .vertices
            .is_empty());
        assert_paint_order(&context);
        context.elements.reverse();
        context.rebuild_geometry();
        assert!(context.draw_data.geometry_update.is_none());
        assert_paint_order(&context);
        context.elements.truncate(4);
        context.rebuild_geometry();
        assert_paint_order(&context);
        context.elements.clear();
        context.rebuild_geometry();
        assert!(context.draw_data.vertices.is_empty() && context.draw_data.indices.is_empty());
    }

    #[test]
    fn compaction_bounds_slack_after_large_element_disappears() {
        let mut context = Context::new();
        context.logical_size = vec2(800.0, 600.0);
        context.elements = [2048, 1]
            .into_iter()
            .enumerate()
            .map(|(n, triangles)| Element {
                id: Id::new(n),
                layer: Id::new("panel"),
                clip: Rect::from_min_size(vec2(0.0, 0.0), vec2(800.0, 600.0)),
                mesh: mesh(triangles, 0.0),
                blur: None,
                scroll_hint: false,
            })
            .collect();
        context.rebuild_geometry();
        context.elements[0].mesh = mesh(1, 0.0);
        context.modified.insert(context.elements[0].id);
        context.rebuild_geometry();
        assert_eq!(context.draw_data.vertices.len(), 6);
        assert_eq!(context.draw_data.indices.len(), 6);
        assert_paint_order(&context);
    }

    #[test]
    fn dirty_range_merging_covers_overlaps_without_touching_gaps() {
        let mut ranges = vec![8..12, 0..3, 2..5, 5..7, 20..24];
        merge_ranges(&mut ranges);
        assert_eq!(ranges, [0..7, 8..12, 20..24]);
        let empty = DrawData::default();
        assert!(empty.geometry_update.is_none());
    }
}
