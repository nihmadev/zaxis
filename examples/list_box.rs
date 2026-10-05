use std::collections::HashSet;
use zaxis::{
    icons, move_item, vec2, App, Badge, Context, Frame, Id, Image, Insertion, ListBox, ListEntry,
    ListEvent, ListMode, Root, SegmentOption, SegmentedControl, SegmentedSize, Switch, Text,
    TextEdit, Theme, Ui,
};

const COUNT: u32 = 120_000;
const GROUP: u32 = 1_000;
const STEMS: [&str; 6] = ["report", "invoice", "design", "backup", "notes", "render"];
const EXTENSIONS: [&str; 5] = ["rs", "md", "png", "zip", "mp3"];

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Pick {
    Single,
    Multiple,
    Checks,
}
enum Line {
    Group(u32),
    File(u32),
}

struct Demo {
    names: Vec<String>,
    lowercase: Vec<String>,
    groups: Vec<String>,
    order: Vec<u32>,
    lines: Vec<Line>,
    revision: u64,
    filter: String,
    pick: Pick,
    sections: bool,
    auto_height: bool,
    reorder: bool,
    selected: HashSet<Id>,
    pinned: HashSet<u32>,
    opened: Option<u32>,
    smoke: bool,
    passes: usize,
}

fn key(id: u32) -> Id {
    Id::new(id)
}
fn icon(id: u32) -> &'static icons::Icon {
    match id % 5 {
        0 => &icons::FILE_CODE,
        1 => &icons::FILE_TEXT,
        2 => &icons::IMAGE,
        3 => &icons::ARCHIVE,
        _ => &icons::MUSIC_2,
    }
}

impl Demo {
    fn new(smoke: bool) -> Self {
        let names: Vec<String> = (0..COUNT)
            .map(|id| {
                let stem = STEMS[id as usize % STEMS.len()];
                format!(
                    "{stem}-{id:06}.{}",
                    EXTENSIONS[id as usize % EXTENSIONS.len()]
                )
            })
            .collect();
        let mut demo = Self {
            lowercase: names.iter().map(|n| n.to_lowercase()).collect(),
            groups: (0..COUNT.div_ceil(GROUP))
                .map(|g| format!("Folder {g:03}"))
                .collect(),
            names,
            order: (0..COUNT).collect(),
            lines: Vec::new(),
            revision: 0,
            filter: String::new(),
            pick: Pick::Multiple,
            sections: true,
            auto_height: false,
            reorder: false,
            selected: HashSet::new(),
            pinned: HashSet::new(),
            opened: None,
            smoke,
            passes: 0,
        };
        demo.rebuild();
        demo
    }

    /// The visible sequence: the filter applied to the user's order, with a header per folder.
    fn rebuild(&mut self) {
        let needle = self.filter.trim().to_lowercase();
        self.lines.clear();
        let mut group = u32::MAX;
        for &id in &self.order {
            if !needle.is_empty() && !self.lowercase[id as usize].contains(&needle) {
                continue;
            }
            if self.sections && id / GROUP != group {
                group = id / GROUP;
                self.lines.push(Line::Group(group));
            }
            self.lines.push(Line::File(id));
        }
        self.revision += 1;
    }

