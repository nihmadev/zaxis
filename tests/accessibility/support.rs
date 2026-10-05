//! A context with a mirror of its accessibility tree and a consumer that checks every update.
#![allow(dead_code)]

pub use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};
pub use zaxis::accesskit::{
    self, Action, ActionData, Node, NodeId, Role, TextPosition, TextSelection, Toggled,
};
pub use zaxis::testing::*;
pub use zaxis::*;

use accesskit_consumer::{NodeRef, Tree, TreeChangeHandler};

struct Ignore;
impl TreeChangeHandler for Ignore {
    fn node_added(&mut self, _: &NodeRef) {}
    fn node_updated(&mut self, _: &NodeRef, _: &NodeRef) {}
    fn focus_moved(&mut self, _: Option<&NodeRef>, _: Option<&NodeRef>) {}
    fn node_removed(&mut self, _: &NodeRef) {}
}

/// One window with assistive technology attached.
pub struct Harness {
    pub context: Context,
    pub tree: AccessTree,
    pub consumer: Option<Tree>,
}

impl Harness {
    pub fn new() -> Self {
        Self::with_scale(1.0)
    }

    pub fn with_scale(scale: f64) -> Self {
        let mut context = Context::new();
        let size = PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32);
        context.set_viewport(size, scale);
        let tree = AccessTree::attach(&mut context);
        Self {
            context,
            tree,
            consumer: None,
        }
    }

    /// Run one pass and apply its update. Returns the number of nodes the update carried,
    /// `None` when the pass published nothing.
    pub fn pass<R>(&mut self, build: impl FnOnce(&mut Context) -> R) -> Option<usize> {
        self.context.run(|context| drop(build(context)));
        let count = self.tree.sync(&mut self.context)?;
        let update = self.tree.last.clone().expect("an update was applied");
        match &mut self.consumer {
            Some(consumer) => consumer.update_and_process_changes(update, &mut Ignore),
            None => self.consumer = Some(Tree::new(update, true)),
        }
        self.tree.validate();
        Some(count)
    }

    /// Passes until the UI settles: a request needs one pass to apply and often another
    /// for widgets built before the change to show it.
    pub fn settle<R>(&mut self, mut build: impl FnMut(&mut Context) -> R) {
        for _ in 0..4 {
            self.pass(&mut build);
        }
    }

    pub fn act(&mut self, target: NodeId, action: Action) -> bool {
        self.tree.act(&mut self.context, target, action, None)
    }

    pub fn act_with(&mut self, target: NodeId, action: Action, data: ActionData) -> bool {
        self.tree.act(&mut self.context, target, action, Some(data))
    }

    pub fn node(&self, role: Role, name: &str) -> &Node {
        self.tree.node(self.tree.expect(role, name))
    }

    /// The node as platform adapters see it, for the queries they run (text ranges,
    /// names, filters).
    pub fn consumer_node(&self, id: NodeId) -> NodeRef<'_> {
        self.consumer
            .as_ref()
            .expect("a pass has run")
            .state()
            .node_by_tree_local_id(id, accesskit::TreeId::ROOT)
            .expect("the node is in the consumer tree")
    }
}
