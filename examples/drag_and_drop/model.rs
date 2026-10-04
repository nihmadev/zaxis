//! Application-owned data. The library only reports moves; every mutation is here.
use std::collections::HashMap;
use zaxis::{Id, ImageSource, Insertion, TreeChildren, TreeModel, TreeNode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskDrag(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileDrag(pub usize);

pub struct Task {
    pub id: u32,
    pub title: String,
}

/// A move of one task into a list, before/after another task or at the end.
pub struct TaskMove {
    pub task: u32,
    pub list: &'static str,
    pub anchor: Option<(u32, Insertion)>,
}

pub const FILES: [&str; 3] = ["report.pdf", "notes.txt", "photo.png"];

pub struct Data {
    pub todo: Vec<Task>,
    pub done: Vec<Task>,
    pub archive: Vec<usize>,
    pub rows: Vec<u32>,
    pub objects: Objects,
    next_row: u32,
}

impl Data {
    pub fn new() -> Self {
        Self {
            todo: (1..=24)
                .map(|id| Task {
                    id,
                    title: format!("Task {id}"),
                })
                .collect(),
            done: Vec::new(),
            archive: Vec::new(),
            rows: (1000..1040).collect(),
            objects: Objects::new(),
            next_row: 2000,
        }
    }

    fn take(&mut self, task: u32) -> Option<Task> {
        for list in [&mut self.todo, &mut self.done] {
            if let Some(i) = list.iter().position(|t| t.id == task) {
                return Some(list.remove(i));
            }
        }
        None
    }

    pub fn apply(&mut self, m: TaskMove) {
        if m.anchor.is_some_and(|(anchor, _)| anchor == m.task) {
            return;
        }
        let Some(task) = self.take(m.task) else {
            return;
        };
        let list = if m.list == "todo" {
            &mut self.todo
        } else {
            &mut self.done
        };
        let at = m
            .anchor
            .and_then(|(anchor, pos)| {
                list.iter()
                    .position(|t| t.id == anchor)
                    .map(|i| i + usize::from(pos != Insertion::Before))
            })
            .unwrap_or(list.len());
        list.insert(at, task);
    }

    /// A task dropped on the table becomes a table row; it leaves its list.
    pub fn task_to_table(&mut self, task: u32) {
        if self.take(task).is_some() {
            self.rows.push(self.next_row);
            self.next_row += 1;
        }
    }

    pub fn move_row(&mut self, row: u32, target: u32, pos: Insertion) {
        if row == target {
            return;
        }
        let Some(from) = self.rows.iter().position(|r| *r == row) else {
            return;
        };
        self.rows.remove(from);
        if let Some(at) = self.rows.iter().position(|r| *r == target) {
            self.rows
                .insert(at + usize::from(pos != Insertion::Before), row);
        }
    }
}

pub fn id(n: u64) -> Id {
    Id::new(n)
}

pub struct Node {
    pub label: String,
    pub leaf: bool,
    pub parent: Option<Id>,
    pub children: Vec<Id>,
}

pub struct Objects {
    pub revision: u64,
    pub roots: Vec<Id>,
    pub nodes: HashMap<Id, Node>,
    pub icon: ImageSource,
}

impl Objects {
    fn new() -> Self {
        let mut o = Self {
            revision: 0,
            roots: vec![id(1), id(2)],
            nodes: HashMap::new(),
            icon: ImageSource::bytes(include_bytes!("../../assets/images/icon.svg")),
        };
        for (n, parent, label) in [
            (1, None, "Scene"),
            (11, Some(1), "Camera"),
            (12, Some(1), "Lights"),
            (121, Some(12), "Key light"),
            (122, Some(12), "Fill light"),
            (13, Some(1), "Meshes"),
            (131, Some(13), "Floor"),
            (132, Some(13), "Wall"),
            (2, None, "Assets"),
            (21, Some(2), "Textures"),
            (22, Some(2), "Materials"),
            (23, Some(2), "Sounds"),
        ] {
            o.nodes.insert(
                id(n),
                Node {
                    label: label.into(),
                    leaf: matches!(n, 11 | 121 | 122 | 131 | 132),
                    parent: parent.map(id),
                    children: Vec::new(),
                },
            );
            if let Some(p) = parent {
                o.nodes.get_mut(&id(p)).unwrap().children.push(id(n));
            }
        }
        o
    }

    fn is_within(&self, node: Id, ancestor: Id) -> bool {
        let mut current = Some(node);
        while let Some(n) = current {
            if n == ancestor {
                return true;
            }
            current = self.nodes.get(&n).and_then(|n| n.parent);
        }
        false
    }

    /// Apply `TreeEvent::Moved`. The tree already refused cycles; check anyway.
    pub fn move_node(&mut self, node: Id, target: Id, pos: Insertion) {
        if self.is_within(target, node) {
            return;
        }
        let old = self.nodes[&node].parent;
        match old {
            Some(p) => self
                .nodes
                .get_mut(&p)
                .unwrap()
                .children
                .retain(|c| *c != node),
            None => self.roots.retain(|r| *r != node),
        }
        let (parent, index) = match pos {
            Insertion::Inside => (Some(target), None),
            _ => {
                let parent = self.nodes[&target].parent;
                let siblings = match parent {
                    Some(p) => &self.nodes[&p].children,
                    None => &self.roots,
                };
                let i = siblings.iter().position(|c| *c == target).unwrap_or(0);
                (parent, Some(i + usize::from(pos == Insertion::After)))
            }
        };
        let list = match parent {
            Some(p) => &mut self.nodes.get_mut(&p).unwrap().children,
            None => &mut self.roots,
        };
        list.insert(index.unwrap_or(list.len()), node);
        self.nodes.get_mut(&node).unwrap().parent = parent;
        self.revision += 1;
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
                .children(if n.leaf {
                    TreeChildren::Leaf
                } else {
                    TreeChildren::Loaded
                })
        })
    }
    fn parent(&self, id: Id) -> Option<Id> {
        self.nodes.get(&id).and_then(|n| n.parent)
    }
}
