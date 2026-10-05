//! Popups, menus, dialogs, tooltips, toasts and the custom title bar.
use crate::support::*;
use accesskit_consumer::{NodeRef, Tree, TreeChangeHandler};
use zaxis::accesskit::Live;

mod combo;
mod dialogs;
mod hints;
mod menu_bar;
mod menus;
mod modals;
mod popup;

/// A harness whose UI does not animate: a request shows its whole effect in `settle`.
fn still() -> Harness {
    let mut harness = Harness::new();
    let mut style = harness.context.style().clone();
    style.motion.reduced_motion = true;
    harness.context.set_style(style);
    harness
}

/// Whether `node` is `ancestor` or inside it.
fn inside(tree: &AccessTree, node: NodeId, ancestor: NodeId) -> bool {
    let mut at = Some(node);
    while let Some(id) = at {
        if id == ancestor {
            return true;
        }
        at = tree.parent(id);
    }
    false
}

/// Nodes of `role` in the order a screen reader walks them, by name.
fn names(tree: &AccessTree, role: Role) -> Vec<String> {
    tree.all(role).into_iter().map(|id| tree.name(id)).collect()
}

/// Middle of a node in logical pixels (the harness runs at scale 1).
fn middle(node: &Node) -> Vec2 {
    let bounds = node.bounds().expect("the node has bounds");
    vec2(
        ((bounds.x0 + bounds.x1) * 0.5) as f32,
        ((bounds.y0 + bounds.y1) * 0.5) as f32,
    )
}

fn press(context: &mut Context, at: Vec2) {
    context.move_pointer(at);
    context.primary_button(ElementState::Pressed);
    context.primary_button(ElementState::Released);
}

fn key(context: &mut Context, code: KeyCode) {
    context.key(code, ElementState::Pressed, false);
    context.key(code, ElementState::Released, false);
}

/// What a platform adapter would announce: live regions that appeared or were renamed.
#[derive(Default)]
struct Spoken(Vec<String>);

impl TreeChangeHandler for Spoken {
    fn node_added(&mut self, node: &NodeRef) {
        if node.live() != Live::Off {
            self.0.extend(node.label());
        }
    }
    fn node_updated(&mut self, old: &NodeRef, new: &NodeRef) {
        if new.live() != Live::Off && (old.live() != new.live() || old.label() != new.label()) {
            self.0.extend(new.label());
        }
    }
    fn focus_moved(&mut self, _: Option<&NodeRef>, _: Option<&NodeRef>) {}
    fn node_removed(&mut self, _: &NodeRef) {}
}

/// [`Harness::pass`] at a chosen time, recording what would be announced.
fn pass_at(
    harness: &mut Harness,
    now: Instant,
    spoken: &mut Spoken,
    build: impl FnOnce(&mut Context),
) -> Option<usize> {
    harness.context.run_at(now, build);
    let count = harness.tree.sync(&mut harness.context)?;
    let update = harness.tree.last.clone().expect("an update was applied");
    match &mut harness.consumer {
        Some(consumer) => consumer.update_and_process_changes(update, spoken),
        None => harness.consumer = Some(Tree::new(update, true)),
    }
    harness.tree.validate();
    Some(count)
}
