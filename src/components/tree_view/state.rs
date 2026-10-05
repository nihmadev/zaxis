use super::{TreeChildren, TreeEvent, TreeIssue, TreeModel};
use crate::Id;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
pub(super) struct Row {
    pub id: Id,
    pub parent: Option<Id>,
    pub depth: usize,
    pub enabled: bool,
    pub children: TreeChildren,
    /// Position among the children of `parent`, and how many there are.
    pub set: (u32, u32),
}
pub(crate) struct TreeState {
    pub last_frame: u64,
    pub(super) revision: Option<u64>,
    pub(super) open: HashSet<Id>,
    pub(super) selected: Option<Id>,
    pub(super) focused: Option<Id>,
    pub(super) rows: Vec<Row>,
    pub(super) index: HashMap<Id, usize>,
    pub(super) accessible: Vec<usize>,
    pub(super) issues: Vec<TreeIssue>,
    pub(super) dirty: bool,
    pub(super) scroll: bool,
    pub(super) pending_reveal: Option<Id>,
    pub(crate) action_ids: HashMap<Id, Id>,
    pub(super) owner_focused: bool,
}
/// A node waiting to become a row: id, parent, depth, and its place among its siblings.
type Pending = (Id, Option<Id>, usize, (u32, u32));
/// Give freshly pushed siblings their count and put the first one on top of the stack.
fn number_siblings(siblings: &mut [Pending]) {
    let count = siblings.len() as u32;
    for sibling in siblings.iter_mut() {
        sibling.3 .1 = count;
    }
    siblings.reverse();
}
impl TreeState {
    pub(super) fn new(open: Vec<Id>, selected: Option<Id>) -> Self {
        Self {
            last_frame: 0,
            revision: None,
            open: open.into_iter().collect(),
            selected,
            focused: None,
            rows: Vec::new(),
            index: HashMap::new(),
            accessible: Vec::new(),
            issues: Vec::new(),
            dirty: true,
            scroll: false,
            pending_reveal: None,
            action_ids: HashMap::new(),
            owner_focused: false,
        }
    }
    pub(super) fn rebuild(&mut self, model: &impl TreeModel, events: &mut Vec<TreeEvent>) {
        let previous_index = self
            .focused
            .and_then(|f| self.index.get(&f).copied())
            .unwrap_or(0);
        let previous_parent = self
            .focused
            .and_then(|f| self.index.get(&f))
            .and_then(|i| self.rows[*i].parent);
        let mut previous_ancestors = Vec::new();
        let mut ancestor = previous_parent;
        while let Some(id) = ancestor {
            previous_ancestors.push(id);
            ancestor = self.index.get(&id).and_then(|i| self.rows[*i].parent);
        }
        self.rows.clear();
        self.index.clear();
        self.accessible.clear();
        self.issues.clear();
        let roots = (0..).zip(model.roots());
        let mut stack: Vec<_> = roots.map(|(n, id)| (id, None, 0, (n, 0))).collect();
        number_siblings(&mut stack);
        let mut visited = HashSet::new();
        while let Some((id, parent, depth, set)) = stack.pop() {
            if !visited.insert(id) {
                self.issues.push(TreeIssue::DuplicateOrCycle(id));
                continue;
            }
            let Some(node) = model.node(id) else {
                self.issues.push(TreeIssue::MissingNode(id));
                continue;
            };
            let index = self.rows.len();
            self.index.insert(id, index);
            if node.enabled {
                self.accessible.push(index);
            }
            self.rows.push(Row {
                id,
                parent,
                depth,
                enabled: node.enabled,
                children: node.children,
                set,
            });
            if node.children.expandable() && self.open.contains(&id) {
                let start = stack.len();
                let children = (0..).zip(model.children(id));
                stack.extend(children.map(|(n, child)| (child, Some(id), depth + 1, (n, 0))));
                number_siblings(&mut stack[start..]);
            }
        }
        // Check only UI state IDs, not every closed subtree in the model.
        self.open.retain(|id| model.node(*id).is_some());
        if self.selected.is_some_and(|id| model.node(id).is_none()) {
            self.select(None, events);
        }
        if self
            .focused
            .is_none_or(|id| !self.index.contains_key(&id) || !self.rows[self.index[&id]].enabled)
        {
            let mut parent = self
                .focused
                .and_then(|id| model.parent(id))
                .or(previous_parent);
            let mut path = HashSet::new();
            let mut target = None;
            while let Some(id) = parent {
                if !path.insert(id) {
                    break;
                }
                if self.index.get(&id).is_some_and(|i| self.rows[*i].enabled) {
                    target = Some(id);
                    break;
                }
                parent = model
                    .parent(id)
                    .or_else(|| self.index.get(&id).and_then(|i| self.rows[*i].parent));
            }
            if target.is_none() {
                target = previous_ancestors
                    .into_iter()
                    .find(|id| self.index.get(id).is_some_and(|i| self.rows[*i].enabled));
            }
            if target.is_none() {
                target = self
                    .accessible
                    .iter()
                    .copied()
                    .find(|i| *i >= previous_index)
                    .or_else(|| self.accessible.last().copied())
                    .map(|i| self.rows[i].id);
            }
            if self.focused != target {
                self.focused = target;
                self.scroll = true;
            }
        }
        if let Some(id) = self.pending_reveal.filter(|id| self.index.contains_key(id)) {
            if self.rows[self.index[&id]].enabled {
                self.focused = Some(id);
                self.scroll = true;
            }
            self.pending_reveal = None;
        }
        self.revision = Some(model.revision());
        self.dirty = false;
    }
    pub(super) fn select(&mut self, node: Option<Id>, events: &mut Vec<TreeEvent>) {
        if self.selected != node {
            self.selected = node;
            events.push(TreeEvent::Selected { node });
        }
    }
    pub(super) fn set_open(
        &mut self,
        id: Id,
        open: bool,
        model: &impl TreeModel,
        events: &mut Vec<TreeEvent>,
    ) {
        let Some(node) = model.node(id).filter(|n| n.children.expandable()) else {
            return;
        };
        if self.open.contains(&id) == open {
            return;
        }
        if open {
            self.open.insert(id);
        } else {
            self.open.remove(&id);
        }
        events.push(TreeEvent::OpenChanged { node: id, open });
        if open && matches!(node.children, TreeChildren::Unloaded | TreeChildren::Error) {
            events.push(TreeEvent::RequestChildren { node: id });
        }
        self.dirty = true;
    }
    pub(super) fn reveal(&mut self, node: Id, model: &impl TreeModel, events: &mut Vec<TreeEvent>) {
        let mut path = HashSet::new();
        let mut ancestors = Vec::new();
        let mut current = Some(node);
        while let Some(id) = current {
            if !path.insert(id) || model.node(id).is_none() {
                self.issues.push(TreeIssue::InvalidRevealPath(id));
                return;
            }
            ancestors.push(id);
            current = model.parent(id);
        }
        if ancestors
            .last()
            .is_none_or(|root| !model.roots().any(|id| id == *root))
        {
            self.issues.push(TreeIssue::InvalidRevealPath(node));
            return;
        }
        for id in ancestors.into_iter().skip(1).rev() {
            self.set_open(id, true, model, events);
        }
        self.pending_reveal = Some(node);
        self.dirty = true;
    }
}
