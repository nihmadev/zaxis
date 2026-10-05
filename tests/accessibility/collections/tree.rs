use super::*;
use std::collections::HashMap;

/// A model of numbered nodes: label, children, kind and whether it is enabled.
#[derive(Default)]
struct Model {
    nodes: HashMap<Id, (String, Vec<Id>, TreeChildren, bool)>,
    roots: Vec<Id>,
    parents: HashMap<Id, Id>,
}
fn id(n: u32) -> Id {
    Id::new(n)
}
impl Model {
    fn add(&mut self, n: u32, label: &str, parent: Option<u32>, kind: TreeChildren) {
        self.nodes
            .insert(id(n), (label.into(), Vec::new(), kind, true));
        match parent {
            Some(parent) => {
                self.nodes.get_mut(&id(parent)).unwrap().1.push(id(n));
                self.parents.insert(id(n), id(parent));
            }
            None => self.roots.push(id(n)),
        }
    }
    /// Documents { Letters { A, B }, Notes }, Pictures (empty branch), Readme.
    fn files() -> Self {
        let mut model = Self::default();
        let (branch, leaf) = (TreeChildren::Loaded, TreeChildren::Leaf);
        model.add(0, "Documents", None, branch);
        model.add(1, "Letters", Some(0), branch);
        model.add(2, "A", Some(1), leaf);
        model.add(3, "B", Some(1), leaf);
        model.add(4, "Notes", Some(0), leaf);
        model.add(5, "Pictures", None, branch);
        model.add(6, "Readme", None, leaf);
        model
    }
}
impl TreeModel for Model {
    fn revision(&self) -> u64 {
        0
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        self.roots.iter().copied()
    }
    fn children(&self, node: Id) -> impl Iterator<Item = Id> {
        self.nodes[&node].1.iter().copied()
    }
    fn node(&self, node: Id) -> Option<TreeNode<'_>> {
        let (label, _, kind, enabled) = self.nodes.get(&node)?;
        Some(TreeNode::leaf(label).children(*kind).enabled(*enabled))
    }
    fn parent(&self, node: Id) -> Option<Id> {
        self.parents.get(&node).copied()
    }
}

struct Tree {
    harness: Harness,
    model: Model,
    enabled: bool,
    events: Vec<TreeEvent>,
    built: usize,
    edits: Vec<Id>,
}
impl Tree {
    fn new(model: Model) -> Self {
        let mut tree = Self {
            harness: Harness::new(),
            model,
            enabled: true,
            events: Vec::new(),
            built: 0,
            edits: Vec::new(),
        };
        tree.pass();
        tree
    }
    fn pass(&mut self) -> Option<usize> {
        let (model, events, built, edits) = (
            &self.model,
            &mut self.events,
            &mut self.built,
            &mut self.edits,
        );
        let view = TreeView::new("files")
            .accessible_label("Files")
            .enabled(self.enabled)
            .max_height(240.0);
        self.harness.pass(|ctx| {
            Root::new().show(ctx, |ui| {
                let out = view.show_with_actions(ui, model, |ui, node| {
                    if node == id(4) && ui.button("Edit").clicked() {
                        edits.push(node);
                    }
                });
                events.extend(out.events.iter().copied());
                *built = out.rows_built;
            });
        })
    }
    fn settle(&mut self) {
        for _ in 0..4 {
            self.pass();
        }
    }
    fn root(&self) -> NodeId {
        self.harness.tree.expect(Role::Tree, "Files")
    }
    fn item(&self, name: &str) -> NodeId {
        self.harness.tree.expect(Role::TreeItem, name)
    }
    fn act(&mut self, name: &str, action: Action) -> bool {
        let item = self.item(name);
        let accepted = self.harness.act(item, action);
        self.settle();
        accepted
    }
    /// Names of the rows, in order, each with its level.
    fn rows(&self) -> Vec<(String, usize)> {
        let tree = &self.harness.tree;
        let rows = children(&self.harness, self.root(), Role::TreeItem);
        rows.iter()
            .map(|row| (tree.name(*row), tree.node(*row).level().unwrap()))
            .collect()
    }
}

fn named(rows: &[(&str, usize)]) -> Vec<(String, usize)> {
    rows.iter()
        .map(|(name, level)| ((*name).to_owned(), *level))
        .collect()
}

