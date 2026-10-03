use std::collections::HashMap;
use zaxis::{Id, ImageSource, TreeChildren, TreeModel, TreeNode};
pub fn id(n: u64) -> Id {
    Id::new(n)
}
pub struct Object {
    pub label: String,
    pub parent: Option<Id>,
    pub children: Vec<Id>,
    pub kind: TreeChildren,
    pub enabled: bool,
}
pub struct Objects {
    pub revision: u64,
    pub roots: Vec<Id>,
    pub nodes: HashMap<Id, Object>,
    pub icon: ImageSource,
    pub next: u64,
}
impl Objects {
    pub fn new() -> Self {
        let mut model = Self {
            revision: 0,
            roots: vec![id(0), id(100)],
            nodes: HashMap::new(),
            icon: ImageSource::bytes(include_bytes!("../../assets/images/icon.svg")),
            next: 50_000,
        };
        model.insert(0, None, "Workspace", TreeChildren::Loaded);
        model.insert(1, Some(id(0)), "Objects", TreeChildren::Loaded);
        model.insert(2, Some(id(0)), "Remote", TreeChildren::Unloaded);
        model.insert(3, Some(id(0)), "Settings", TreeChildren::Leaf);
        model.insert(4, Some(id(1)), "Object", TreeChildren::Leaf);
        model.insert(5, Some(id(1)), "Object", TreeChildren::Leaf);
        model.insert(6, Some(id(1)), "Unavailable", TreeChildren::Loaded);
        model.nodes.get_mut(&id(6)).unwrap().enabled = false;
        model.insert(7, Some(id(6)), "Available child", TreeChildren::Leaf);
        model.insert(100, None, "10,000 objects", TreeChildren::Loaded);
        for n in 1000..11000 {
            model.insert(
                n,
                Some(id(100)),
                &format!("Object {}", n - 999),
                TreeChildren::Leaf,
            );
        }
        model
    }
    fn insert(&mut self, n: u64, parent: Option<Id>, label: &str, kind: TreeChildren) {
        self.nodes.insert(
            id(n),
            Object {
                label: label.into(),
                parent,
                children: Vec::new(),
                kind,
                enabled: true,
            },
        );
        if let Some(p) = parent {
            self.nodes.get_mut(&p).unwrap().children.push(id(n));
        }
    }
    pub fn add(&mut self) {
        let n = self.next;
        self.next += 1;
        self.insert(n, Some(id(1)), "Object", TreeChildren::Leaf);
        self.revision += 1;
    }
    pub fn remove(&mut self, selected: Id) {
        if self
            .nodes
            .get(&selected)
            .is_some_and(|n| n.kind == TreeChildren::Leaf)
        {
            let node = self.nodes.remove(&selected).unwrap();
            if let Some(p) = node.parent {
                self.nodes
                    .get_mut(&p)
                    .unwrap()
                    .children
                    .retain(|id| *id != selected);
            }
            self.revision += 1;
        }
    }
    pub fn load(&mut self, branch: Id) {
        if let Some(n) = self.nodes.get_mut(&branch) {
            n.kind = TreeChildren::Loading;
            self.revision += 1;
        }
    }
    pub fn finish_loading(&mut self) {
        if self.nodes[&id(2)].kind == TreeChildren::Loading {
            let n = self.next;
            self.next += 1;
            self.insert(n, Some(id(2)), "Remote object", TreeChildren::Leaf);
            self.nodes.get_mut(&id(2)).unwrap().kind = TreeChildren::Loaded;
            self.revision += 1;
        }
    }
}
impl TreeModel for Objects {
    fn revision(&self) -> u64 {
        self.revision
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        self.roots.iter().copied()
    }
    fn children(&self, id: Id) -> impl Iterator<Item = Id> {
        self.nodes
            .get(&id)
            .into_iter()
            .flat_map(|n| n.children.iter().copied())
    }
    fn node(&self, id: Id) -> Option<TreeNode<'_>> {
        self.nodes.get(&id).map(|n| {
            TreeNode::leaf(&n.label)
                .icon(&self.icon)
                .enabled(n.enabled)
                .children(n.kind)
        })
    }
    fn parent(&self, id: Id) -> Option<Id> {
        self.nodes.get(&id).and_then(|n| n.parent)
    }
}
