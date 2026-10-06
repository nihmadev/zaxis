//! All-in-one: the main zaxis components on one screen, for screenshots.
//! `--smoke-test` runs a few passes and exits.
#![forbid(unsafe_code)]

use zaxis::{
    icons, App, Badge, Button, ButtonVariant, Card, CollapsingHeader, ComboBox, ComboBoxOption,
    Context, Frame, Loader, NumberInput, Progress, ProgressState, RadioGroup, Root, SegmentOption,
    SegmentWidth, SegmentedControl, TextEdit, Theme, Toast, Ui,
};

const W: f32 = 360.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Scheme {
    Dark,
    Light,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum View {
    List,
    Board,
    Table,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Plan {
    Free,
    Pro,
    Team,
}

struct Showcase {
    scheme: Scheme,
    plan: Plan,
    view: View,
    tab: u8,
    name: String,
    notes: String,
    notifications: bool,
    sync: bool,
    agree: bool,
    volume: f32,
    count: u32,
    region: Option<usize>,
    regions: Vec<ComboBoxOption<usize>>,
    clicks: u32,
    smoke: bool,
    passes: usize,
}

impl Showcase {
    fn buttons(&mut self, ui: &mut Ui<'_>) {
        Card::new("buttons").width(W).show(ui, |ui| {
            ui.heading("Buttons & badges");
            ui.horizontal(|ui| {
                for (label, variant) in [
                    ("Solid", ButtonVariant::Solid),
                    ("Soft", ButtonVariant::Soft),
                    ("Outline", ButtonVariant::Outline),
                ] {
                    if ui.add(Button::new(label).variant(variant)).clicked() {
                        self.clicks += 1;
                    }
                }
                ui.add(Button::new("Disabled").enabled(false));
            });
            ui.horizontal(|ui| {
                ui.add(Badge::new("New"));
                ui.add(Badge::new("GPU"));
                ui.add(Badge::new("Rust"));
                ui.muted(format!("clicks: {}", self.clicks));
            });
            if ui
                .add(Button::new("Show toast").variant(ButtonVariant::Soft))
                .clicked()
            {
                ui.context()
                    .toast(Toast::new("Saved").content("Immediate mode, retained speed."));
            }
        });
    }

    fn inputs(&mut self, ui: &mut Ui<'_>) {
        Card::new("inputs").width(W).show(ui, |ui| {
            ui.heading("Inputs");
            ui.add(
                TextEdit::new(&mut self.name)
                    .id_source("name")
                    .placeholder("Your name")
                    .width(W - 40.0),
            );
            ui.add(
                TextEdit::new(&mut self.notes)
                    .id_source("notes")
                    .multiline()
                    .auto_height(2.0, 4.0)
                    .placeholder("Multiline notes…")
                    .width(W - 40.0),
            );
            ui.add(
                ComboBox::new(&mut self.region, &self.regions)
                    .id_source("region")
                    .label("Region")
                    .filterable(true)
                    .width(W - 40.0),
            );
            ui.horizontal(|ui| {
                ui.add(NumberInput::new(&mut self.count).range(0..=99).width(90.0));
                ui.slider(&mut self.volume, 0.0..=1.0);
            });
        });
    }

    fn selection(&mut self, ui: &mut Ui<'_>) {
        Card::new("selection").width(W).show(ui, |ui| {
            ui.heading("Selection");
            ui.checkbox(&mut self.agree, "Accept terms");
            ui.switch(&mut self.notifications, "Notifications");
            ui.switch(&mut self.sync, "Sync");
            ui.muted("Radio group");
            ui.add(RadioGroup::new(
                &mut self.plan,
                [
                    (Plan::Free, "Free"),
                    (Plan::Pro, "Pro"),
                    (Plan::Team, "Team"),
                ],
            ));
            ui.muted("Segmented control");
            ui.add(
                SegmentedControl::new(
                    &mut self.view,
                    [
                        SegmentOption::new(View::List, "List").icon(&icons::LIST),
                        SegmentOption::new(View::Board, "Board").icon(&icons::SQUARE_KANBAN),
                        SegmentOption::new(View::Table, "Table").icon(&icons::TABLE),
                    ],
                )
                .width(SegmentWidth::Fill),
            );
            ui.muted("Theme");
            ui.segmented(
                &mut self.scheme,
                [
                    SegmentOption::new(Scheme::Dark, "Dark").icon(&icons::MOON),
                    SegmentOption::new(Scheme::Light, "Light").icon(&icons::SUN),
                ],
            );
        });
    }

    fn feedback(&mut self, ui: &mut Ui<'_>) {
        Card::new("feedback").width(W).show(ui, |ui| {
            ui.heading("Navigation & feedback");
            ui.tab_bar(
                &mut self.tab,
                [(0u8, "Overview"), (1, "Activity"), (2, "Settings")],
            );
            ui.add(Progress::new(ProgressState::Determinate(self.volume)));
            ui.horizontal(|ui| {
                ui.add(Loader::new());
                ui.muted("Loading…");
            });
            ui.skeleton(14.0);
            ui.skeleton(14.0);
            CollapsingHeader::new("more", "Collapsing header")
                .default_open(true)
                .show(ui, |ui| {
                    ui.muted("Tables, trees, lists, split panes, modals, drag & drop, WGSL materials, accessibility.");
                });
        });
    }
}

impl App for Showcase {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        c.set_theme(match self.scheme {
            Scheme::Dark => Theme::dark(),
            Scheme::Light => Theme::light(),
        });
        Root::new().show(c, |ui| {
            ui.title("zaxis");
            ui.muted("GPU-rendered immediate-mode GUI for Rust");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    self.buttons(ui);
                    ui.add_space(8.0);
                    self.inputs(ui);
                });
                ui.add_space(8.0);
                ui.vertical(|ui| self.selection(ui));
                ui.add_space(8.0);
                ui.vertical(|ui| self.feedback(ui));
            });
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
    let regions = [
        "Frankfurt",
        "Helsinki",
        "Oregon",
        "São Paulo",
        "Singapore",
        "Sydney",
        "Tokyo",
    ]
    .iter()
    .enumerate()
    .map(|(i, name)| ComboBoxOption::new(i, i, *name))
    .collect();
    zaxis::run_with_options(
        Showcase {
            scheme: Scheme::Dark,
            plan: Plan::Pro,
            view: View::Board,
            tab: 0,
            name: String::new(),
            notes: String::new(),
            notifications: true,
            sync: false,
            agree: true,
            volume: 0.65,
            count: 7,
            region: Some(0),
            regions,
            clicks: 0,
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — showcase")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(1180.0, 640.0)),
            ..Default::default()
        },
    )
}