#[test]
fn a_tree_is_a_flat_run_of_items_with_levels_and_sets() {
    let mut tree = Tree::new(Model::files());
    assert_eq!(
        tree.rows(),
        named(&[("Documents", 1), ("Pictures", 1), ("Readme", 1)])
    );
    let root = tree.root();
    assert!(tree.harness.tree.all(Role::ScrollView).is_empty());
    assert!(tree.harness.tree.node(root).clips_children());
    let node = |tree: &Tree, name: &str| tree.harness.tree.node(tree.item(name)).clone();
    let documents = node(&tree, "Documents");
    assert_eq!(documents.is_expanded(), Some(false));
    assert_eq!(documents.is_selected(), Some(false));
    assert_eq!(
        (documents.position_in_set(), documents.size_of_set()),
        (Some(0), Some(3))
    );
    assert!(documents.supports_action(Action::Expand) && documents.supports_action(Action::Click));
    let readme = node(&tree, "Readme");
    assert_eq!(readme.is_expanded(), None, "a leaf does not expand");
    assert!(!readme.supports_action(Action::Expand));
    assert_eq!(
        (readme.position_in_set(), readme.size_of_set()),
        (Some(2), Some(3))
    );
    assert_eq!(tree.pass(), None, "an idle pass publishes nothing");

    assert!(tree.act("Documents", Action::Expand));
    assert!(tree.act("Letters", Action::Expand));
    assert_eq!(
        tree.rows(),
        named(&[
            ("Documents", 1),
            ("Letters", 2),
            ("A", 3),
            ("B", 3),
            ("Notes", 2),
            ("Pictures", 1),
            ("Readme", 1)
        ])
    );
    let b = node(&tree, "B");
    assert_eq!((b.position_in_set(), b.size_of_set()), (Some(1), Some(2)));
    let notes = node(&tree, "Notes");
    assert_eq!(
        (notes.position_in_set(), notes.size_of_set()),
        (Some(1), Some(2))
    );
    assert_eq!(node(&tree, "Documents").is_expanded(), Some(true));
    // A widget built into a row is inside that row's item.
    let edit = tree.harness.tree.expect(Role::Button, "Edit");
    assert_eq!(tree.harness.tree.parent(edit), Some(tree.item("Notes")));
    let (row, button) = (
        logical(&tree.harness, tree.item("Notes")),
        logical(&tree.harness, edit),
    );
    assert!(
        row.min.x <= button.min.x && row.max.x >= button.max.x,
        "{row:?} {button:?}"
    );
    assert_eq!(tree.pass(), None);
}

#[test]
fn expand_and_collapse_requests_are_idempotent_and_report_one_event() {
    let mut tree = Tree::new(Model::files());
    assert!(tree.act("Documents", Action::Expand));
    let opened = TreeEvent::OpenChanged {
        node: id(0),
        open: true,
    };
    assert_eq!(tree.events, [opened]);
    assert!(
        tree.act("Documents", Action::Expand),
        "accepted, and already open"
    );
    assert_eq!(tree.events, [opened]);
    assert_eq!(tree.rows().len(), 5);
    assert!(tree.act("Documents", Action::Collapse));
    assert!(tree.act("Documents", Action::Collapse));
    let closed = TreeEvent::OpenChanged {
        node: id(0),
        open: false,
    };
    assert_eq!(tree.events, [opened, closed]);
    assert_eq!(tree.rows().len(), 3);
    // The chevron does the same thing through the same events.
    let mut by_pointer = Tree::new(Model::files());
    let row = logical(&by_pointer.harness, by_pointer.item("Documents"));
    click(
        &mut by_pointer.harness.context,
        Vec2::new(row.min.x + 12.0, row.center().y),
    );
    by_pointer.settle();
    assert_eq!(by_pointer.events, [opened]);
    // Expanding does not select and a leaf refuses to expand.
    assert_eq!(
        tree.harness.tree.node(tree.item("Documents")).is_selected(),
        Some(false)
    );
    assert!(!tree.act("Readme", Action::Expand));
    assert_eq!(tree.events.len(), 2);
}

#[test]
fn a_click_request_selects_like_a_click_on_the_row() {
    let mut by_request = Tree::new(Model::files());
    assert!(by_request.act("Pictures", Action::Click));
    let selected = TreeEvent::Selected { node: Some(id(5)) };
    assert_eq!(by_request.events, [selected], "one request, one event");
    let row = by_request.item("Pictures");
    assert_eq!(by_request.harness.tree.node(row).is_selected(), Some(true));
    assert_eq!(
        by_request.harness.tree.node(row).is_expanded(),
        Some(false),
        "a click does not open"
    );
    assert!(by_request.act("Pictures", Action::Click));
    assert_eq!(
        by_request.events,
        [selected],
        "selecting the selected row reports nothing"
    );

    let mut by_pointer = Tree::new(Model::files());
    let row = logical(&by_pointer.harness, by_pointer.item("Pictures"));
    click(
        &mut by_pointer.harness.context,
        Vec2::new(row.max.x - 20.0, row.center().y),
    );
    by_pointer.settle();
    assert_eq!(by_pointer.events, by_request.events);
    for tree in [&by_request, &by_pointer] {
        let selected: Vec<_> = children(&tree.harness, tree.root(), Role::TreeItem)
            .into_iter()
            .filter(|row| tree.harness.tree.node(*row).is_selected() == Some(true))
            .map(|row| tree.harness.tree.name(row))
            .collect();
        assert_eq!(selected, ["Pictures"]);
        // Both leave the keyboard cursor on the row.
        let active = tree.harness.tree.node(tree.root()).active_descendant();
        assert_eq!(active, Some(tree.item("Pictures")));
    }
    assert_eq!(by_request.pass(), None);
}

