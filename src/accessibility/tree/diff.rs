//! Stage 5: what differs from the previous tree, and the update that carries it. The
//! previous tree is only read here; `Snapshot::keep` replaces it afterwards.
//!
//! A node is sent when its own description, its flags, its child list or its resolved
//! relations changed; a removed node is sent as its parent's shorter child list. The first
//! tree, a tree at a new scale and the tree after an update nobody took are sent whole.

use super::{outline::ANNOUNCEMENTS, policy::Motion, span, Built, Entry, Snapshot};
use crate::accessibility::{convert, ids, AccessNode, Announcement};
use crate::Id;
use accesskit::{Node, NodeId, TreeId, TreeInfo, TreeUpdate};

/// How this pass is compared with the previous tree.
#[derive(Clone, Copy)]
pub(super) struct Rules {
    /// Send every node.
    pub full: bool,
    /// Bounds-only changes keep their published bounds (see `policy`).
    pub hold: bool,
}

/// The nodes that differ from the previous tree, converted.
pub(super) struct Changes {
    pub nodes: Vec<(NodeId, Node)>,
    /// A node kept its published bounds.
    pub held: bool,
}

enum Compared {
    Same,
    /// Only its bounds changed, and it keeps the published ones.
    Held,
    Changed,
}

impl Snapshot {
    /// How a pass at `scale` is compared with this tree: whole when there is no tree yet,
    /// the scale changed or the last update was not taken.
    pub(super) fn rules(&self, scale: f32, motion: Motion) -> Rules {
        Rules {
            full: !self.built || self.update.is_some() || self.scale != scale,
            hold: self.hold.applies(motion),
        }
    }
}

/// The published nodes of `built` that differ from `previous`, each followed by its text
/// runs. A held node gets its published bounds back in `nodes`, so the snapshot keeps
/// what assistive technology was told.
pub(super) fn changes(
    nodes: &mut [AccessNode],
    built: &Built,
    previous: &Snapshot,
    rules: Rules,
) -> Changes {
    let resolve = |id: Id| built.by_id.get(&id).copied();
    let mut out = Changes {
        nodes: Vec::new(),
        held: false,
    };
    for (node, entry) in nodes.iter_mut().zip(&built.entries) {
        if !entry.alive {
            continue;
        }
        let children = built.children(entry);
        let compared = match previous.index.get(&entry.nid) {
            Some(at) if !rules.full => {
                compare(node, entry, children, previous, *at as usize, rules.hold)
            }
            _ => Compared::Changed,
        };
        match compared {
            Compared::Same => {}
            Compared::Held => out.held = true,
            Compared::Changed => {
                let runs = &built.runs[span(entry.runs)];
                let converted = convert::node(node, entry, children, runs, built.scale, &resolve);
                out.nodes.push((NodeId(entry.nid), converted));
                convert::runs(node, runs, built.scale, &mut out.nodes);
            }
        }
    }
    out
}

/// How `node` compares with the node published at `at` in `previous`.
fn compare(
    node: &mut AccessNode,
    entry: &Entry,
    children: &[u64],
    previous: &Snapshot,
    at: usize,
    hold: bool,
) -> Compared {
    let (old, was) = (&previous.nodes[at], &previous.entries[at]);
    let flags = |e: &Entry| (e.focusable, e.clickable, e.scrolled, e.links);
    if flags(was) != flags(entry) || previous.children(was) != children {
        return Compared::Changed;
    }
    if old == node {
        return Compared::Same;
    }
    if !hold {
        return Compared::Changed;
    }
    // In motion, a node that differs only in where it is keeps its published bounds until
    // the motion ends or the hold expires.
    let bounds = (node.rect, node.clip);
    (node.rect, node.clip) = (old.rect, old.clip);
    if old == node {
        return Compared::Held;
    }
    (node.rect, node.clip) = bounds;
    Compared::Changed
}

/// The window and the live regions, appended to `out` when they differ from `previous`.
pub(super) fn window(
    built: &Built,
    previous: &Snapshot,
    full: bool,
    announcements: &[Announcement; 2],
    out: &mut Vec<(NodeId, Node)>,
) {
    if full || previous.root != built.root || previous.window != built.window {
        let (title, size) = (built.window.0.as_deref(), built.window.1);
        out.push((NodeId(ids::ROOT), convert::root(title, size, &built.root)));
    }
    for (slot, id) in ANNOUNCEMENTS.into_iter().enumerate() {
        let serial = built.announced[slot];
        if serial > 0 && (full || serial != previous.announced[slot]) {
            let node = convert::announcement(&announcements[slot], slot == 1);
            out.push((NodeId(id), node));
        }
    }
}

/// The update of the pass: `None` when no node changed and focus stayed where it was.
pub(super) fn update(
    nodes: Vec<(NodeId, Node)>,
    full: bool,
    focus: u64,
    previous_focus: u64,
) -> Option<TreeUpdate> {
    if !full && nodes.is_empty() && focus == previous_focus {
        return None;
    }
    Some(TreeUpdate {
        nodes,
        tree: full.then(|| {
            let mut info = TreeInfo::new(NodeId(ids::ROOT));
            info.toolkit_name = Some("zaxis".into());
            info.toolkit_version = Some(env!("CARGO_PKG_VERSION").into());
            info
        }),
        tree_id: TreeId::ROOT,
        focus: NodeId(focus),
    })
}