    fn apply(&mut self, events: &[ListEvent]) {
        let position = |order: &[u32], k: Id| order.iter().position(|id| key(*id) == k);
        let mut changed = false;
        for event in events {
            match *event {
                ListEvent::Activated(k) => {
                    self.opened = self.order.iter().copied().find(|id| key(*id) == k)
                }
                ListEvent::Context(k) => {
                    self.order.retain(|id| key(*id) != k);
                    self.selected.remove(&k);
                    changed = true;
                }
                ListEvent::Moved {
                    key: moved,
                    target,
                    position: place,
                } => {
                    if let (Some(from), Some(at)) =
                        (position(&self.order, moved), position(&self.order, target))
                    {
                        let slot = if place == Insertion::Before {
                            at
                        } else {
                            at + 1
                        };
                        move_item(
                            &mut self.order,
                            from,
                            if from < slot { slot - 1 } else { slot },
                        );
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        if changed {
            self.rebuild();
        }
    }

    fn controls(&mut self, ui: &mut Ui<'_>) {
        ui.horizontal(|ui| {
            if ui
                .add(
                    TextEdit::new(&mut self.filter)
                        .placeholder("Filter")
                        .width(220.0),
                )
                .changed()
            {
                self.rebuild();
            }
            ui.add_space(8.0);
            ui.add(
                SegmentedControl::new(
                    &mut self.pick,
                    [
                        SegmentOption::new(Pick::Single, "Single"),
                        SegmentOption::new(Pick::Multiple, "Multiple"),
                        SegmentOption::new(Pick::Checks, "Checks"),
                    ],
                )
                .size(SegmentedSize::Small),
            );
            ui.add_space(8.0);
            if ui.add(Switch::new(&mut self.sections, "Folders")).changed() {
                self.rebuild();
            }
            ui.add(Switch::new(&mut self.auto_height, "Auto height"));
            ui.add(Switch::new(&mut self.reorder, "Reorder"));
        });
    }

    fn list(&mut self, ui: &mut Ui<'_>) {
        let Self {
            names,
            groups,
            lines,
            selected,
            pinned,
            ..
        } = self;
        let mut list = ListBox::new("files")
            .mode(match self.pick {
                Pick::Single => ListMode::Single,
                Pick::Multiple => ListMode::Multiple,
                Pick::Checks => ListMode::Checks,
            })
            .selection(selected)
            .drag_rows(self.reorder)
            .revision(self.revision)
            .max_height((ui.available_height() - 44.0).max(120.0))
            .empty_text("No matches");
        list = if self.auto_height {
            list.measured_rows(44.0)
        } else {
            list.row_height(44.0)
        };
        let auto = self.auto_height;
        let out = list.show_rows(
            ui,
            lines.len(),
            |i| match lines[i] {
                Line::Group(g) => ListEntry::header(("group", g), &groups[g as usize]),
                // Every thirteenth file is read-only.
                Line::File(id) => ListEntry::item(id, &names[id as usize]).enabled(id % 13 != 0),
            },
            |ui, row| {
                let Line::File(id) = lines[row.index] else {
                    return;
                };
                ui.add(Image::new(icon(id)).size(vec2(18.0, 18.0)).tint(row.muted));
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.add(Text::new(&names[id as usize]).color(row.color).wrap(false));
                    let detail = format!("{} KB · {} days ago", id * 37 % 900 + 4, id % 31 + 1);
                    ui.add(Text::new(detail).size(12.0).color(row.muted).wrap(false));
                    if auto && id % 3 == 0 {
                        let note = format!("Reviewed by {} people", id % 7 + 1);
                        ui.add(Text::new(note).size(12.0).color(row.muted).wrap(false));
                    }
                });
                row.trailing(ui, |ui| {
                    let on = pinned.contains(&id);
                    if on {
                        ui.add(Badge::new("Pinned").font_size(11.0));
                        ui.add_space(6.0);
                    }
                    if ui.button(if on { "Unpin" } else { "Pin" }).clicked() && !pinned.remove(&id)
                    {
                        pinned.insert(id);
                    }
                });
            },
        );
        self.apply(&out.events);
    }
}

impl App for Demo {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        c.set_theme(Theme::dark());
        Root::new().show(c, |ui| {
            self.controls(ui);
            ui.add_space(8.0);
            self.list(ui);
            if let Some(id) = self.opened {
                ui.add_space(6.0);
                ui.label(self.names[id as usize].clone());
            }
        });
        if self.smoke {
            self.passes += 1;
            match self.passes {
                2 => {
                    self.filter = "9".into();
                    self.rebuild();
                    c.request_repaint();
                }
                p if p >= 4 => frame.close(),
                _ => c.request_repaint(),
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Demo::new(std::env::args().any(|a| a == "--smoke-test")),
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — List box")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(760.0, 680.0)),
            ..Default::default()
        },
    )
}
