//! A small IDE built around the tab strip: an explorer, two editor groups whose tabs can
//! be dragged between them, onto each other's text, or out into a window of their own,
//! and a panel with underlined tabs.
#[path = "tabs/editor.rs"]
mod editor;
#[path = "tabs/explorer.rs"]
mod explorer;
#[path = "tabs/model.rs"]
mod model;
#[path = "tabs/panel.rs"]
mod panel;

use model::Workspace;
use zaxis::{
    vec2, App, Button, ButtonVariant, Context, Frame, Id, MenuBar, MenuItem, Root, SplitPanel,
    SplitSize, TextEdit, Theme, Window,
};

struct Ide {
    work: Workspace,
    light: bool,
    smoke: bool,
    passes: u32,
}

impl Ide {
    fn menus(&self) -> Vec<MenuItem> {
        vec![
            MenuItem::submenu(
                "File",
                [
                    MenuItem::new("new", "New file"),
                    MenuItem::new("save", "Save").shortcut("Ctrl+S"),
                    MenuItem::new("save-all", "Save all"),
                ],
            ),
            MenuItem::submenu(
                "View",
                [MenuItem::new("light", "Light theme").checked(self.light)],
            ),
        ]
    }

    fn command(&mut self, id: Id) {
        let is = |name: &str| Id::new(name) == id;
        if is("new") {
            self.work.create(self.work.focus);
        } else if is("save") {
            if let Some(file) = self.work.active() {
                self.work.save(file);
            }
        } else if is("save-all") {
            self.work.save_all();
        } else if is("light") {
            self.light = !self.light;
        }
    }

    fn apply(&mut self, p: editor::Pending) {
        let work = &mut self.work;
        if let Some(group) = p.create {
            work.create(group);
        }
        if let Some((group, key)) = p.close {
            work.close(group, key);
        }
        if let Some(mv) = p.moved {
            work.apply(mv);
        }
        if let Some((group, key, at)) = p.torn {
            work.tear_off(group, key, at);
        }
        if let Some(group) = p.send {
            // Send the active tab to the other group, behind its last tab.
            let other = 1 - group;
            if let Some(key) = work.file(work.groups[group].active).map(model::File::key) {
                work.apply(zaxis::TabMove {
                    tab: key,
                    from_bar: work.groups[group].bar(),
                    to_bar: work.groups[other].bar(),
                    before: None,
                });
            }
        }
        if let Some(group) = p.focus {
            work.focus = group;
        }
    }

    fn floating(&mut self, c: &mut Context) {
        let mut dock = Vec::new();
        for (id, at) in self.work.floating.clone() {
            let Some(file) = self.work.file_mut(id) else {
                continue;
            };
            let title = file.name.clone();
            Window::new(title)
                .id(Id::new(("float", id)))
                .default_position(at - vec2(40.0, 12.0))
                .default_size(vec2(420.0, 300.0))
                .show(c, |ui| {
                    if ui
                        .add(Button::new("Dock").variant(ButtonVariant::Soft))
                        .clicked()
                    {
                        dock.push(id);
                    }
                    ui.add(
                        TextEdit::new(&mut file.text)
                            .id_source(("float-text", id))
                            .multiline()
                            .monospace()
                            .width(ui.available_width())
                            .height(ui.available_height()),
                    );
                });
        }
        for id in dock {
            self.work.open(id);
        }
    }
}

impl App for Ide {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        c.set_theme(if self.light {
            Theme::light()
        } else {
            Theme::dark()
        });
        let input = c.input();
        let save = input.modifiers.control_key()
            && input
                .keys_pressed
                .contains(&zaxis::winit::keyboard::KeyCode::KeyS);
        if save {
            if let Some(file) = self.work.active() {
                self.work.save(file);
            }
        }
        let items = self.menus();
        let mut picked = None;
        let mut pending = editor::Pending::default();
        Root::new().padding(zaxis::Padding::all(0.0)).show(c, |ui| {
            picked = MenuBar::new("menu", &items).show(ui).selected;
            ui.split_vertical(
                "ide",
                [
                    SplitPanel::new("work").default_size(SplitSize::Weight(1.0)),
                    SplitPanel::new("status")
                        .default_size(SplitSize::Pixels(26.0))
                        .min_size(26.0),
                ],
                |split| {
                    split.panel("work", |ui| {
                        ui.split_horizontal(
                            "main",
                            [
                                SplitPanel::new("explorer")
                                    .default_size(SplitSize::Pixels(210.0))
                                    .min_size(120.0),
                                SplitPanel::new("center")
                                    .default_size(SplitSize::Weight(1.0))
                                    .min_size(300.0),
                            ],
                            |split| {
                                split.panel("explorer", |ui| explorer::show(ui, &mut self.work));
                                split.panel("center", |ui| {
                                    ui.split_vertical(
                                        "center-rows",
                                        [
                                            SplitPanel::new("groups")
                                                .default_size(SplitSize::Weight(3.0))
                                                .min_size(160.0),
                                            SplitPanel::new("panel")
                                                .default_size(SplitSize::Pixels(190.0))
                                                .min_size(90.0),
                                        ],
                                        |split| {
                                            split.panel("groups", |ui| {
                                                ui.split_horizontal(
                                                    "groups-cols",
                                                    [
                                                        SplitPanel::new("g0")
                                                            .default_size(SplitSize::Weight(1.0))
                                                            .min_size(180.0),
                                                        SplitPanel::new("g1")
                                                            .default_size(SplitSize::Weight(1.0))
                                                            .min_size(180.0),
                                                    ],
                                                    |split| {
                                                        split.panel("g0", |ui| {
                                                            editor::show(
                                                                ui,
                                                                &mut self.work,
                                                                0,
                                                                &mut pending,
                                                            )
                                                        });
                                                        split.panel("g1", |ui| {
                                                            editor::show(
                                                                ui,
                                                                &mut self.work,
                                                                1,
                                                                &mut pending,
                                                            )
                                                        });
                                                    },
                                                );
                                            });
                                            split.panel("panel", |ui| {
                                                panel::show(ui, &mut self.work)
                                            });
                                        },
                                    );
                                });
                            },
                        );
                    });
                    split.panel("status", |ui| {
                        let work = &self.work;
                        let name = work
                            .active()
                            .and_then(|id| work.file(id))
                            .map_or(String::new(), |f| f.name.clone());
                        let dirty = work.files.iter().filter(|f| f.dirty()).count();
                        ui.muted(format!(
                            "group {}   {name}   {dirty} unsaved",
                            work.focus + 1
                        ));
                    });
                },
            );
        });
        if let Some(id) = picked {
            self.command(id);
        }
        self.apply(pending);
        self.floating(c);
        if self.smoke {
            self.passes += 1;
            if self.passes >= 3 {
                frame.close();
            } else {
                c.request_repaint();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Ide {
            work: Workspace::new(),
            light: false,
            smoke: std::env::args().any(|arg| arg == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Tabs IDE")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(1180.0, 720.0)),
            ..Default::default()
        },
    )
}
