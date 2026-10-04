use crate::{Id, ImageSource};

/// Unloaded/loading/error remain branches even with no known children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeChildren {
    Leaf,
    Unloaded,
    Loading,
    Loaded,
    Error,
}
impl TreeChildren {
    pub fn expandable(self) -> bool {
        self != Self::Leaf
    }
}
#[derive(Clone, Copy, Debug)]
pub struct TreeNode<'a> {
    pub label: &'a str,
    pub icon: Option<&'a ImageSource>,
    pub enabled: bool,
    pub children: TreeChildren,
}
impl<'a> TreeNode<'a> {
    pub fn leaf(label: &'a str) -> Self {
        Self {
            label,
            icon: None,
            enabled: true,
            children: TreeChildren::Leaf,
        }
    }
    pub fn branch(label: &'a str) -> Self {
        Self {
            children: TreeChildren::Loaded,
            ..Self::leaf(label)
        }
    }
    pub fn icon(mut self, icon: &'a ImageSource) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn children(mut self, children: TreeChildren) -> Self {
        self.children = children;
        self
    }
}
/// Application-owned model. Ids are globally unique within this tree.
/// `node` must look up hidden nodes too; None means deleted, never offscreen.
/// Increment revision for topology, enabled, loading, label or icon changes.
pub trait TreeModel {
    fn revision(&self) -> u64;
    fn roots(&self) -> impl Iterator<Item = Id>;
    fn children(&self, node: Id) -> impl Iterator<Item = Id>;
    fn node(&self, node: Id) -> Option<TreeNode<'_>>;
    /// Required for reveal and optimal hidden-focus repair. Must be direct lookup.
    fn parent(&self, _node: Id) -> Option<Id> {
        None
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeEvent {
    OpenChanged {
        node: Id,
        open: bool,
    },
    Selected {
        node: Option<Id>,
    },
    Activated {
        node: Id,
    },
    ContextAction {
        node: Id,
    },
    RequestChildren {
        node: Id,
    },
    /// A drag finished: move `node` relative to `target`. The application edits
    /// its model and bumps the revision.
    Moved {
        node: Id,
        target: Id,
        position: crate::Insertion,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeIssue {
    DuplicateOrCycle(Id),
    MissingNode(Id),
    InvalidRevealPath(Id),
}
