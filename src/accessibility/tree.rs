//! Building the tree of a pass and the update that takes the previous tree to it.
//!
//! The previous pass's nodes are kept as they were collected, so comparing is cheap and
//! only nodes that differ are converted to AccessKit nodes. The end of a pass runs in
//! stages, each with explicit inputs and results:
//!
//! 1. [`select`]: which collected nodes are published, their parents, the top-level order.
//! 2. [`bounds`]: bounds of derived containers, then physical pixels.
//! 3. [`identity`]: unique node and text run ids, focus and click flags, relations.
//! 4. [`outline`]: child lists, the window's children and the focused node.
//! 5. [`diff`]: what differs from the previous tree and the update that carries it; the
//!    tree then becomes the [`Snapshot`] the next pass compares with.
//!
//! [`policy`] decides when bounds that move with the UI are held back. Collected nodes,
//! their slots and the entries of every stage are parallel tables: an index names the same
//! node in each.

mod bounds;
mod diff;
mod identity;
mod outline;
mod policy;
mod select;

use super::AccessNode;
use crate::{Context, Id, Vec2};
use accesskit::TreeUpdate;
use std::{collections::HashMap, ops::Range};

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

fn span((start, end): (u32, u32)) -> Range<usize> {
    start as usize..end as usize
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

/// The tree of this pass, from stages 1 to 4, before it is compared with the previous one.
struct Built {
    entries: Vec<Entry>,
    children: Vec<u64>,
    runs: Vec<u64>,
    root: Vec<u64>,
    /// The window's title and size in physical pixels.
    window: (Option<String>, Vec2),
    /// Serials of the announcements, polite then assertive.
    announced: [u64; 2],
    focus: u64,
    by_id: HashMap<Id, u64>,
    /// Physical pixels per logical pixel.
    scale: f32,
}

impl Built {
    fn children(&self, entry: &Entry) -> &[u64] {
        &self.children[span(entry.children)]
    }
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
    window: (Option<String>, Vec2),
    /// Serials of the announcements last published, polite then assertive.
    announced: [u64; 2],
    pub focus: u64,
    pub scale: f32,
    built: bool,
    pub update: Option<TreeUpdate>,
    hold: policy::MotionHold,
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
        &self.children[span(entry.children)]
    }

    /// Count the pass, keep its update for the host and make `built` the tree the next
    /// pass compares with. The collected `nodes` move in; the previous ones go back to be
    /// collected into.
    fn keep(&mut self, built: Built, nodes: &mut Vec<AccessNode>, update: Option<TreeUpdate>) {
        self.stats.passes += 1;
        self.stats.nodes = built.entries.iter().filter(|entry| entry.alive).count() as u64;
        if let Some(update) = update {
            self.stats.updates += 1;
            self.stats.full_updates += u64::from(update.tree.is_some());
            self.stats.nodes_sent += update.nodes.len() as u64;
            self.update = Some(update);
        }
        self.index.clear();
        self.runs.clear();
        for (i, entry) in built.entries.iter().enumerate() {
            if !entry.alive {
                continue;
            }
            self.index.insert(entry.nid, i as u32);
            for (run, id) in built.runs[span(entry.runs)].iter().enumerate() {
                self.runs.insert(*id, (i as u32, run as u32));
            }
        }
        std::mem::swap(&mut self.nodes, nodes);
        self.entries = built.entries;
        self.children = built.children;
        self.root = built.root;
        self.window = built.window;
        self.announced = built.announced;
        self.focus = built.focus;
        self.scale = built.scale;
        self.built = true;
    }
}

impl Context {
    /// End of pass: turn the collected nodes into the tree and the update that leads to it.
    pub(crate) fn finish_accessibility(&mut self) {
        if !self.a11y.active {
            return;
        }
        self.a11y.stack.clear();
        let scale = self.scale_factor();
        let modal = self.top_modal_id();
        // 1. Published nodes.
        let layers = select::Layers {
            modal: modal.map(|layer| self.layer_rank(layer)),
            rank: |layer| self.layer_rank(layer),
        };
        let select::Selection { mut entries, top } =
            select::published(&self.a11y.nodes, &self.a11y.slots, layers);
        // 2. Bounds.
        bounds::resolve(&mut self.a11y.nodes, &self.a11y.slots, &entries, scale);
        // 3. Ids, flags and relations.
        let hits = identity::Hits::new(&self.interaction.previous_hits, &self.a11y.clickable);
        let identity = identity::assign(&self.a11y.nodes, &mut entries, &hits);
        identity::relate(&self.a11y.nodes, &mut entries, &identity.by_id);
        // 4. Children, the window and focus.
        let announced = self.a11y.announcements.each_ref().map(|a| a.serial);
        let outline = outline::build(&mut entries, &identity.runs, &top, announced);
        let dialog = outline::dialog(&self.a11y.nodes, &entries, &top, modal);
        let built = Built {
            focus: outline::focus(self.focused(), &identity.focus_owner, dialog),
            window: (
                self.a11y.title.clone(),
                bounds::physical(self.viewport(), scale).size(),
            ),
            entries,
            children: outline.children,
            runs: identity.runs,
            root: outline.root,
            announced,
            by_id: identity.by_id,
            scale,
        };
        // 5. The update from the previous tree, which this one then replaces.
        let motion = policy::Motion {
            moving: self.repaint_requested() || self.wants_animation_frame(),
            now: self.frame_time(),
        };
        let tree = &mut self.a11y.tree;
        let rules = tree.rules(scale, motion);
        let mut changes = diff::changes(&mut self.a11y.nodes, &built, tree, rules);
        diff::window(
            &built,
            tree,
            rules.full,
            &self.a11y.announcements,
            &mut changes.nodes,
        );
        let update = diff::update(changes.nodes, rules.full, built.focus, tree.focus);
        tree.keep(built, &mut self.a11y.nodes, update);
        if let Some(delay) = tree.hold.settle(changes.held, motion.now) {
            // What was held is published once the motion stops or the hold expires.
            self.request_repaint_after(delay);
        }
    }
}
