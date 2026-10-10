use crate::{Id, Layout, Rect};
use std::fmt;

/// Identity derived from an application value, never from a panel's index or title.
pub type PanelId = Id;

#[derive(Clone, Debug, PartialEq)]
pub enum DockNode {
    Split {
        id: Id,
        axis: Layout,
        children: Vec<DockChild>,
    },
    Tabs {
        id: Id,
        panels: Vec<PanelId>,
        active: Option<PanelId>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DockChild {
    pub fraction: f32,
    pub node: DockNode,
}
impl DockChild {
    pub fn new(node: DockNode, fraction: f32) -> Self {
        Self { node, fraction }
    }
}
impl DockNode {
    pub fn tabs(source: impl std::hash::Hash, panels: impl IntoIterator<Item = PanelId>) -> Self {
        let panels: Vec<_> = panels.into_iter().collect();
        let active = panels.first().copied();
        Self::Tabs {
            id: Id::new(source),
            panels,
            active,
        }
    }
    pub fn split(
        source: impl std::hash::Hash,
        axis: Layout,
        children: impl IntoIterator<Item = DockChild>,
    ) -> Self {
        Self::Split {
            id: Id::new(source),
            axis,
            children: children.into_iter().collect(),
        }
    }
    pub fn id(&self) -> Id {
        match self {
            Self::Split { id, .. } | Self::Tabs { id, .. } => *id,
        }
    }
    pub fn contains(&self, panel: PanelId) -> bool {
        let mut nodes = vec![self];
        while let Some(node) = nodes.pop() {
            match node {
                Self::Tabs { panels, .. } if panels.contains(&panel) => return true,
                Self::Split { children, .. } => nodes.extend(children.iter().map(|c| &c.node)),
                _ => {}
            }
        }
        false
    }
    pub(super) fn group_mut(&mut self, panel: PanelId) -> Option<&mut Self> {
        self.group_mut_at(panel, 0)
    }
    fn group_mut_at(&mut self, panel: PanelId, depth: usize) -> Option<&mut Self> {
        if depth > 128 {
            return None;
        }
        match self {
            Self::Tabs { panels, .. } if panels.contains(&panel) => Some(self),
            Self::Split { children, .. } => children
                .iter_mut()
                .find_map(|c| c.node.group_mut_at(panel, depth + 1)),
            _ => None,
        }
    }
    pub(super) fn group_id_mut(&mut self, group: Id) -> Option<&mut Self> {
        self.group_id_mut_at(group, 0)
    }
    fn group_id_mut_at(&mut self, group: Id, depth: usize) -> Option<&mut Self> {
        if depth > 128 {
            return None;
        }
        if self.id() == group {
            return Some(self);
        }
        match self {
            Self::Split { children, .. } => children
                .iter_mut()
                .find_map(|c| c.node.group_id_mut_at(group, depth + 1)),
            _ => None,
        }
    }
    pub(super) fn collect(&self, out: &mut Vec<PanelId>) {
        let mut nodes = vec![self];
        while let Some(node) = nodes.pop() {
            match node {
                Self::Tabs { panels, .. } => out.extend(panels),
                Self::Split { children, .. } => {
                    nodes.extend(children.iter().rev().map(|c| &c.node))
                }
            }
        }
    }
}

/// Internal floating Window. Bounds and stacking order remain application data.
#[derive(Clone, Debug, PartialEq)]
pub struct DockFloat {
    pub node: DockNode,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockState {
    pub root: Option<DockNode>,
    /// Back to front.
    pub floats: Vec<DockFloat>,
    pub focused: Option<PanelId>,
}
impl DockState {
    pub fn new(root: DockNode) -> Self {
        let mut state = Self {
            root: Some(root),
            ..Self::default()
        };
        state.normalize();
        state
    }
    pub fn panels(&self) -> Vec<PanelId> {
        let mut ids = Vec::new();
        if let Some(root) = &self.root {
            root.collect(&mut ids);
        }
        for float in &self.floats {
            float.node.collect(&mut ids);
        }
        ids
    }
    pub fn contains(&self, panel: PanelId) -> bool {
        self.root.as_ref().is_some_and(|r| r.contains(panel))
            || self.floats.iter().any(|f| f.node.contains(panel))
    }
    pub(super) fn group_mut(&mut self, panel: PanelId) -> Option<&mut DockNode> {
        self.root
            .as_mut()
            .and_then(|r| r.group_mut(panel))
            .or_else(|| self.floats.iter_mut().find_map(|f| f.node.group_mut(panel)))
    }
    pub(super) fn group_id_mut(&mut self, id: Id) -> Option<&mut DockNode> {
        self.root
            .as_mut()
            .and_then(|r| r.group_id_mut(id))
            .or_else(|| self.floats.iter_mut().find_map(|f| f.node.group_id_mut(id)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DockIssue {
    DuplicatePanel(PanelId),
    DuplicateNode(Id),
    MissingPanel(PanelId),
    InvalidFraction(Id),
    InvalidBounds(Id),
    TooDeep,
    InvalidFormat,
}
/// Invalid application data or an operation referring to a missing panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockError(pub DockIssue);
impl fmt::Display for DockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid dock layout: {:?}", self.0)
    }
}
impl std::error::Error for DockError {}
