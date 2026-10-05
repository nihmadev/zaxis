//! Stage 1: which collected nodes are published, and where they hang.
//!
//! A node is published when the caller did not hide it, it is still placed (a dismissed
//! popup takes its hit regions with it), its parent is published, and it is not behind an
//! open modal dialog. A node that is not published takes its whole subtree with it, so a
//! hidden, unplaced or blocked node never comes back into the tree through a descendant.

use super::Entry;
use crate::accessibility::{AccessNode, AccessRole, Geometry, Slot, NO_PARENT};
use crate::Id;
use std::collections::HashMap;

/// The published part of the collected nodes.
pub(super) struct Selection {
    /// One per collected node, in build order: `alive`, `parent` and `scrolled` are set.
    pub entries: Vec<Entry>,
    /// Published nodes without a parent: by layer, bottom first, then in build order.
    pub top: Vec<usize>,
}

/// Where the layers of the pass are.
pub(super) struct Layers<F> {
    /// The rank of the top modal's layer; layers below it are blocked.
    pub modal: Option<usize>,
    /// The rank of a layer, bottom first.
    pub rank: F,
}

/// Mark the nodes that are published, their parents and whether a scrollable container
/// holds them. `nodes` and `slots` are parallel: an index names the same node in each, and
/// a parent is always built before its children.
pub(super) fn published(
    nodes: &[AccessNode],
    slots: &[Slot],
    layers: Layers<impl Fn(Id) -> usize>,
) -> Selection {
    debug_assert_eq!(nodes.len(), slots.len());
    let mut ranks: HashMap<Id, usize> = HashMap::new();
    let mut entries = vec![Entry::default(); nodes.len()];
    let mut top = Vec::new();
    for (i, (node, slot)) in nodes.iter().zip(slots).enumerate() {
        let alive = !slot.removed
            && slot.geometry != Geometry::Pending
            && match slot.parent {
                NO_PARENT => {
                    let layer = node.layer;
                    let rank = *ranks.entry(layer).or_insert_with(|| (layers.rank)(layer));
                    top.push((rank, i));
                    layers.modal.is_none_or(|modal| rank >= modal) || escapes_modal(node.role)
                }
                parent => entries[parent as usize].alive,
            };
        entries[i].alive = alive;
        entries[i].parent = slot.parent;
        if slot.parent != NO_PARENT {
            let parent = slot.parent as usize;
            let scrolls = nodes[parent]
                .more
                .as_ref()
                .is_some_and(|more| more.scroll.is_some());
            entries[i].scrolled = entries[parent].scrolled || scrolls;
        }
    }
    top.retain(|(_, i)| entries[*i].alive);
    top.sort();
    Selection {
        entries,
        top: top.into_iter().map(|(_, i)| i).collect(),
    }
}

/// Messages and hints stay published even behind a modal dialog.
fn escapes_modal(role: AccessRole) -> bool {
    matches!(
        role,
        AccessRole::Status | AccessRole::Alert | AccessRole::Tooltip
    )
}
