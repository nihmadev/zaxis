use zaxis::{
    App, Button, CollapsingHeader, Context, Frame, ImageSource, Root, ScrollArea, Theme, Ui,
};

struct Settings {
    open: bool,
    flags: [bool; 3],
    name: String,
    order: Vec<u64>,
    theme: usize,
    reduced: bool,
    smoke: bool,
    passes: usize,
    icon: ImageSource,
}
impl Settings {
    fn content(&mut self, ui: &mut Ui<'_>) {
        ui.horizontal(|ui| {
            for (n, label) in ["Dark", "Light", "High contrast"].into_iter().enumerate() {
                if ui
                    .add(Button::new(label).selected(self.theme == n))
                    .clicked()
                {
                    self.theme = n;
                }
            }
            ui.checkbox(&mut self.reduced, "Reduced motion");
        });
        ScrollArea::vertical()
            .id_source("settings-scroll")
            .max_height(ui.available_height())
            .show(ui, |ui| {
                let out = CollapsingHeader::new("appearance", "Appearance")
                    .open(&mut self.open)
                    .icon(&self.icon)
                    .show_with_actions(
                        ui,
                        |ui| ui.button("Reset").clicked(),
                        |ui| {
                            ui.checkbox(&mut self.flags[0], "Animations");
                            CollapsingHeader::new("advanced", "Advanced")
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.checkbox(&mut self.flags[1], "Smooth scrolling");
                                    ui.text_edit(&mut self.name);
                                });
                        },
                    );
                if out.actions {
                    self.flags = [true; 3];
                }
                ui.horizontal(|ui| {
                    if ui.button("Toggle Appearance").clicked() {
                        self.open = !self.open;
                    }
                    if ui.button("Reorder").clicked() {
                        self.order.reverse();
                    }
                });
                for &key in &self.order {
                    CollapsingHeader::new(key, "Options")
                        .default_open(key == 1)
                        .show(ui, |ui| {
                            ui.push_id(key, |ui| {
                                ui.checkbox(&mut self.flags[2], "Enabled");
                            });
                        });
                }
                CollapsingHeader::new("locked", "Always available")
                    .default_open(true)
                    .expandable(false)
                    .show(ui, |ui| {
                        ui.checkbox(&mut self.flags[0], "Animations");
                    });
            });
    }
}
impl App for Settings {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        let mut theme = match self.theme {
            1 => Theme::light(),
            2 => Theme::high_contrast(),
            _ => Theme::dark(),
        };
        let mut motion = c.style().motion.clone();
        motion.reduced_motion = self.reduced;
        theme.overrides.motion = Some(motion);
        c.set_theme(theme);
        Root::new().show(c, |ui| self.content(ui));
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
        Settings {
            open: true,
            flags: [true; 3],
            name: "Workspace".into(),
            order: vec![1, 2],
            theme: 0,
            reduced: false,
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            passes: 0,
            icon: ImageSource::bytes(include_bytes!("../assets/images/icon.svg")),
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Collapsing headers")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(720.0, 620.0)),
            ..Default::default()
        },
    )
}
