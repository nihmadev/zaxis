//! Stage 4: child lists, the children of the window, and the node that has focus.

use super::{span, Entry};
use crate::accessibility::{ids, AccessNode, AccessRole, NO_PARENT};
use crate::Id;
use std::collections::HashMap;

/// The shape of the published tree.
pub(super) struct Outline {
    /// Child lists of every node, back to back; an entry's `children` spans its own.
    pub children: Vec<u64>,
    /// The window's children: top-level nodes as ordered by selection, then the live
    /// regions that have carried an announcement.
    pub root: Vec<u64>,
}

/// Child lists: a node's text runs first, then its published children in build order.
/// `announced` holds the serials of the polite and the assertive announcement.
pub(super) fn build(
    entries: &mut [Entry],
    runs: &[u64],
    top: &[usize],
    announced: [u64; 2],
) -> Outline {
    let mut lists: Vec<Vec<u64>> = vec![Vec::new(); entries.len()];
    for (i, entry) in entries.iter().enumerate() {
        if !entry.alive {
            continue;
        }
        // A parent comes before its children, so its runs are already in its list.
        lists[i].extend(&runs[span(entry.runs)]);
        if entry.parent != NO_PARENT {
            lists[entry.parent as usize].push(entry.nid);
        }
    }
    let mut children = Vec::with_capacity(entries.len());
    for (entry, list) in entries.iter_mut().zip(&lists) {
        let start = children.len() as u32;
        children.extend(list);
        entry.children = (start, children.len() as u32);
    }
    let mut root: Vec<u64> = top.iter().map(|i| entries[*i].nid).collect();
    for (serial, id) in announced.into_iter().zip(ANNOUNCEMENTS) {
        if serial > 0 {
            root.push(id);
        }
    }
    Outline { children, root }
}

/// The live regions of the polite and the assertive announcement.
pub(super) const ANNOUNCEMENTS: [u64; 2] = [ids::ANNOUNCE_POLITE, ids::ANNOUNCE_ASSERTIVE];

/// The dialog of the modal layer `modal`: its last top-level dialog node.
pub(super) fn dialog(
    nodes: &[AccessNode],
    entries: &[Entry],
    top: &[usize],
    modal: Option<Id>,
) -> Option<u64> {
    let layer = modal?;
    let found = top.iter().rev().find(|i| {
        nodes[**i].layer == layer
            && matches!(
                nodes[**i].role,
                AccessRole::Dialog | AccessRole::AlertDialog
            )
    });
    found.map(|i| entries[*i].nid)
}

/// The focused node: the one that stands for the focused widget, else the open dialog,
/// else the window itself.
pub(super) fn focus(
    focused: Option<Id>,
    focus_owner: &HashMap<Id, u64>,
    dialog: Option<u64>,
) -> u64 {
    focused
        .and_then(|id| focus_owner.get(&id).copied())
        .or(dialog)
        .unwrap_or(ids::ROOT)
}
