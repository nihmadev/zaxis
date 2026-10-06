use crate::prelude::*;
use std::sync::Arc;
use zaxis::context::geometry::merge_ranges;
use zaxis::shapes::Mesh;
use zaxis::{vec2, DrawData};

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
        .probe()
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
    *context.probe_mut().logical_size = vec2(800.0, 600.0);
    *context.probe_mut().elements = (0..40)
        .map(|i| Element {
            id: Id::new(i),
            layer: Id::new("panel"),
            clip: Rect::from_min_size(vec2(0.0, 0.0), vec2(800.0, 600.0)),
            mesh: mesh(8, i as f32),
            blur: None,
            scroll_hint: false,
            material: None,
        })
        .collect();
    context.rebuild_geometry();
    assert_paint_order(&context);
    let initial_full = context.probe_mut().stats.geometry_full_rebuilds;
    for triangles in [12, 3, 0, 20, 4, 32, 8, 1, 0, 2] {
        context.probe_mut().modified.clear();
        let id = context.probe_mut().elements[0].id;
        context.probe_mut().modified.insert(id);
        context.probe_mut().elements[0].mesh = mesh(triangles, triangles as f32);
        let previous_revision = context.probe_mut().draw_data.revision;
        let bytes = context.probe_mut().stats.geometry_bytes_copied;
        context.rebuild_geometry();
        let update = context
            .probe_mut()
            .draw_data
            .geometry_update
            .as_ref()
            .unwrap();
        assert_eq!(
            (update.from_revision, update.to_revision),
            (previous_revision, previous_revision + 1)
        );
        assert_eq!(
            context.probe_mut().stats.geometry_bytes_copied - bytes,
            (triangles * 3 * (32 + 4)) as u64
        );
        assert_eq!(
            context.probe_mut().stats.geometry_full_rebuilds,
            initial_full
        );
        assert_paint_order(&context);
    }
    context.probe_mut().modified.clear();
    context.probe_mut().elements[1].clip = context.probe_mut().elements[1].clip.shrink(15.0);
    context.probe_mut().elements[1].blur = Some(8.0);
    let bytes = context.probe_mut().stats.geometry_bytes_copied;
    context.rebuild_geometry();
    assert_eq!(context.probe_mut().stats.geometry_bytes_copied, bytes);
    assert!(context
        .probe()
        .draw_data
        .geometry_update
        .as_ref()
        .unwrap()
        .vertices
        .is_empty());
    assert_paint_order(&context);
    context.probe_mut().elements.reverse();
    context.rebuild_geometry();
    assert!(context.probe_mut().draw_data.geometry_update.is_none());
    assert_paint_order(&context);
    context.probe_mut().elements.truncate(4);
    context.rebuild_geometry();
    assert_paint_order(&context);
    context.probe_mut().elements.clear();
    context.rebuild_geometry();
    assert!(
        context.probe_mut().draw_data.vertices.is_empty()
            && context.probe_mut().draw_data.indices.is_empty()
    );
}

#[test]
fn compaction_bounds_slack_after_large_element_disappears() {
    let mut context = Context::new();
    *context.probe_mut().logical_size = vec2(800.0, 600.0);
    *context.probe_mut().elements = [2048, 1]
        .into_iter()
        .enumerate()
        .map(|(n, triangles)| Element {
            id: Id::new(n),
            layer: Id::new("panel"),
            clip: Rect::from_min_size(vec2(0.0, 0.0), vec2(800.0, 600.0)),
            mesh: mesh(triangles, 0.0),
            blur: None,
            scroll_hint: false,
            material: None,
        })
        .collect();
    context.rebuild_geometry();
    context.probe_mut().elements[0].mesh = mesh(1, 0.0);
    let id = context.probe().elements[0].id;
    context.probe_mut().modified.insert(id);
    context.rebuild_geometry();
    assert_eq!(context.probe_mut().draw_data.vertices.len(), 6);
    assert_eq!(context.probe_mut().draw_data.indices.len(), 6);
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
