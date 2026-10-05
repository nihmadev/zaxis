use zaxis::{
    App, Card, Color, Context, Field, Frame, Progress, ProgressState, RadioGroup, RadioOption,
    RadioSize, Root, ScrollArea, Text, Theme, Ui, Validation,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Scheme {
    Dark,
    Light,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Preview {
    Compact,
    Normal,
    Large,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Accent {
    Coral,
    Mint,
    Sky,
    Amber,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Plan {
    Free,
    Pro,
    Team,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Delivery {
    Standard,
    Express,
}

struct Gallery {
    scheme: Scheme,
    level: u8,
    preview: Preview,
    accent: Accent,
    plan: Plan,
    delivery: Option<Delivery>,
    small: u8,
    smoke: bool,
    passes: usize,
}

impl Gallery {
    fn content(&mut self, ui: &mut Ui<'_>) {
        ui.add(
            RadioGroup::new(
                &mut self.scheme,
                [(Scheme::Dark, "Dark"), (Scheme::Light, "Light")],
            )
            .horizontal(),
        );
        ui.add_space(12.0);

        Card::new("levels").show(ui, |ui| {
            ui.horizontal(|ui| {
                let options = (1..=5u8)
                    .map(|n| RadioOption::new(n, format!("Radio button {n}")).enabled(n != 5));
                ui.add(RadioGroup::new(&mut self.level, options));
                ui.add_space(32.0);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.add(Progress::new(ProgressState::Determinate(
                        f32::from(self.level) / 4.0,
                    )));
                });
            });
        });
        ui.add_space(12.0);

        Card::new("preview").show(ui, |ui| {
            ui.add(
                RadioGroup::new(
                    &mut self.preview,
                    [
                        (Preview::Compact, "Compact"),
                        (Preview::Normal, "Normal"),
                        (Preview::Large, "Large"),
                    ],
                )
                .horizontal()
                .size(RadioSize::Small)
                .focus_follows_selection(false),
            );
            ui.add_space(8.0);
            ui.add(Text::new("Preview").size(match self.preview {
                Preview::Compact => 12.0,
                Preview::Normal => 16.0,
                Preview::Large => 26.0,
            }));
        });
        ui.add_space(12.0);

        Card::new("accent").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    RadioGroup::new(
                        &mut self.accent,
                        [
                            (Accent::Coral, "Coral"),
                            (Accent::Mint, "Mint"),
                            (Accent::Sky, "Sky"),
                            (Accent::Amber, "Amber"),
                        ],
                    )
                    .columns(2)
                    .column_gap(28.0),
                );
                ui.add_space(24.0);
                let color = match self.accent {
                    Accent::Coral => Color::rgb(240, 110, 90),
                    Accent::Mint => Color::rgb(70, 190, 150),
                    Accent::Sky => Color::rgb(70, 150, 235),
                    Accent::Amber => Color::rgb(235, 175, 50),
                };
                ui.add(Text::new("Accent").size(22.0).color(color));
            });
        });
        ui.add_space(12.0);

        Card::new("plan").width(320.0).show(ui, |ui| {
            ui.add(RadioGroup::new(
                &mut self.plan,
                [
                    RadioOption::new(Plan::Free, "Free").description("For personal projects"),
                    RadioOption::new(Plan::Pro, "Pro").description(
                        "Unlimited projects with priority support and a longer history",
                    ),
                    RadioOption::new(Plan::Team, "Team")
                        .description("Shared workspaces")
                        .tooltip("Per seat"),
                ],
            ));
            ui.add_space(8.0);
            ui.add(Text::new(match self.plan {
                Plan::Free => "$0",
                Plan::Pro => "$12",
                Plan::Team => "$30",
            }));
        });
        ui.add_space(12.0);

        Card::new("delivery").show(ui, |ui| {
            let validation = if self.delivery.is_none() {
                Validation::error("Choose a delivery option")
            } else {
                Validation::ok()
            };
            Field::new("Delivery")
                .validation(validation)
                .show(ui, |ui| {
                    ui.add(RadioGroup::new(
                        &mut self.delivery,
                        [
                            (Some(Delivery::Standard), "Standard"),
                            (Some(Delivery::Express), "Express"),
                        ],
                    ));
                });
        });
        ui.add_space(12.0);

        Card::new("locked").show(ui, |ui| {
            ui.add(
                RadioGroup::new(
                    &mut self.small,
                    (0..3u8).map(|n| (n, format!("Option {n}"))),
                )
                .enabled(false),
            );
        });
    }
}

impl App for Gallery {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        c.set_theme(match self.scheme {
            Scheme::Dark => Theme::dark(),
            Scheme::Light => Theme::light(),
        });
        Root::new().show(c, |ui| {
            ScrollArea::vertical().show(ui, |ui| self.content(ui));
        });
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
        Gallery {
            scheme: Scheme::Dark,
            level: 1,
            preview: Preview::Normal,
            accent: Accent::Coral,
            plan: Plan::Pro,
            delivery: None,
            small: 1,
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Radio group")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(560.0, 820.0)),
            ..Default::default()
        },
    )
}
