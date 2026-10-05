use std::collections::{HashMap, HashSet};
use zaxis::{Id, ImageSource, TreeChildren, TreeModel, TreeNode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Profile,
    Data,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Plan {
    Free,
    Pro,
    Team,
}

pub struct Member {
    pub id: u32,
    pub name: &'static str,
    pub role: &'static str,
    pub active: bool,
}

struct Folder {
    label: &'static str,
    parent: Option<Id>,
    children: Vec<Id>,
}

/// A small project tree: two folders with files.
pub struct Projects {
    roots: Vec<Id>,
    nodes: HashMap<Id, Folder>,
}

impl Projects {
    fn new() -> Self {
        let mut projects = Self {
            roots: Vec::new(),
            nodes: HashMap::new(),
        };
        for (key, parent, label) in [
            ("src", None, "src"),
            ("main", Some("src"), "main.rs"),
            ("lib", Some("src"), "lib.rs"),
            ("docs", None, "docs"),
            ("guide", Some("docs"), "guide.md"),
        ] {
            let (id, parent) = (Id::new(key), parent.map(Id::new));
            projects.nodes.insert(
                id,
                Folder {
                    label,
                    parent,
                    children: Vec::new(),
                },
            );
            match parent {
                Some(parent) => projects.nodes.get_mut(&parent).unwrap().children.push(id),
                None => projects.roots.push(id),
            }
        }
        projects
    }
}

impl TreeModel for Projects {
    fn revision(&self) -> u64 {
        0
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        self.roots.iter().copied()
    }
    fn children(&self, node: Id) -> impl Iterator<Item = Id> {
        self.nodes
            .get(&node)
            .into_iter()
            .flat_map(|folder| folder.children.iter().copied())
    }
    fn node(&self, node: Id) -> Option<TreeNode<'_>> {
        let folder = self.nodes.get(&node)?;
        Some(if folder.children.is_empty() {
            TreeNode::leaf(folder.label)
        } else {
            TreeNode::branch(folder.label).children(TreeChildren::Loaded)
        })
    }
    fn parent(&self, node: Id) -> Option<Id> {
        self.nodes.get(&node)?.parent
    }
}

pub struct Model {
    pub page: Page,
    pub name: String,
    pub email: String,
    pub notes: String,
    pub subscribe: bool,
    pub plan: Plan,
    pub volume: f32,
    pub region: Option<&'static str>,
    pub files: Vec<String>,
    pub selected_files: HashSet<Id>,
    pub projects: Projects,
    pub open_folders: HashSet<Id>,
    pub selected_node: Option<Id>,
    pub members: Vec<Member>,
    pub autosave: bool,
    pub about: bool,
    pub saved: u32,
    pub refreshed: u32,
    pub logo: ImageSource,
}

impl Model {
    pub fn new() -> Self {
        let member = |id, name, role, active| Member {
            id,
            name,
            role,
            active,
        };
        Self {
            page: Page::Profile,
            name: "Ada Lovelace".into(),
            email: "ada@example.com".into(),
            notes: "Reads proofs on Mondays.\nPrefers plain text.".into(),
            subscribe: true,
            plan: Plan::Pro,
            volume: 40.0,
            region: Some("europe"),
            files: [
                "notes.txt",
                "report.pdf",
                "photo.jpg",
                "budget.xlsx",
                "todo.md",
            ]
            .map(String::from)
            .to_vec(),
            selected_files: HashSet::new(),
            projects: Projects::new(),
            open_folders: HashSet::from([Id::new("src")]),
            selected_node: None,
            members: vec![
                member(1, "Ada Lovelace", "Owner", true),
                member(2, "Alan Turing", "Editor", true),
                member(3, "Grace Hopper", "Viewer", false),
            ],
            autosave: true,
            about: false,
            saved: 0,
            refreshed: 0,
            logo: ImageSource::bytes(include_bytes!("../../assets/images/icon.svg")),
        }
    }

    /// The message under the email field, when the address cannot be one.
    pub fn email_problem(&self) -> Option<&'static str> {
        let valid = self
            .email
            .split_once('@')
            .is_some_and(|(name, host)| !name.is_empty() && host.contains('.'));
        (!valid).then_some("Enter an address like name@example.com")
    }
}
