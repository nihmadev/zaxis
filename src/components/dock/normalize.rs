use super::{DockChild, DockFloat, DockIssue, DockNode, DockState, PanelId};
use crate::{Id, Rect, Vec2};
use std::collections::HashSet;

impl DockState {
    /// Validate against the application's live panel registry. Does not mutate data.
    pub fn validate(&self, available: impl IntoIterator<Item = PanelId>) -> Vec<DockIssue> {
        let available: HashSet<_> = available.into_iter().collect();
        let mut issues = Vec::new();
        let mut panels = HashSet::new();
        let mut nodes = HashSet::new();
        if let Some(root) = &self.root {
            validate(root, 0, &available, &mut panels, &mut nodes, &mut issues);
        }
        for f in &self.floats {
            validate(&f.node, 0, &available, &mut panels, &mut nodes, &mut issues);
            if !f.bounds.is_finite() || f.bounds.is_empty() {
                issues.push(DockIssue::InvalidBounds(f.node.id()));
            }
        }
        if let Some(p) = self.focused.filter(|p| !panels.contains(p)) {
            issues.push(DockIssue::MissingPanel(p));
        }
        issues
    }
    /// Repair duplicates, bad fractions/active IDs and empty or redundant splits.
    /// The first occurrence of a duplicate panel wins, in tree order.
    pub fn normalize(&mut self) {
        let mut panels = HashSet::new();
        let mut nodes = HashSet::new();
        self.root = self
            .root
            .take()
            .and_then(|r| normalize(r, 0, &mut panels, &mut nodes));
        self.floats = std::mem::take(&mut self.floats)
            .into_iter()
            .filter_map(|f| {
                let node = normalize(f.node, 0, &mut panels, &mut nodes)?;
                let bounds = if f.bounds.is_finite() && !f.bounds.is_empty() {
                    f.bounds
                } else {
                    Rect::from_min_size(Vec2::splat(40.0), Vec2::new(380.0, 240.0))
                };
                Some(DockFloat { node, bounds })
            })
            .collect();
        if self.focused.is_none_or(|p| !panels.contains(&p)) {
            self.focused = self
                .root
                .as_ref()
                .and_then(super::options::active)
                .or_else(|| {
                    self.floats
                        .iter()
                        .find_map(|f| super::options::active(&f.node))
                });
        }
    }
    /// Remove panels whose application data no longer exists, then normalize.
    pub fn retain_panels(&mut self, mut keep: impl FnMut(PanelId) -> bool) {
        self.normalize();
        fn retain(node: &mut DockNode, keep: &mut impl FnMut(PanelId) -> bool) {
            match node {
                DockNode::Tabs { panels, .. } => panels.retain(|p| keep(*p)),
                DockNode::Split { children, .. } => {
                    for c in children {
                        retain(&mut c.node, keep);
                    }
                }
            }
        }
        if let Some(root) = &mut self.root {
            retain(root, &mut keep);
        }
        for float in &mut self.floats {
            retain(&mut float.node, &mut keep);
        }
        self.normalize();
    }
}
fn validate(
    node: &DockNode,
    depth: usize,
    available: &HashSet<Id>,
    panels: &mut HashSet<Id>,
    nodes: &mut HashSet<Id>,
    issues: &mut Vec<DockIssue>,
) {
    if depth > 128 {
        issues.push(DockIssue::TooDeep);
        return;
    }
    if !nodes.insert(node.id()) {
        issues.push(DockIssue::DuplicateNode(node.id()));
    }
    match node {
        DockNode::Tabs {
            panels: ids,
            active,
            ..
        } => {
            for p in ids {
                if !panels.insert(*p) {
                    issues.push(DockIssue::DuplicatePanel(*p));
                }
                if !available.contains(p) {
                    issues.push(DockIssue::MissingPanel(*p));
                }
            }
            if let Some(p) = active.filter(|p| !ids.contains(p)) {
                issues.push(DockIssue::MissingPanel(p));
            }
        }
        DockNode::Split { children, .. } => {
            for c in children {
                if !c.fraction.is_finite() || c.fraction <= 0.0 {
                    issues.push(DockIssue::InvalidFraction(node.id()));
                }
                validate(&c.node, depth + 1, available, panels, nodes, issues);
            }
        }
    }
}
fn normalize(
    node: DockNode,
    depth: usize,
    panels_seen: &mut HashSet<Id>,
    nodes: &mut HashSet<Id>,
) -> Option<DockNode> {
    if depth > 128 {
        // Explicit worklist avoids recursive Drop on malformed application trees.
        let mut pending = vec![node];
        while let Some(node) = pending.pop() {
            if let DockNode::Split { children, .. } = node {
                pending.extend(children.into_iter().map(|c| c.node));
            }
        }
        return None;
    }
    let mut id = node.id();
    let mut suffix = 0_u64;
    while !nodes.insert(id) {
        id = id.with(("repair", suffix));
        suffix += 1;
    }
    match node {
        DockNode::Tabs { panels, active, .. } => {
            let panels: Vec<_> = panels
                .into_iter()
                .filter(|p| panels_seen.insert(*p))
                .collect();
            if panels.is_empty() {
                return None;
            }
            let active = active
                .filter(|p| panels.contains(p))
                .or_else(|| panels.first().copied());
            Some(DockNode::Tabs { id, panels, active })
        }
        DockNode::Split { axis, children, .. } => {
            let mut flat = Vec::new();
            for c in children {
                let fraction = if c.fraction.is_finite() && c.fraction > 0.0 {
                    c.fraction
                } else {
                    1.0
                };
                let Some(node) = normalize(c.node, depth + 1, panels_seen, nodes) else {
                    continue;
                };
                match node {
                    DockNode::Split {
                        axis: child_axis,
                        children,
                        ..
                    } if axis == child_axis => {
                        for c in children {
                            flat.push(DockChild::new(c.node, fraction * c.fraction));
                        }
                    }
                    node => flat.push(DockChild::new(node, fraction)),
                }
            }
            match flat.len() {
                0 => None,
                1 => flat.pop().map(|c| c.node),
                _ => {
                    let sum: f64 = flat.iter().map(|c| f64::from(c.fraction)).sum();
                    for c in &mut flat {
                        c.fraction = ((f64::from(c.fraction) / sum) as f32).max(f32::MIN_POSITIVE);
                    }
                    Some(DockNode::Split {
                        id,
                        axis,
                        children: flat,
                    })
                }
            }
        }
    }
}
