use zaxis::{
    vec2, App, Button, Column, Context, Frame, Grid, GridStyle, Id, Padding, SortDirection, Table,
    TextEdit, Window,
};

struct Record {
    id: u64,
    name: String,
    enabled: bool,
}
struct Example {
    name: String,
    notifications: bool,
    volume: f32,
    records: Vec<Record>,
    smoke: bool,
    passes: usize,
}
impl App for Example {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        Window::new("Settings and records")
            .default_size(vec2(870.0, 690.0))
            .min_size(vec2(420.0, 400.0))
            .show(context, |ui| {
                ui.muted("Shared columns · stable row IDs · drag header edges to resize");
                Grid::new("cards")
                    .columns([
                        Column::remainder("general").min_width(150.0),
                        Column::remainder("sound").min_width(150.0),
                    ])
                    .padding(Padding::all(0.0))
                    .cell_padding(Padding::all(0.0))
                    .fill(zaxis::Color::TRANSPARENT)
                    .show(ui, |cards| {
                        cards.row("settings", |row| {
                            row.cell(|ui| {
                                Grid::new("general")
                                    .columns([Column::content("label"), Column::remainder("value")])
                                    .show(ui, |grid| {
                                        grid.row("title", |row| {
                                            row.cell(|ui| {
                                                ui.heading("General");
                                            });
                                        });
                                        grid.row("name", |row| {
                                            row.cell(|ui| {
                                                ui.label("Profile");
                                            });
                                            row.cell(|ui| {
                                                let width = ui.available_width().max(1.0);
                                                ui.add(TextEdit::new(&mut self.name).width(width));
                                            });
                                        });
                                        grid.row("notifications", |row| {
                                            row.cell(|ui| {
                                                ui.label("Alerts");
                                            });
                                            row.cell(|ui| {
                                                ui.checkbox(&mut self.notifications, "Enabled");
                                            });
                                        });
                                    });
                            });
                            row.cell(|ui| {
                                Grid::new("sound")
                                    .columns([Column::content("label"), Column::remainder("value")])
                                    .style(GridStyle {
                                        cell_padding: Padding::symmetric(8.0, 4.0),
                                        ..ui.style().grid
                                    })
                                    .show(ui, |grid| {
                                        grid.row("title", |row| {
                                            row.cell(|ui| {
                                                ui.heading("Sound");
                                            });
                                        });
                                        grid.row("volume", |row| {
                                            row.cell(|ui| {
                                                ui.label("Volume");
                                            });
                                            row.cell(|ui| {
                                                ui.slider(&mut self.volume, 0.0..=100.0);
                                            });
                                        });
                                        grid.row("actions", |row| {
                                            row.cell(|ui| {
                                                ui.muted("Preview");
                                            });
                                            row.cell(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.button("Play");
                                                    ui.button("Stop");
                                                });
                                            });
                                        });
                                    });
                            });
                        })
                    });
                ui.heading("Records");
                let height = (ui.available_height() - 32.0).max(0.0);
                let output = Table::new("records")
                    .max_height(height)
                    .separators(true)
                    .columns([
                        Column::fixed("id", 76.0)
                            .min_width(48.0)
                            .title("ID")
                            .sortable(true),
                        Column::remainder("name")
                            .min_width(130.0)
                            .title("Name")
                            .sortable(true),
                        Column::fixed("enabled", 110.0)
                            .min_width(80.0)
                            .title("Enabled"),
                        Column::fixed("action", 92.0)
                            .min_width(70.0)
                            .title("Action"),
                    ])
                    .show_rows(ui, 36.0, self.records.len(), |body, index| {
                        let record = &mut self.records[index];
                        body.row(record.id, |row| {
                            row.cell(|ui| {
                                ui.label(record.id.to_string());
                            });
                            row.cell(|ui| {
                                ui.label(&record.name);
                            });
                            row.cell(|ui| {
                                ui.checkbox(&mut record.enabled, "Active");
                            });
                            row.cell(|ui| {
                                if ui
                                    .add(
                                        Button::new("Toggle").padding(Padding::symmetric(8.0, 2.0)),
                                    )
                                    .clicked()
                                {
                                    record.enabled = !record.enabled;
                                }
                            });
                        });
                    });
                if let Some(sort) = output.sort_request {
                    self.records.sort_by(|a, b| {
                        let order = if sort.column == Id::new("name") {
                            a.name.cmp(&b.name)
                        } else {
                            a.id.cmp(&b.id)
                        };
                        if sort.direction == SortDirection::Ascending {
                            order
                        } else {
                            order.reverse()
                        }
                    });
                }
                ui.muted(format!(
                    "Built {} of {} rows · selected {:?}",
                    output.inner.len(),
                    self.records.len(),
                    output.selected_row
                ));
            });
        if self.smoke {
            self.passes += 1;
            if self.passes >= 4 {
                frame.close();
            } else {
                context.request_repaint();
            }
        }
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Example {
            name: "Personal".into(),
            notifications: true,
            volume: 65.0,
            records: (0..10_000)
                .map(|id| Record {
                    id,
                    name: format!("Record {id:05}"),
                    enabled: id % 3 != 0,
                })
                .collect(),
            smoke: std::env::args().any(|arg| arg == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            presentation_mode: zaxis::PresentationMode::Immediate,
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Grid and Table")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(970.0, 790.0)),
            ..Default::default()
        },
    )
}
