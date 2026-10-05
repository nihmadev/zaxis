//! Stage 3: node ids, text run ids, what the hit regions allow, and relations.
//!
//! A node's id is its widget's [`Id`], so it is the same in every pass the widget is built
//! in, and again after it was gone. A repeated id is replaced by the first free alternate
//! in build order, so repeated widgets keep distinct ids that are stable across passes.

use super::Entry;
use crate::accessibility::{ids, AccessNode};
use crate::{context::HitRegion, Id};
use std::collections::{HashMap, HashSet};

/// What the hit regions of the pass say about the nodes.
pub(super) struct Hits<'a> {
    /// Whether the published hit region with an id takes keyboard focus; the last region
    /// registered with an id decides.
    focusable: HashMap<Id, bool>,
    /// Hit regions that take clicks, including those scrolled out of view.
    clickable: &'a HashSet<Id>,
}

impl<'a> Hits<'a> {
    pub(super) fn new(published: &[HitRegion], clickable: &'a HashSet<Id>) -> Self {
        let focusable = published
            .iter()
            .map(|hit| (hit.id, hit.action.focusable()))
            .collect();
        Self {
            focusable,
            clickable,
        }
    }
}

/// The ids of the published nodes.
pub(super) struct Identity {
    /// Text run ids of every published node, back to back; an entry's `runs` spans its own.
    pub runs: Vec<u64>,
    /// The node id of a widget id: the first node built with it.
    pub by_id: HashMap<Id, u64>,
    /// The node that stands for a focusable hit region: the first one built for it.
    pub focus_owner: HashMap<Id, u64>,
}

/// Give every published node its id, the ids of its text runs and its focus and click
/// flags, in build order.
pub(super) fn assign(nodes: &[AccessNode], entries: &mut [Entry], hits: &Hits) -> Identity {
    let mut used: HashSet<u64> = HashSet::with_capacity(nodes.len() + 4);
    used.extend([ids::ROOT, ids::ANNOUNCE_POLITE, ids::ANNOUNCE_ASSERTIVE]);
    let mut out = Identity {
        runs: Vec::new(),
        by_id: HashMap::with_capacity(nodes.len()),
        focus_owner: HashMap::new(),
    };
    for (node, entry) in nodes.iter().zip(entries.iter_mut()) {
        if !entry.alive {
            continue;
        }
        let nid = ids::unique(ids::node(node.id), &mut used);
        entry.nid = nid;
        out.by_id.entry(node.id).or_insert(nid);
        let focus = node.focus.unwrap_or(node.id);
        let focusable = hits.focusable.get(&focus).copied().unwrap_or_default();
        if focusable && !out.focus_owner.contains_key(&focus) {
            entry.focusable = true;
            out.focus_owner.insert(focus, nid);
        }
        entry.clickable = node.click.is_some_and(|id| hits.clickable.contains(&id));
        let start = out.runs.len() as u32;
        let runs = node.more.as_ref().and_then(|more| more.text.as_ref());
        for run in 0..runs.map_or(0, |text| text.0.runs.len()) {
            out.runs.push(ids::unique(ids::run(nid, run), &mut used));
        }
        entry.runs = (start, out.runs.len() as u32);
    }
    out
}

/// Hash the node ids each published node's relations resolve to, so a relation whose
/// target changed id sends the node again. Needs every id of the pass.
pub(super) fn relate(nodes: &[AccessNode], entries: &mut [Entry], by_id: &HashMap<Id, u64>) {
    for (node, entry) in nodes.iter().zip(entries.iter_mut()) {
        let Some(more) = node.more.as_ref().filter(|_| entry.alive) else {
            continue;
        };
        let targets = more
            .labelled_by
            .iter()
            .chain(&more.described_by)
            .chain(&more.controls)
            .chain(&more.error_message)
            .chain(&more.active_descendant);
        entry.links = targets.fold(0, |hash, id| {
            fold(hash, by_id.get(id).copied().unwrap_or(0))
        });
    }
}

fn fold(hash: u64, value: u64) -> u64 {
    (hash ^ value)
        .wrapping_mul(0x0000_0100_0000_01b3)
        .rotate_left(17)
}
