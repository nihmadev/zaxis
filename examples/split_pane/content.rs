use super::{Panel, Workspace};
use zaxis::{vec2, Button, Column, Image, Padding, ScrollArea, Table, TextEdit, Ui};

const FILES: [&str; 8] = [
    "main.rs",
    "editor.rs",
    "layout.rs",
    "input.rs",
    "renderer.rs",
    "style.rs",
    "landscape.jpg",
    "icon.svg",
];
impl Workspace {
    pub(super) fn content(&mut self, ui: &mut Ui<'_>, panel: Panel) {
        match panel {
            Panel::Files => {
                ScrollArea::vertical().show(ui, |ui| {
                    for (i, file) in FILES.iter().enumerate() {
                        if ui
                            .add(
                                Button::new(*file)
                                    .selected(self.file == i)
                                    .padding(Padding::symmetric(8.0, 5.0)),
                            )
                            .clicked()
                        {
                            self.file = i;
                            self.text = (*file).into();
                            self.logs.push(format!("Opened {file}"));
                        }
                    }
                });
            }
            Panel::Editor => {
                ui.add(TextEdit::new(&mut self.text).width(ui.available_width().max(1.0)));
                ScrollArea::vertical().show(ui, |ui| {
                    for line in [
                        "fn main() {",
                        "    let width = 180.0;",
                        "    let split = SplitPane::horizontal(\"app\");",
                        "}",
                    ] {
                        ui.label(line);
                    }
                });
            }
            Panel::Tasks => {
                Table::new("tasks")
                    .max_height(ui.available_height())
                    .columns([
                        Column::fixed("id", 40.0).title("ID"),
                        Column::remainder("task").title("Action"),
                    ])
                    .show_rows(ui, 32.0, self.runs.len(), |body, i| {
                        body.row(i, |row| {
                            row.cell(|ui| {
                                ui.label(i.to_string());
                            });
                            row.cell(|ui| {
                                if ui.button(format!("Run {}##task", self.runs[i])).clicked() {
                                    self.runs[i] += 1;
                                    self.logs.push(format!("Task {i} completed"));
                                }
                            });
                        });
                    });
            }
            Panel::Log => {
                ScrollArea::vertical().show(ui, |ui| {
                    for line in self.logs.iter().rev().take(100) {
                        ui.label(line);
                    }
                });
            }
            Panel::Inspector => {
                ui.checkbox(&mut self.glass, "Blur");
                ui.slider_labeled(&mut self.zoom, 0.5..=2.0, "Zoom");
                self.image(ui);
                if ui.button("Apply").clicked() {
                    self.logs.push(format!("Saved {}", self.text));
                }
            }
            Panel::Preview => {
                self.image(ui);
            }
            Panel::Work => {}
        }
    }
    fn image(&self, ui: &mut Ui<'_>) {
        let bytes: &'static [u8] = if self.file == 7 {
            include_bytes!("../../assets/images/icon.svg")
        } else {
            include_bytes!("../../assets/images/landscape.jpg")
        };
        ui.add(Image::new(bytes).size(vec2(ui.available_width() * self.zoom, 160.0 * self.zoom)));
    }
}
