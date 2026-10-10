//! Single-value controls: switches, radio groups, tabs, disclosures, numbers, key and
//! color pickers, and the field that labels them.

mod dock;
mod field;
mod focus_group;
mod numbers;
mod pickers;
mod tabs;
mod toggles;

use crate::support::*;

/// Role and name of every child of `parent`, in order.
fn children(harness: &Harness, parent: NodeId) -> Vec<(Role, String)> {
    let tree = &harness.tree;
    tree.node(parent)
        .children()
        .iter()
        .map(|id| (tree.node(*id).role(), tree.name(*id)))
        .collect()
}

/// How many controls of the last pass a screen reader could not name.
fn nameless(harness: &Harness) -> usize {
    harness
        .context
        .diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::MissingAccessibleName)
        .count()
}

/// The names a platform adapter gives the nodes of `role`, in tree order. The consumer
/// follows a `labelled_by` relation one step, which is what a screen reader hears.
fn platform_names(harness: &Harness, role: Role) -> Vec<Option<String>> {
    fn walk(node: accesskit_consumer::NodeRef<'_>, role: Role, out: &mut Vec<Option<String>>) {
        if node.role() == role {
            out.push(node.label());
        }
        for child in node.children() {
            walk(child, role, out);
        }
    }
    let mut names = Vec::new();
    let consumer = harness.consumer.as_ref().expect("a pass was run");
    walk(consumer.state().root(), role, &mut names);
    names
}

/// Zero-based position and size of the set `id` belongs to.
fn set(harness: &Harness, id: NodeId) -> (Option<usize>, Option<usize>) {
    let node = harness.tree.node(id);
    (node.position_in_set(), node.size_of_set())
}
