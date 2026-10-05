//! Building the tree of a pass and the update that takes the previous tree to it.
//!
//! The previous pass's nodes are kept as they were collected, so comparing is cheap and
//! only nodes that differ are converted to AccessKit nodes. A node is sent when its own
//! description, its children or its resolved relations changed; a removed widget is sent as
//! its parent's shorter child list.

use super::{convert, ids, AccessNode, AccessRole, Geometry, NO_PARENT};
use crate::{time::Instant, Context, Id, Rect, Vec2};
use accesskit::{NodeId, TreeId, TreeInfo, TreeUpdate};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

/// Bounds that change while the UI is in motion are sent at most this often.
const HOLD: Duration = Duration::from_millis(200);

/// What the tree knows about one node besides its description.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Entry {
    pub nid: u64,
    pub parent: u32,
    pub alive: bool,
    pub focusable: bool,
    pub clickable: bool,
    /// Inside a scrollable container: `ScrollIntoView` applies.
    pub scrolled: bool,
    /// Hash of the node ids its relations resolved to.
    pub links: u64,
    children: (u32, u32),
    runs: (u32, u32),
}

/// Counters for tests and benchmarks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AccessStats {
    /// Passes that built a tree.
    pub passes: u64,
    /// Updates produced; a pass that changes nothing produces none.
    pub updates: u64,
    /// Updates that carried the whole tree.
    pub full_updates: u64,
    /// Nodes converted and sent, text runs included.
    pub nodes_sent: u64,
    /// Nodes in the last tree, text runs excluded.
    pub nodes: u64,
}

/// The tree of the last pass.
#[derive(Default)]
pub(crate) struct Snapshot {
    pub nodes: Vec<AccessNode>,
    pub entries: Vec<Entry>,
    children: Vec<u64>,
    pub index: HashMap<u64, u32>,
    /// Text run id to its node and run number.
    pub runs: HashMap<u64, (u32, u32)>,
    root: Vec<u64>,
    root_state: (Option<String>, Vec2),
    /// Serials of the announcements last published, polite then assertive.
    announced: [u64; 2],
    pub focus: u64,
    pub scale: f32,
    built: bool,
    pub update: Option<TreeUpdate>,
    held_since: Option<Instant>,
    pub stats: AccessStats,
}

impl Snapshot {
    /// Forget the tree: the next pass that builds one sends all of it.
    pub(crate) fn reset(&mut self) {
        let stats = self.stats;
        *self = Self {
            stats,
            ..Self::default()
        };
    }

