//! An IDE workspace using the library Dock, with application-owned panels and layout.
#![forbid(unsafe_code)]
#[path = "dock/content.rs"]
mod content;
#[path = "dock/tree.rs"]
mod tree;
#[path = "dock/workspace.rs"]
mod workspace;
use workspace::{panel, Workspace};
use zaxis::winit::keyboard::KeyCode;
use zaxis::{
    Action, Actions, App, Button, Context, Dock, DockActions, DockMotion, DockPreset, DockStyle,
    Frame, Mods, Padding, Root, ScrollArea, Theme,
};

#[derive(Debug, Hash)]
enum Command {
    Close,
    Right,
    Down,
}
struct Demo {
    workspace: Workspace,
    preset: DockPreset,
    motion: DockMotion,
    light: bool,
    reduced: bool,
    stress: bool,
    installed: bool,
    smoke: bool,
    frames: usize,
}
impl App for Demo {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        if !self.installed {
            c.set_actions(
                Actions::new()
                    .register(
                        Action::new(Command::Close, "Close panel")
                            .shortcut(Mods::PRIMARY.key(KeyCode::KeyW)),
                    )
                    .register(
                        Action::new(Command::Right, "Split right")
                            .shortcut((Mods::PRIMARY | Mods::ALT).key(KeyCode::KeyR)),
                    )
                    .register(
                        Action::new(Command::Down, "Split down")
                            .shortcut((Mods::PRIMARY | Mods::ALT).key(KeyCode::KeyD)),
                    ),
            );
            self.installed = true;
        }
        let mut theme = if self.light {
            Theme::light()
        } else {
            Theme::dark()
        };
        let mut motion = c.style().motion.clone();
        motion.reduced_motion = self.reduced;
        theme.overrides.motion = Some(motion);
        c.set_theme(theme);
        Root::new().padding(Padding::all(8.0)).show(c, |ui| {
            ScrollArea::horizontal()
                .id_source("toolbar")
                .max_height(38.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (preset, label) in
                            [(DockPreset::Tiled, "Tiled"), (DockPreset::Flat, "Flat")]
                        {
                            if ui
                                .add(Button::new(label).selected(self.preset == preset))
                                .clicked()
                            {
                                self.preset = preset;
                            }
                        }
                        for (motion, label) in [
                            (DockMotion::Snappy, "Snappy"),
                            (DockMotion::Smooth, "Smooth"),
                            (DockMotion::Off, "Off"),
                        ] {
                            if ui
                                .add(Button::new(label).selected(self.motion == motion))
                                .clicked()
                            {
                                self.motion = motion;
                            }
                        }
                        ui.checkbox(&mut self.light, "Light");
                        ui.checkbox(&mut self.reduced, "Reduced motion");
                        if ui.checkbox(&mut self.stress, "64 panels").changed() {
                            self.workspace.layout = if self.stress {
                                workspace::stress()
                            } else {
                                workspace::layout()
                            };
                        }
                        if ui
                            .add(
                                Button::new("Open Files")
                                    .enabled(!self.workspace.layout.contains(panel(0))),
                            )
                            .clicked()
                        {
                            let target =
                                self.workspace.layout.contains(panel(1)).then_some(panel(1));
                            let _ = self.workspace.layout.open(panel(0), target);
                        }
                        if ui.button("Save").clicked() {
                            self.workspace.save();
                        }
                        if ui.button("Load").clicked() {
                            self.workspace.load();
                        }
                    });
                });
            let actions = DockActions {
                close: Some(Action::new(Command::Close, "").id()),
                split_right: Some(Action::new(Command::Right, "").id()),
                split_down: Some(Action::new(Command::Down, "").id()),
            };
            Dock::new("workspace", &mut self.workspace.layout)
                .style(DockStyle {
                    preset: Some(self.preset),
                    motion: Some(self.motion),
                    minimum: Some(zaxis::vec2(
                        if self.stress { 45.0 } else { 120.0 },
                        if self.stress { 42.0 } else { 90.0 },
                    )),
                    ..Default::default()
                })
                .actions(actions)
                .show(ui, &mut self.workspace.content);
            if let Some(error) = self.workspace.error.take() {
                ui.context().toast(zaxis::Toast::new(error));
            }
        });
        if self.smoke {
            self.frames += 1;
            if self.frames >= 4 {
                frame.close();
            } else {
                c.request_repaint();
            }
        }
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Demo {
            workspace: Workspace::new(),
            preset: DockPreset::Tiled,
            motion: DockMotion::Snappy,
            light: false,
            reduced: false,
            stress: false,
            installed: false,
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            frames: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Dock")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(1280.0, 800.0)),
            ..Default::default()
        },
    )
}
