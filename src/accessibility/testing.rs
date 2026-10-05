//! The accessibility tree as assistive technology would hold it, for tests: no window, no
//! GPU and no screen reader. [`AccessTree`] mirrors the updates of a [`Context`], finds
//! nodes by role and name, and sends requests the way an adapter does.

use crate::Context;
use accesskit::{Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId, TreeUpdate};
use std::collections::{HashMap, HashSet};

/// A mirror of one context's accessibility tree, built only from the updates it publishes.
#[derive(Default)]
pub struct AccessTree {
    nodes: HashMap<NodeId, Node>,
    root: Option<NodeId>,
    focus: Option<NodeId>,
    /// The last update that was applied, as it would have gone to the platform adapter.
    pub last: Option<TreeUpdate>,
}

impl AccessTree {
    /// Turn collection on, as a screen reader asking for the tree does. The next pass
    /// publishes the whole tree.
    pub fn attach(context: &mut Context) -> Self {
        context.set_accessibility_active(true);
        Self::default()
    }

    /// Apply the update of the last pass. Returns how many nodes it carried, or `None`
    /// when the pass changed nothing.
    pub fn sync(&mut self, context: &mut Context) -> Option<usize> {
        let update = context.take_accessibility_update()?;
        let count = update.nodes.len();
        self.apply(update);
        Some(count)
    }

    /// Apply `update`, then drop nodes no longer reachable from the root, as a consumer of
    /// the tree does.
    pub fn apply(&mut self, update: TreeUpdate) {
        if let Some(tree) = &update.tree {
            self.root = Some(tree.root);
        }
        for (id, node) in &update.nodes {
            self.nodes.insert(*id, node.clone());
        }
        self.focus = Some(update.focus);
        let mut reachable = HashSet::new();
        let mut open: Vec<NodeId> = self.root.into_iter().collect();
        while let Some(id) = open.pop() {
            if reachable.insert(id) {
                if let Some(node) = self.nodes.get(&id) {
                    open.extend(node.children());
                }
            }
        }
        self.nodes.retain(|id, _| reachable.contains(id));
        self.last = Some(update);
    }

    pub fn root(&self) -> NodeId {
        self.root.expect("no tree yet: run a pass and sync")
    }
    pub fn focus(&self) -> NodeId {
        self.focus.expect("no tree yet: run a pass and sync")
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }
    pub fn node(&self, id: NodeId) -> &Node {
        self.get(id)
            .unwrap_or_else(|| panic!("node {id:?} is not in the tree"))
    }

    /// What a screen reader calls the node: its label, the text of a label node, or the
    /// own names of the nodes that label it (one step, as platform adapters resolve it).
    pub fn name(&self, id: NodeId) -> String {
        let node = self.node(id);
        match self.own_name(node) {
            Some(name) => name,
            None => node
                .labelled_by()
                .iter()
                .filter_map(|id| self.own_name(self.get(*id)?))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    fn own_name(&self, node: &Node) -> Option<String> {
        match node.label() {
            Some(label) => Some(label.to_owned()),
            None if node.role() == Role::Label => node.value().map(str::to_owned),
            None => None,
        }
    }

    /// Nodes in tree order (depth first, children in their order), text runs included.
    pub fn ids(&self) -> Vec<NodeId> {
        let mut order = Vec::new();
        let mut open: Vec<NodeId> = self.root.into_iter().collect();
        while let Some(id) = open.pop() {
            order.push(id);
            if let Some(node) = self.nodes.get(&id) {
                open.extend(node.children().iter().rev());
            }
        }
        order
    }

    /// Every node of `role`, in tree order.
    pub fn all(&self, role: Role) -> Vec<NodeId> {
        let mut found = self.ids();
        found.retain(|id| self.node(*id).role() == role);
        found
    }

    /// The first node of `role` named `name`, in tree order.
    pub fn find(&self, role: Role, name: &str) -> Option<NodeId> {
        self.all(role).into_iter().find(|id| self.name(*id) == name)
    }

    /// [`Self::find`] that panics with the names that do exist.
    #[track_caller]
    pub fn expect(&self, role: Role, name: &str) -> NodeId {
        self.find(role, name).unwrap_or_else(|| {
            let names: Vec<_> = self.all(role).iter().map(|id| self.name(*id)).collect();
            panic!("no {role:?} named {name:?}; {role:?} nodes: {names:?}")
        })
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes
            .iter()
            .find(|(_, node)| node.children().contains(&id))
            .map(|(parent, _)| *parent)
    }

    /// Panic unless the tree is well formed: every child exists and has one parent, the
    /// focus is a node of the tree, and so is every node a relation points to.
    #[track_caller]
    pub fn validate(&self) {
        let root = self.root();
        assert!(self.nodes.contains_key(&root), "the root is missing");
        assert!(
            self.nodes.contains_key(&self.focus()),
            "focus {:?} is not a node of the tree",
            self.focus()
        );
        let mut parents: HashMap<NodeId, NodeId> = HashMap::new();
        for (id, node) in &self.nodes {
            for child in node.children() {
                assert!(
                    self.nodes.contains_key(child),
                    "{id:?} lists missing child {child:?}"
                );
                assert_ne!(*child, root, "the root is listed as a child of {id:?}");
                if let Some(other) = parents.insert(*child, *id) {
                    panic!("{child:?} is a child of both {other:?} and {id:?}");
                }
            }
            let relations = node
                .labelled_by()
                .iter()
                .chain(node.described_by())
                .chain(node.controls())
                .copied()
                .chain(node.error_message())
                .chain(node.active_descendant());
            for target in relations {
                assert!(
                    self.nodes.contains_key(&target),
                    "{id:?} refers to missing {target:?}"
                );
            }
        }
        assert_eq!(
            parents.len() + 1,
            self.nodes.len(),
            "a node is not reachable"
        );
    }

    /// Send a request as assistive technology would. Returns whether it was accepted.
    pub fn act(
        &self,
        context: &mut Context,
        target: NodeId,
        action: Action,
        data: Option<ActionData>,
    ) -> bool {
        context
            .on_accessibility_action(&ActionRequest {
                action,
                target_tree: TreeId::ROOT,
                target_node: target,
                data,
            })
            .consumed
    }
}
