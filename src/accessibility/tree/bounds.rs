//! Stage 2: bounds of containers that never placed themselves, then physical pixels.

use super::Entry;
use crate::accessibility::{AccessNode, Geometry, Slot, NO_PARENT};
use crate::{Rect, Vec2};

/// Give every derived container the union of its published children, then convert the
/// bounds of the published nodes to physical pixels. Unpublished nodes are left as they
/// were collected; nothing reads their bounds.
pub(super) fn resolve(nodes: &mut [AccessNode], slots: &[Slot], entries: &[Entry], scale: f32) {
    derive(nodes, slots, entries);
    for (node, entry) in nodes.iter_mut().zip(entries) {
        if entry.alive {
            node.rect = physical(node.rect, scale);
            node.clip = physical(node.clip, scale);
        }
    }
}

/// Containers that never placed themselves take the union of their children. Children
/// come after their parent, so walking backwards settles a nested container before the
/// container that holds it.
fn derive(nodes: &mut [AccessNode], slots: &[Slot], entries: &[Entry]) {
    let mut seeded = vec![false; nodes.len()];
    for i in (0..nodes.len()).rev() {
        let parent = entries[i].parent;
        if !entries[i].alive || parent == NO_PARENT {
            continue;
        }
        let parent = parent as usize;
        if slots[parent].geometry != Geometry::Derived {
            continue;
        }
        let (rect, clip) = (nodes[i].rect, nodes[i].clip);
        let node = &mut nodes[parent];
        if seeded[parent] {
            node.rect = union(node.rect, rect);
            node.clip = union(node.clip, clip);
        } else {
            (node.rect, node.clip) = (rect, clip);
            seeded[parent] = true;
        }
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    Rect::from_min_max(a.min.min(b.min), a.max.max(b.max))
}

/// `rect` in physical pixels, on the pixel grid; anything not finite collapses to nothing.
pub(super) fn physical(rect: Rect, scale: f32) -> Rect {
    let snap = |v: f32| {
        let v = (v * scale).round();
        if v.is_finite() {
            v
        } else {
            0.0
        }
    };
    Rect::from_min_max(
        Vec2::new(snap(rect.min.x), snap(rect.min.y)),
        Vec2::new(snap(rect.max.x), snap(rect.max.y)),
    )
}
