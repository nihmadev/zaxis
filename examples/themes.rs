//! cargo run --example themes
use zaxis::{
    winit::{dpi::LogicalSize, keyboard::KeyCode},
    *,
};

struct Themes {
    mode: usize,
    compact: bool,
    accent: usize,
    glass: bool,
    animate: bool,
    checked: bool,
    slider: f32,
    number: f64,
    text: String,
    empty: String,
    color: Color,
    choice: Option<usize>,
    row: Option<Id>,
    popup: bool,
    options: Vec<ComboBoxOption<usize>>,
}
impl Default for Themes {
    fn default() -> Self {
        Self {
            mode: 0,
            compact: false,
            accent: 0,
            glass: false,
            animate: false,
            checked: true,
            slider: 0.45,
            number: 42.0,
            text: "Text".into(),
            empty: String::new(),
            color: Color::rgb(105, 155, 205),
            choice: Some(2),
            row: Some(Id::new(2)),
            popup: false,
            options: (0..20)
                .map(|i| ComboBoxOption::new(i, i, format!("Option {i:02}")))
                .collect(),
        }
    }
}
impl Themes {
    fn theme(&self) -> Theme {
        let mut t = match self.mode {
            1 => Theme::light(),
            2 => Theme::high_contrast(),
            _ => Theme::dark(),
        };
        if self.accent > 0 {
            t = t.accent(
                [
                    Color::rgb(110, 170, 235),
                    Color::rgb(130, 200, 155),
                    Color::rgb(210, 130, 200),
                ][self.accent - 1],
            );
        }
        t = t.density(if self.compact {
            Density::Compact
        } else {
            Density::Comfortable
        });
        t.metrics.blur = if self.glass { 12.0 } else { 0.0 };
        t
    }
    fn controls(&mut self, ui: &mut Ui<'_>) {
        ui.add(Text::new("Controls").typography(TypographyRole::Heading));
        ui.horizontal(|ui| {
            ui.button("Button");
            ui.add(Button::new("Selected").selected(true));
        });
        ui.horizontal(|ui| {
            ui.add(Button::new("Disabled").enabled(false));
            ui.add(Button::new("Success").status(SemanticStatus::Success));
        });
        ui.checkbox(&mut self.checked, "Checked");
        ui.add_enabled(false, Checkbox::new(&mut false, "Disabled"));
        ui.add(
            Slider::new(&mut self.slider, 0.0..=1.0)
                .width(300.0)
                .text("Accent"),
        );
        ui.add(
            Slider::new(&mut self.slider, 0.0..=1.0)
                .width(300.0)
                .status(SliderStatus::Warning),
        );
        ui.label("Input");
        ui.add(
            TextEdit::new(&mut self.text)
                .id_source("editor")
                .width(300.0),
        );
        ui.add(
            TextEdit::new(&mut self.empty)
                .placeholder("Placeholder")
                .width(300.0),
        );
        ui.horizontal(|ui| {
            ui.number_input(&mut self.number);
            ui.drag_value(&mut self.number);
        });
        ui.separator();
        let explicit = ButtonStyle {
            surface: ControlStyle {
                idle: SurfaceStyle {
                    foreground: Some(Color::WHITE),
                    rounding: Some(2.0.into()),
                    ..SurfaceStyle::fill(Color::rgb(80, 45, 105))
                },
                hover: SurfaceStyle::fill(Color::rgb(105, 60, 135)),
                ..Default::default()
            },
            ..Default::default()
        };
        ui.add(Button::new("Override").style(explicit));
        ui.add(
            Button::new("Custom").painter(PaintMode::Replace, |p, info| {
                let s = info.style;
                p.paint(
                    Shape::rect(info.bounds, s.fill.unwrap().start)
                        .corner_radius(s.rounding.unwrap())
                        .border(s.border.unwrap()),
                );
                p.paint(Shape::rect(
                    Rect::from_min_size(
                        info.bounds.min + vec2(4.0, 4.0),
                        vec2(3.0, (info.bounds.size().y - 8.0).max(0.0)),
                    ),
                    s.foreground.unwrap(),
                ));
            }),
        );
        ui.horizontal(|ui| {
            ui.add(Loader::new().active(false));
            ui.add(Progress::new(ProgressState::Determinate(self.slider)));
        });
    }
    fn colors(&mut self, ui: &mut Ui<'_>) {
        ui.add(Text::new("Colors").typography(TypographyRole::Heading));
        ui.add(
            ColorPicker::new(&mut self.color, "Color")
                .width(310.0)
                .default_open(true),
        );
        ui.add(
            ComboBox::new(&mut self.choice, &self.options)
                .label("Select")
                .width(310.0)
                .filterable(true),
        );
        let r = ui.button("Popup");
        if r.clicked() {
            self.popup = !self.popup;
        }
        let local = if self.mode == 1 {
            Theme::dark()
        } else {
            Theme::light()
        }
        .accent(Color::rgb(140, 65, 160));
        ui.with_theme(&local, |ui| {
            Grid::new("local-card")
                .column(Column::remainder("content"))
                .show(ui, |grid| {
                    grid.row(0, |row| {
                        row.cell(|ui| {
                            ui.label("Local theme");
                            ui.button("Local button");
                            let patch = StyleOverrides {
                                button: ButtonStyle {
                                    surface: ControlStyle {
                                        idle: SurfaceStyle {
                                            border: Some(Border::NONE),
                                            rounding: Some(CornerRadius::ZERO),
                                            blur: Some(0.0),
                                            ..SurfaceStyle::fill(Color::rgb(65, 85, 105))
                                        },
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                ..Default::default()
                            };
                            ui.with_style(&patch, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(Button::new("Override").style(ButtonStyle {
                                        surface: ControlStyle {
                                            idle: SurfaceStyle {
                                                foreground: Some(Color::WHITE),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    }));
                                });
                            });
                        })
                    })
                });
            let output = Popup::new("local-popup", r.rect)
                .size(vec2(290.0, 150.0))
                .show(ui, &mut self.popup, |ui| {
                    ui.label("Local popup");
                    ui.checkbox(&mut self.checked, "Checked");
                    ui.button("Close").clicked()
                });
            if output.is_some_and(|p| p.inner) {
                self.popup = false;
            }
        });
    }
    fn data(&mut self, ui: &mut Ui<'_>) {
        ui.add(Text::new("Data").typography(TypographyRole::Heading));
        let table = Table::new("table")
            .column(Column::fixed("n", 75.0).title("ID").sortable(true))
            .column(Column::remainder("name").title("Item"))
            .max_height(250.0)
            .selected_row(self.row)
            .show(ui, |body| {
                for i in 0..12 {
                    body.row(i, |row| {
                        row.cell(|ui| {
                            ui.label(format!("{i:02}"));
                        });
                        row.cell(|ui| {
                            ui.label(format!("Item {i}"));
                        });
                    });
                }
            });
        self.row = table.selected_row;
        Grid::new("grid")
            .column(Column::remainder("a"))
            .column(Column::remainder("b"))
            .show(ui, |g| {
                g.row(0, |row| {
                    row.cell(|ui| {
                        ui.label("Grid");
                    });
                    row.cell(|ui| {
                        ui.checkbox(&mut self.checked, "Cell");
                    });
                });
            });
        ScrollArea::vertical()
            .id_source("list")
            .max_height(120.0)
            .show(ui, |ui| {
                for i in 0..15 {
                    ui.label(format!("Item {i:02}"));
                }
            });
        SplitPane::horizontal("split")
            .panel(SplitPanel::new("left"))
            .panel(SplitPanel::new("right"))
            .size(vec2(ui.available_width(), 65.0))
            .show(ui, |split| {
                split.panel("left", |ui| {
                    ui.label("Left");
                });
                split.panel("right", |ui| {
                    ui.label("Right");
                });
            });
    }
}
impl App for Themes {
    fn update(&mut self, c: &mut Context, _frame: &mut Frame<'_>) {
        let keys = &c.input().keys_pressed;
        for (key, mode) in [(KeyCode::F1, 0), (KeyCode::F2, 1), (KeyCode::F3, 2)] {
            if keys.contains(&key) {
                self.mode = mode;
            }
        }
        if keys.contains(&KeyCode::F4) {
            self.compact = !self.compact;
        }
        if keys.contains(&KeyCode::F5) {
            self.accent = (self.accent + 1) % 4;
        }
        if keys.contains(&KeyCode::F6) {
            self.glass = !self.glass;
        }
        let theme = self.theme();
        if self.animate {
            c.set_theme_animated(
                theme,
                TweenOptions::new(std::time::Duration::from_millis(180)),
            );
        } else {
            c.set_theme(theme);
        }
        Root::new().show(c, |ui| {
            ui.horizontal(|ui| {
                for (mode, label) in ["Dark", "Light", "High contrast"].into_iter().enumerate() {
                    if ui
                        .add(Button::new(label).selected(self.mode == mode))
                        .clicked()
                    {
                        self.mode = mode;
                    }
                }
                if ui.button("Accent").clicked() {
                    self.accent = (self.accent + 1) % 4;
                }
                ui.checkbox(&mut self.compact, "Compact");
                ui.checkbox(&mut self.glass, "Blur");
                ui.checkbox(&mut self.animate, "Animate");
            });
            ui.horizontal(|ui| {
                ui.vertical(|ui| ui.with_width(320.0, |ui| self.controls(ui)));
                ui.vertical(|ui| ui.with_width(330.0, |ui| self.colors(ui)));
                ui.fill(|ui| self.data(ui));
            });
        });
        Window::new("Window")
            .id(Id::new("floating-example"))
            .default_position(vec2(720.0, 705.0))
            .default_size(vec2(340.0, 125.0))
            .show(c, |ui| {
                ui.label("Floating window");
            });
    }
}
fn main() -> Result<(), RunError> {
    run_with_options(
        Themes::default(),
        RunOptions {
            window_attributes: winit::window::Window::default_attributes()
                .with_title("zaxis — Themes")
                .with_inner_size(LogicalSize::new(1140.0, 880.0)),
            ..Default::default()
        },
    )
}