#[test]
fn focus_is_the_tree_and_the_cursor_row_is_its_active_descendant() {
    let mut tree = Tree::new(Model::files());
    let root = tree.root();
    assert!(tree.harness.act(root, Action::Focus));
    tree.settle();
    assert_eq!(tree.harness.tree.focus(), root, "rows are not Tab stops");
    let active = |tree: &Tree| tree.harness.tree.node(tree.root()).active_descendant();
    assert_eq!(active(&tree), Some(tree.item("Documents")));
    press(&mut tree.harness.context, KeyCode::ArrowDown);
    tree.settle();
    assert_eq!(active(&tree), Some(tree.item("Pictures")));
    press(&mut tree.harness.context, KeyCode::ArrowRight);
    tree.settle();
    assert_eq!(
        tree.events,
        [TreeEvent::OpenChanged {
            node: id(5),
            open: true
        }]
    );
    assert_eq!(
        tree.harness.tree.node(tree.item("Pictures")).is_expanded(),
        Some(true)
    );
    assert_eq!(tree.harness.tree.focus(), root);
    assert_eq!(tree.pass(), None);
}

#[test]
fn disabled_rows_and_disabled_trees_refuse_requests() {
    let mut model = Model::files();
    model.nodes.get_mut(&id(5)).unwrap().3 = false;
    let mut tree = Tree::new(model);
    let row = tree.item("Pictures");
    assert!(tree.harness.tree.node(row).is_disabled());
    assert!(!tree.act("Pictures", Action::Click));
    assert!(!tree.act("Pictures", Action::Expand));
    assert!(tree.events.is_empty());

    tree.enabled = false;
    tree.settle();
    assert!(tree.harness.tree.node(tree.root()).is_disabled());
    assert!(!tree.act("Documents", Action::Expand));
    assert!(!tree.act("Documents", Action::Click));
    let root = tree.root();
    assert!(!tree.harness.act(root, Action::ScrollDown));
    tree.settle();
    assert!(tree.events.is_empty());
    assert_eq!(tree.rows().len(), 3);
}

#[test]
fn a_button_in_a_row_is_clicked_on_its_own() {
    let mut tree = Tree::new(Model::files());
    tree.act("Documents", Action::Expand);
    tree.events.clear();
    let edit = tree.harness.tree.expect(Role::Button, "Edit");
    assert!(tree.harness.act(edit, Action::Click));
    tree.settle();
    assert_eq!(tree.edits, [id(4)]);
    assert!(
        tree.events.is_empty(),
        "the row was not selected: {:?}",
        tree.events
    );
}

#[test]
fn a_large_tree_publishes_the_rows_it_built() {
    let mut model = Model::default();
    model.add(0, "Root", None, TreeChildren::Loaded);
    for n in 1..2000 {
        model.add(n, &format!("Node {n}"), Some(0), TreeChildren::Leaf);
    }
    let mut tree = Tree::new(model);
    tree.act("Root", Action::Expand);
    let root = tree.root();
    let rows = children(&tree.harness, root, Role::TreeItem);
    assert!(
        rows.len() < 20 && rows.len() == tree.built,
        "{} of {}",
        rows.len(),
        tree.built
    );
    assert!(tree.harness.context.accessibility_stats().nodes < 40);
    let first = tree.harness.tree.node(tree.item("Node 1")).clone();
    assert_eq!(
        (first.position_in_set(), first.size_of_set()),
        (Some(0), Some(1999))
    );
    assert_eq!(first.level(), Some(2));
    assert_eq!(tree.pass(), None);

    // Scrolling builds far rows; they say where they are in the whole set.
    let pitch = f64::from(logical(&tree.harness, tree.item("Node 1")).size().y);
    let far = ActionData::SetScrollOffset(accesskit::Point::new(0.0, 1500.0 * pitch));
    assert!(tree.harness.act_with(root, Action::SetScrollOffset, far));
    tree.settle();
    assert_eq!(
        tree.harness.tree.node(root).scroll_y(),
        Some(1500.0 * pitch)
    );
    let node = tree.harness.tree.node(tree.item("Node 1500")).clone();
    assert_eq!(
        (node.position_in_set(), node.size_of_set()),
        (Some(1499), Some(1999))
    );
    assert!(children(&tree.harness, root, Role::TreeItem).len() < 20);
    tree.events.clear();
    assert!(tree.act("Node 1500", Action::Click));
    assert_eq!(
        tree.events,
        [TreeEvent::Selected {
            node: Some(id(1500))
        }]
    );
    assert_eq!(tree.pass(), None);
}
