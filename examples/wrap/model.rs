//! The application's data: the user's choices and the tasks they filter.
use zaxis::Align;

/// A position along an axis, as the user picks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edge {
    Start,
    Middle,
    End,
}

impl Edge {
    pub fn align(self) -> Align {
        match self {
            Edge::Start => Align::Start,
            Edge::Middle => Align::Center,
            Edge::End => Align::End,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Flag {
    Open,
    Bug,
    Docs,
    Urgent,
    Mine,
}

impl Flag {
    pub const ALL: [Flag; 5] = [Flag::Open, Flag::Bug, Flag::Docs, Flag::Urgent, Flag::Mine];

    pub fn label(self) -> &'static str {
        match self {
            Flag::Open => "Open",
            Flag::Bug => "Bug",
            Flag::Docs => "Docs",
            Flag::Urgent => "Urgent",
            Flag::Mine => "Mine",
        }
    }

    fn matches(self, task: &Task) -> bool {
        match self {
            Flag::Open => task.open,
            Flag::Bug => task.bug,
            Flag::Docs => task.docs,
            Flag::Urgent => task.urgent,
            Flag::Mine => task.mine,
        }
    }
}

pub struct Task {
    pub title: &'static str,
    pub note: &'static str,
    pub open: bool,
    pub bug: bool,
    pub docs: bool,
    pub urgent: bool,
    pub mine: bool,
}

/// `flags` lists the properties by initial: open, bug, docs, urgent, mine.
fn task(title: &'static str, note: &'static str, flags: &str) -> Task {
    Task {
        title,
        note,
        open: flags.contains('o'),
        bug: flags.contains('b'),
        docs: flags.contains('d'),
        urgent: flags.contains('u'),
        mine: flags.contains('m'),
    }
}

pub struct Model {
    pub tasks: Vec<Task>,
    pub active: Vec<Flag>,
    pub tags: Vec<String>,
    pub draft: String,
    pub menu: bool,
    pub item_align: Edge,
    pub line_align: Edge,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            tasks: vec![
                task(
                    "Crash on resize",
                    "Reproduces with the window below 300 px.",
                    "obum",
                ),
                task("Update the install guide", "", "od"),
                task(
                    "Slow scrolling in long lists",
                    "Profile the row cache first, then the text layout.",
                    "ob",
                ),
                task("Document the theme tokens", "", "odm"),
                task("Tooltip clipped by a window edge", "", "obu"),
                task(
                    "Export settings",
                    "Needs a file dialog and a schema version.",
                    "om",
                ),
                task("Fix typos in the changelog", "", "dm"),
                task(
                    "Focus lost after reorder",
                    "Only when the item moves to another line.",
                    "obum",
                ),
                task("Release checklist", "", "ou"),
                task("Remove the old demo", "Superseded by the showcase.", "bd"),
            ],
            active: Vec::new(),
            tags: ["resize", "focus", "guide", "list"]
                .map(String::from)
                .into(),
            draft: String::new(),
            menu: false,
            item_align: Edge::Start,
            line_align: Edge::Start,
        }
    }
}

impl Model {
    pub fn is_on(&self, flag: Flag) -> bool {
        self.active.contains(&flag)
    }

    pub fn toggle(&mut self, flag: Flag) {
        match self.active.iter().position(|f| *f == flag) {
            Some(index) => {
                self.active.remove(index);
            }
            None => self.active.push(flag),
        }
    }

    /// Add the drafted word as a tag unless it is empty or already there.
    pub fn add_tag(&mut self) {
        let word = self.draft.trim().to_lowercase();
        self.draft.clear();
        if !word.is_empty() && !self.tags.contains(&word) {
            self.tags.push(word);
        }
    }

    pub fn clear(&mut self) {
        self.active.clear();
        self.tags.clear();
    }

    /// Indices of the tasks that satisfy every flag and, with tags, contain at least one.
    pub fn visible(&self) -> Vec<usize> {
        (0..self.tasks.len())
            .filter(|&i| {
                let task = &self.tasks[i];
                let text = format!("{} {}", task.title, task.note).to_lowercase();
                self.active.iter().all(|flag| flag.matches(task))
                    && (self.tags.is_empty()
                        || self.tags.iter().any(|tag| text.contains(tag.as_str())))
            })
            .collect()
    }
}
