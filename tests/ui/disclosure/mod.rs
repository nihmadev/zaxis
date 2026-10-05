use crate::prelude::*;
use std::cell::Cell;
use winit::{dpi::PhysicalSize, event::ElementState};
use zaxis::{CollapsingHeader, Rect, Root, TreeChildren, TreeEvent, TreeModel, TreeNode, TreeView};
mod collapsing;
mod more;
mod navigation;
mod tree;

fn id(n: u64) -> Id {
    Id::new(n)
}
struct Model {
    revision: u64,
    roots: Vec<Id>,
    nodes: HashMap<Id, (Option<Id>, Vec<Id>, bool, TreeChildren)>,
    visits: Cell<usize>,
}
impl Model {
    fn new(count: usize) -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(
            id(0),
            (
                None,
                (1..=count as u64).map(id).collect(),
                true,
                TreeChildren::Loaded,
            ),
        );
        for n in 1..=count as u64 {
            nodes.insert(id(n), (Some(id(0)), Vec::new(), n != 2, TreeChildren::Leaf));
        }
        Self {
            revision: 0,
            roots: vec![id(0)],
            nodes,
            visits: Cell::new(0),
        }
    }
}
impl TreeModel for Model {
    fn revision(&self) -> u64 {
        self.revision
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        self.roots.iter().copied()
    }
    fn children(&self, n: Id) -> impl Iterator<Item = Id> {
        self.visits.set(self.visits.get() + 1);
        self.nodes
            .get(&n)
            .into_iter()
            .flat_map(|n| n.1.iter().copied())
    }
    fn node(&self, n: Id) -> Option<TreeNode<'_>> {
        self.nodes
            .get(&n)
            .map(|n| TreeNode::leaf("Same caption").enabled(n.2).children(n.3))
    }
    fn parent(&self, n: Id) -> Option<Id> {
        self.nodes.get(&n).and_then(|n| n.0)
    }
}
fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(700, 600), 1.0);
    let mut s = c.style().clone();
    s.motion.reduced_motion = true;
    c.set_style(s);
    c
}
fn key(c: &mut Context, key: KeyCode) {
    assert!(c.on_key_event(key, ElementState::Pressed, false).consumed);
    c.on_key_event(key, ElementState::Released, false);
}
fn click(c: &mut Context, rect: Rect) {
    c.move_pointer(rect.center());
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}
fn tree<'a>(
    c: &mut Context,
    model: &Model,
    build: impl FnOnce(TreeView<'a>) -> TreeView<'a>,
) -> zaxis::TreeOutput {
    let mut out = None;
    c.run(|c| {
        Root::new().show(c, |ui| {
            out = Some(build(TreeView::new("tree").max_height(120.0)).show(ui, model));
        });
    });
    out.unwrap()
}
fn hit(c: &Context, node: Id, chevron: bool) -> Rect {
    c.probe().previous_hits.iter().find(|h| matches!(h.action, HitAction::TreeRow { node: n, chevron: ch, .. } if n==node && ch==chevron)).unwrap().rect
}

fn click_node(c: &mut Context, node: Id, chevron: bool) {
    let rect = hit(c, node, chevron);
    click(c, rect);
}