    pub(crate) fn children(&self, entry: &Entry) -> &[u64] {
        &self.children[entry.children.0 as usize..entry.children.1 as usize]
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    Rect::from_min_max(a.min.min(b.min), a.max.max(b.max))
}

/// `rect` in physical pixels, on the pixel grid; anything not finite collapses to nothing.
fn physical(rect: Rect, scale: f32) -> Rect {
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

fn fold(hash: u64, value: u64) -> u64 {
    (hash ^ value)
        .wrapping_mul(0x0000_0100_0000_01b3)
        .rotate_left(17)
}

impl Context {
    /// End of pass: turn the collected nodes into the tree and the update that leads to it.
    // Nodes, slots and entries are parallel tables; an index names the same node in each.
    #[allow(clippy::needless_range_loop)]
    pub(crate) fn finish_accessibility(&mut self) {
        if !self.a11y.active {
            return;
        }
        self.a11y.stack.clear();
        let scale = self.scale_factor();
        let count = self.a11y.nodes.len();
        let modal = self.top_modal_id();
        let modal_rank = modal.map(|id| self.layer_rank(id));
        let mut ranks: HashMap<Id, usize> = HashMap::new();
        let hits: HashMap<Id, (bool, bool)> = self
            .interaction
            .previous_hits
            .iter()
            .map(|hit| (hit.id, (hit.action.focusable(), hit.action.sense().click())))
            .collect();

        // Which nodes are published: not hidden by the caller, still placed (a dismissed
        // popup takes its hit regions with it), under a published parent, and not behind
        // an open modal dialog.
        let mut entries = vec![Entry::default(); count];
        let mut top = Vec::new();
        for i in 0..count {
            let slot = self.a11y.slots[i];
            let node = &self.a11y.nodes[i];
            let alive = !slot.removed
                && slot.geometry != Geometry::Pending
                && match slot.parent {
                    NO_PARENT => {
                        let layer = node.layer;
                        let rank = *ranks.entry(layer).or_insert_with(|| self.layer_rank(layer));
                        top.push((rank, i));
                        modal_rank.is_none_or(|modal| rank >= modal)
                            || matches!(
                                node.role,
                                AccessRole::Status | AccessRole::Alert | AccessRole::Tooltip
                            )
                    }
                    parent => entries[parent as usize].alive,
                };
            entries[i].alive = alive;
            entries[i].parent = slot.parent;
            if slot.parent != NO_PARENT {
                let parent = entries[slot.parent as usize];
                let scrolls = self.a11y.nodes[slot.parent as usize]
                    .more
                    .as_ref()
                    .is_some_and(|more| more.scroll.is_some());
                entries[i].scrolled = parent.scrolled || scrolls;
            }
        }
        top.retain(|(_, i)| entries[*i].alive);
        top.sort();

        // Containers that never placed themselves take the union of their children.
        let mut seeded = vec![false; count];
        for i in (0..count).rev() {
            let parent = entries[i].parent;
            if !entries[i].alive || parent == NO_PARENT {
                continue;
            }
            let parent = parent as usize;
            if self.a11y.slots[parent].geometry != Geometry::Derived {
                continue;
            }
            let (rect, clip) = (self.a11y.nodes[i].rect, self.a11y.nodes[i].clip);
            let node = &mut self.a11y.nodes[parent];
            if seeded[parent] {
                node.rect = union(node.rect, rect);
                node.clip = union(node.clip, clip);
            } else {
                (node.rect, node.clip) = (rect, clip);
                seeded[parent] = true;
            }
        }

        // Node ids; a repeated id is replaced deterministically.
        let mut used: HashSet<u64> = HashSet::with_capacity(count + 4);
        used.extend([ids::ROOT, ids::ANNOUNCE_POLITE, ids::ANNOUNCE_ASSERTIVE]);
        let mut by_id: HashMap<Id, u64> = HashMap::with_capacity(count);
        let mut focus_owner: HashMap<Id, u64> = HashMap::new();
        let mut run_ids = Vec::new();
        for i in 0..count {
            if !entries[i].alive {
                continue;
            }
            let node = &mut self.a11y.nodes[i];
            node.rect = physical(node.rect, scale);
            node.clip = physical(node.clip, scale);
            let base = ids::node(node.id);
            let (mut nid, mut attempt) = (base, 0);
            while !used.insert(nid) {
                attempt += 1;
                nid = ids::alternate(base, attempt);
            }
            entries[i].nid = nid;
            by_id.entry(node.id).or_insert(nid);
            let focus = node.focus.unwrap_or(node.id);
            let hit = hits.get(&focus).copied().unwrap_or_default();
            if hit.0 && !focus_owner.contains_key(&focus) {
                entries[i].focusable = true;
                focus_owner.insert(focus, nid);
            }
            entries[i].clickable = node
                .click
                .is_some_and(|id| self.a11y.clickable.contains(&id));
            let start = run_ids.len() as u32;
            let runs = node.more.as_ref().and_then(|m| m.text.as_ref());
            for run in 0..runs.map_or(0, |text| text.0.runs.len()) {
                let base = ids::run(nid, run);
                let (mut id, mut attempt) = (base, 0);
                while !used.insert(id) {
                    attempt += 1;
                    id = ids::alternate(base, attempt);
                }
                run_ids.push(id);
            }
            entries[i].runs = (start, run_ids.len() as u32);
        }

        // Child lists: text runs first, then child nodes in build order.
        let mut lists: Vec<Vec<u64>> = vec![Vec::new(); count];
        for i in 0..count {
            let entry = entries[i];
            if !entry.alive {
                continue;
            }
            lists[i].extend(&run_ids[entry.runs.0 as usize..entry.runs.1 as usize]);
            if entry.parent != NO_PARENT {
                lists[entry.parent as usize].push(entry.nid);
            }
            if let Some(more) = &self.a11y.nodes[i].more {
                let targets = more
                    .labelled_by
                    .iter()
                    .chain(&more.described_by)
                    .chain(&more.controls)
                    .chain(&more.error_message)
                    .chain(&more.active_descendant);
                entries[i].links = targets.fold(0, |hash, id| {
                    fold(hash, by_id.get(id).copied().unwrap_or(0))
                });
            }
        }
        let mut children = Vec::with_capacity(count);
        for (i, list) in lists.iter().enumerate() {
            let start = children.len() as u32;
            children.extend(list);
            entries[i].children = (start, children.len() as u32);
        }
        let mut root: Vec<u64> = top.iter().map(|(_, i)| entries[*i].nid).collect();
        let serials = [0, 1].map(|slot| self.a11y.announcements[slot].serial);
        for (slot, id) in [ids::ANNOUNCE_POLITE, ids::ANNOUNCE_ASSERTIVE]
            .into_iter()
            .enumerate()
        {
            if serials[slot] > 0 {
                root.push(id);
            }
        }

        let dialog = modal.and_then(|layer| {
            top.iter().rev().map(|(_, i)| *i).find(|i| {
                self.a11y.nodes[*i].layer == layer
                    && matches!(
                        self.a11y.nodes[*i].role,
                        AccessRole::Dialog | AccessRole::AlertDialog
                    )
            })
        });
        let focus = self
            .focused()
            .and_then(|id| focus_owner.get(&id).copied())
            .or(dialog.map(|i| entries[i].nid))
            .unwrap_or(ids::ROOT);

        // The update: everything on the first pass, otherwise what differs.
        let moving = self.repaint_requested() || self.wants_animation_frame();
        let now = self.frame_time();
        let size = physical(self.viewport(), scale).size();
        let tree = &mut self.a11y.tree;
        let full = !tree.built || tree.update.is_some() || tree.scale != scale;
        let hold = moving
            && tree
                .held_since
                .is_none_or(|since| now.saturating_duration_since(since) < HOLD);
        let mut held = false;
        let mut out = Vec::new();
        let resolve = |id: Id| by_id.get(&id).copied();
        for i in 0..count {
            let entry = entries[i];
            if !entry.alive {
                continue;
            }
            let list = &children[entry.children.0 as usize..entry.children.1 as usize];
            let node = &mut self.a11y.nodes[i];
            let previous = tree.index.get(&entry.nid).map(|at| *at as usize);
            let changed = full
                || previous.is_none_or(|at| {
                    let (old, was) = (&tree.nodes[at], &tree.entries[at]);
                    if (was.focusable, was.clickable, was.scrolled, was.links)
                        != (
                            entry.focusable,
                            entry.clickable,
                            entry.scrolled,
                            entry.links,
                        )
                        || tree.children(was) != list
                    {
                        return true;
                    }
                    if old == node {
                        return false;
                    }
                    // In motion, a node that differs only in where it is keeps its
                    // published bounds until the motion ends or the hold expires.
                    let bounds = (node.rect, node.clip);
                    (node.rect, node.clip) = (old.rect, old.clip);
                    if hold && old == node {
                        held = true;
                        return false;
                    }
                    (node.rect, node.clip) = bounds;
                    true
                });
            if changed {
                let runs = &run_ids[entry.runs.0 as usize..entry.runs.1 as usize];
                out.push((
                    NodeId(entry.nid),
                    convert::node(node, &entry, list, runs, scale, &resolve),
                ));
                convert::runs(node, runs, scale, &mut out);
            }
        }
        let title = self.a11y.title.clone();
        let root_state = (title, size);
        if full || tree.root != root || tree.root_state != root_state {
            out.push((
                NodeId(ids::ROOT),
                convert::root(root_state.0.as_deref(), size, &root),
            ));
        }
        for (slot, id) in [ids::ANNOUNCE_POLITE, ids::ANNOUNCE_ASSERTIVE]
            .into_iter()
            .enumerate()
        {
            if serials[slot] > 0 && (full || serials[slot] != tree.announced[slot]) {
                let announcement = &self.a11y.announcements[slot];
                out.push((NodeId(id), convert::announcement(announcement, slot == 1)));
            }
        }

        tree.stats.passes += 1;
        tree.stats.nodes = entries.iter().filter(|entry| entry.alive).count() as u64;
        if full || !out.is_empty() || tree.focus != focus {
            tree.stats.updates += 1;
            tree.stats.full_updates += u64::from(full);
            tree.stats.nodes_sent += out.len() as u64;
            tree.update = Some(TreeUpdate {
                nodes: out,
                tree: full.then(|| {
                    let mut info = TreeInfo::new(NodeId(ids::ROOT));
                    info.toolkit_name = Some("zaxis".into());
                    info.toolkit_version = Some(env!("CARGO_PKG_VERSION").into());
                    info
                }),
                tree_id: TreeId::ROOT,
                focus: NodeId(focus),
            });
        }
        tree.held_since = match (held, tree.held_since) {
            (true, since) => since.or(Some(now)),
            (false, _) => None,
        };
        tree.index.clear();
        tree.runs.clear();
        for (i, entry) in entries.iter().enumerate().filter(|(_, entry)| entry.alive) {
            tree.index.insert(entry.nid, i as u32);
            let ids = &run_ids[entry.runs.0 as usize..entry.runs.1 as usize];
            for (run, id) in ids.iter().enumerate() {
                tree.runs.insert(*id, (i as u32, run as u32));
            }
        }
        std::mem::swap(&mut tree.nodes, &mut self.a11y.nodes);
        tree.entries = entries;
        tree.children = children;
        tree.root = root;
        tree.root_state = root_state;
        tree.announced = serials;
        tree.focus = focus;
        tree.scale = scale;
        tree.built = true;
        if held {
            // The held bounds are published once the motion stops or the hold expires.
            self.request_repaint_after(HOLD);
        }
    }
}
