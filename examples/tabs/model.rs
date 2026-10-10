//! The workspace behind the IDE: files, two editor groups, the bottom panel, and how the
//! events of the tab strips change them. The strips never edit any of it themselves.
use zaxis::{icons, icons::Icon, tab_after_close, Id, TabMove, Vec2};

pub struct File {
    pub id: u32,
    pub name: String,
    pub text: String,
    saved: String,
    /// Pinned files show as an icon alone and cannot be closed.
    pub pinned: bool,
    /// Generated files are listed but cannot be opened.
    pub locked: bool,
}

impl File {
    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }
    pub fn key(&self) -> Id {
        Id::new(self.id)
    }
    pub fn icon(&self) -> &'static Icon {
        match self.name.rsplit('.').next().unwrap_or("") {
            "rs" => &icons::FILE_CODE,
            "toml" => &icons::FILE_COG,
            "md" => &icons::FILE_TEXT,
            "svg" | "png" => &icons::FILE_IMAGE,
            "lock" => &icons::LOCK,
            "gitignore" => &icons::GIT_BRANCH,
            _ => &icons::FILE,
        }
    }
}

pub struct Group {
    pub name: &'static str,
    pub tabs: Vec<u32>,
    pub active: u32,
}

impl Group {
    pub fn bar(&self) -> Id {
        Id::new(self.name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Panel {
    Terminal,
    Problems,
    Output,
}

pub struct Workspace {
    pub files: Vec<File>,
    pub groups: [Group; 2],
    /// The group that gets files opened from the explorer.
    pub focus: usize,
    pub panel: Panel,
    pub terminal: Vec<String>,
    pub command: String,
    pub log: Vec<String>,
    /// Files torn off into windows of their own, with where they were released.
    pub floating: Vec<(u32, Vec2)>,
    next: u32,
}

fn sample(name: &str) -> String {
    let stem = name.split('.').next().unwrap_or(name);
    let mut text = format!("// {name}\n\n");
    for i in 0..24 {
        text.push_str(&format!(
            "fn {stem}_{i}(input: &str) -> usize {{\n    input.len() + {i}\n}}\n\n"
        ));
    }
    text
}

impl Workspace {
    pub fn new() -> Self {
        let mut files = Vec::new();
        let mut add = |name: &str, pinned: bool, locked: bool| {
            let id = files.len() as u32 + 1;
            let text = sample(name);
            files.push(File {
                id,
                name: name.to_owned(),
                saved: text.clone(),
                text,
                pinned,
                locked,
            });
            id
        };
        let left = vec![
            add("Cargo.toml", true, false),
            add("main.rs", false, false),
            add("app.rs", false, false),
            add("editor.rs", false, false),
            add(
                "a_considerably_longer_name_for_the_renderer_backend.rs",
                false,
                false,
            ),
            add("Cargo.lock", false, true),
        ];
        let mut right = vec![
            add("README.md", false, false),
            add("tabs.rs", false, false),
            add("theme.rs", false, false),
            add("logo.svg", false, false),
        ];
        for i in 1..=10 {
            right.push(add(&format!("case_{i:02}.rs"), false, false));
        }
        add(".gitignore", false, false);
        add("layout.rs", false, false);
        add("input.rs", false, false);
        let (active_left, active_right) = (left[1], right[1]);
        let mut work = Self {
            files,
            groups: [
                Group {
                    name: "left",
                    tabs: left,
                    active: active_left,
                },
                Group {
                    name: "right",
                    tabs: right,
                    active: active_right,
                },
            ],
            focus: 0,
            panel: Panel::Terminal,
            terminal: vec!["Type help".to_owned()],
            command: String::new(),
            log: Vec::new(),
            floating: Vec::new(),
            next: 0,
        };
        work.next = work.files.len() as u32 + 1;
        work.files[1].text.push_str("// edited\n");
        work
    }

    pub fn file(&self, id: u32) -> Option<&File> {
        self.files.iter().find(|f| f.id == id)
    }
    pub fn file_mut(&mut self, id: u32) -> Option<&mut File> {
        self.files.iter_mut().find(|f| f.id == id)
    }
    pub fn by_key(&self, key: Id) -> Option<u32> {
        self.files.iter().find(|f| f.key() == key).map(|f| f.id)
    }
    pub fn group_of(&self, id: u32) -> Option<usize> {
        self.groups.iter().position(|g| g.tabs.contains(&id))
    }
    pub fn note(&mut self, text: String) {
        self.log.push(text);
        if self.log.len() > 200 {
            self.log.remove(0);
        }
    }

    /// Open `id` in the focused group, or show it where it already is.
    pub fn open(&mut self, id: u32) {
        if self.file(id).is_none_or(|f| f.locked) {
            return;
        }
        if let Some(at) = self.floating.iter().position(|(f, _)| *f == id) {
            self.floating.remove(at);
        }
        let at = match self.group_of(id) {
            Some(at) => at,
            None => {
                self.groups[self.focus].tabs.push(id);
                self.focus
            }
        };
        self.groups[at].active = id;
        self.focus = at;
    }

    pub fn create(&mut self, group: usize) {
        let id = self.next;
        self.next += 1;
        let name = format!("untitled_{}.rs", id);
        self.files.push(File {
            id,
            name: name.clone(),
            text: String::new(),
            saved: String::new(),
            pinned: false,
            locked: false,
        });
        self.focus = group;
        self.groups[group].tabs.push(id);
        self.groups[group].active = id;
        self.note(format!("Created {name}"));
    }

    /// Close the tab; the one the rule picks takes its place as the active one.
    pub fn close(&mut self, group: usize, key: Id) {
        let Some(id) = self.by_key(key) else { return };
        let g = &mut self.groups[group];
        let Some(at) = g.tabs.iter().position(|t| *t == id) else {
            return;
        };
        if g.active == id {
            let enabled: Vec<bool> = g
                .tabs
                .iter()
                .map(|t| !self.files.iter().any(|f| f.id == *t && f.locked))
                .collect();
            if let Some(next) = tab_after_close(at, &enabled) {
                g.active = g.tabs[next];
            }
        }
        g.tabs.remove(at);
        let name = self.file(id).map(|f| f.name.clone()).unwrap_or_default();
        self.note(format!("Closed {name}"));
    }

    pub fn apply(&mut self, mv: TabMove) {
        let from = self.groups.iter().position(|g| g.bar() == mv.from_bar);
        let to = self.groups.iter().position(|g| g.bar() == mv.to_bar);
        let (Some(from), Some(to), Some(id)) = (from, to, self.by_key(mv.tab)) else {
            return;
        };
        let Some(at) = self.groups[from].tabs.iter().position(|t| *t == id) else {
            return;
        };
        self.groups[from].tabs.remove(at);
        if from != to && self.groups[from].active == id {
            let g = &mut self.groups[from];
            if let Some(next) = g.tabs.get(at).or(g.tabs.last()) {
                g.active = *next;
            }
        }
        let order: Vec<Id> = self.groups[to]
            .tabs
            .iter()
            .filter_map(|t| self.file(*t).map(File::key))
            .collect();
        let index = mv.insert_index(&order);
        self.groups[to].tabs.insert(index, id);
        self.groups[to].active = id;
        self.focus = to;
        let name = self.file(id).map(|f| f.name.clone()).unwrap_or_default();
        let place = if from == to { "reordered" } else { "moved" };
        self.note(format!("{name} {place}"));
    }

    /// A tab released over no strip leaves the groups and opens in a window of its own.
    pub fn tear_off(&mut self, group: usize, key: Id, at: Vec2) {
        let Some(id) = self.by_key(key) else { return };
        self.close(group, key);
        self.floating.push((id, at));
    }

    pub fn save(&mut self, id: u32) {
        if let Some(file) = self.file_mut(id) {
            file.saved = file.text.clone();
            let name = file.name.clone();
            self.note(format!("Saved {name}"));
        }
    }
    pub fn save_all(&mut self) {
        let dirty: Vec<u32> = self
            .files
            .iter()
            .filter(|f| f.dirty())
            .map(|f| f.id)
            .collect();
        for id in dirty {
            self.save(id);
        }
    }
    pub fn active(&self) -> Option<u32> {
        Some(self.groups[self.focus].active).filter(|id| self.groups[self.focus].tabs.contains(id))
    }

    /// The terminal: a handful of commands that act on the workspace.
    pub fn run(&mut self, line: &str) {
        self.terminal.push(format!("$ {line}"));
        let mut words = line.split_whitespace();
        match words.next() {
            Some("help") => self
                .terminal
                .push("help  ls  open <file>  save  close  echo <text>  clear".to_owned()),
            Some("ls") => {
                let names: Vec<String> = self.files.iter().map(|f| f.name.clone()).collect();
                self.terminal.push(names.join("  "));
            }
            Some("open") => match words
                .next()
                .and_then(|n| self.files.iter().find(|f| f.name == n))
            {
                Some(file) if !file.locked => {
                    let id = file.id;
                    self.open(id);
                }
                _ => self.terminal.push("no such file".to_owned()),
            },
            Some("save") => self.save_all(),
            Some("close") => {
                if let Some(id) = self.active() {
                    let key = Id::new(id);
                    self.close(self.focus, key);
                }
            }
            Some("echo") => self.terminal.push(words.collect::<Vec<_>>().join(" ")),
            Some("clear") => self.terminal.clear(),
            Some(other) => self.terminal.push(format!("{other}: command not found")),
            None => {}
        }
    }
}
