//! Scrolling and collections: scroll areas, lists, trees, tables, split panes, carousels and
//! dragging. Each widget is driven both by requests from assistive technology and by the
//! input they stand for, and the two must leave the same state behind.

mod carousel;
mod drag;
mod list;
mod scale;
mod scroll;
mod split;
mod table;
mod tree;

use crate::support::*;

/// Logical bounds of a node, from its physical ones.
pub(crate) fn logical(harness: &Harness, id: NodeId) -> Rect {
    let scale = harness.context.scale_factor() as f64;
    let b = harness.tree.node(id).bounds().expect("bounds");
    Rect::from_min_max(
        Vec2::new((b.x0 / scale) as f32, (b.y0 / scale) as f32),
        Vec2::new((b.x1 / scale) as f32, (b.y1 / scale) as f32),
    )
}

/// Children of `parent` with `role`, in order.
pub(crate) fn children(harness: &Harness, parent: NodeId, role: Role) -> Vec<NodeId> {
    let tree = &harness.tree;
    let all = tree.node(parent).children().iter().copied();
    all.filter(|id| tree.node(*id).role() == role).collect()
}

/// Whether `id` or one of its ancestors is hidden from assistive technology.
pub(crate) fn hidden(harness: &Harness, id: NodeId) -> bool {
    let mut current = Some(id);
    while let Some(id) = current {
        if harness.tree.node(id).is_hidden() {
            return true;
        }
        current = harness.tree.parent(id);
    }
    false
}

/// A press and release of the primary button at `at`.
pub(crate) fn click(context: &mut Context, at: Vec2) {
    context.move_pointer(at);
    context.primary_button(ElementState::Pressed);
    context.primary_button(ElementState::Released);
}

pub(crate) fn press(context: &mut Context, key: KeyCode) {
    context.key(key, ElementState::Pressed, false);
    context.key(key, ElementState::Released, false);
}
