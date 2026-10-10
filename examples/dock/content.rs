use super::{tree::Files, workspace::panel};
use std::collections::HashSet;
use zaxis::{
    Button, DockViewer, Id, ImageSource, PanelId, ScrollArea, Slider, TextEdit, TreeEvent,
    TreeView, Ui,
};
const FILENAMES: [&str; 5] = [
    "main.rs",
    "layout.rs",
    "input.rs",
    "theme.rs",
    "renderer.rs",
];
pub struct Content {
    text: String,
    files: Files,
    open: HashSet<Id>,
    selected: Option<Id>,
    current: usize,
    tasks: [bool; 8],
    gain: f32,
    logs: Vec<String>,
    scratch: String,
}
impl Content {
    pub fn new() -> Self {
        Self {
            text: "fn main() {\n    let workspace = DockState::new(layout);\n    run(Editor { workspace })\n}\n".into(),
            files: Files,
            open: HashSet::from([Id::new("src")]),
            selected: None,
            current: 0,
            tasks: [false; 8],
            gain: 0.6,
            logs: vec!["Workspace opened".into()],
            scratch: "Ideas\n\n".into(),
        }
    }
}
impl DockViewer for Content {
    fn title(&self, p: &PanelId) -> String {
        let n = (0..64).find(|n| panel(*n) == *p).unwrap_or(64);
        match n {
            0 => "Files".into(),
            1 => FILENAMES[self.current].into(),
            2 => "Tasks".into(),
            3 => "Inspector".into(),
            4 => "Outline".into(),
            5 => "History".into(),
            6 => "Notes".into(),
            _ => format!("View {}", n + 1),
        }
    }
    fn icon(&self, _: &PanelId) -> Option<ImageSource> {
        Some(ImageSource::bytes(br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M5 4h14v16H5zM8 9h8M8 13h8" fill="none" stroke="#fff" stroke-width="2"/></svg>"##))
    }
    fn ui(&mut self, ui: &mut Ui<'_>, p: &PanelId) {
        if *p == panel(0) {
            ScrollArea::vertical().show(ui, |ui| {
                for (i, name) in FILENAMES.iter().enumerate() {
                    if ui
                        .add(Button::new(*name).selected(self.current == i))
                        .clicked()
                    {
                        self.current = i;
                        self.text = format!("// {name}\n\nfn main() {{\n}}\n");
                        self.logs.push(format!("Opened {name}"));
                    }
                }
            });
        } else if *p == panel(1) {
            ui.add(
                TextEdit::new(&mut self.text)
                    .id_source("editor")
                    .multiline()
                    .monospace()
                    .wrap(false)
                    .fill_width()
                    .height(ui.available_height().max(1.0)),
            );
        } else if *p == panel(2) {
            ScrollArea::vertical().show(ui, |ui| {
                for (i, done) in self.tasks.iter_mut().enumerate() {
                    if ui.checkbox(done, format!("Task {}", i + 1)).changed() {
                        self.logs.push(format!(
                            "Task {} {}",
                            i + 1,
                            if *done { "completed" } else { "reopened" }
                        ));
                    }
                }
            });
        } else if *p == panel(3) {
            ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    Slider::new(&mut self.gain, 0.0..=1.0)
                        .width(ui.available_width())
                        .text("Gain"),
                );
                let rect = ui.allocate_space(zaxis::vec2(ui.available_width(), 120.0));
                let color = ui.style().accent;
                for i in 0..32 {
                    let point = |n: usize| {
                        rect.min
                            + zaxis::vec2(
                                rect.size().x * n as f32 / 32.0,
                                rect.size().y * (0.5 - 0.4 * self.gain * (n as f32 * 0.4).sin()),
                            )
                    };
                    ui.paint(zaxis::Shape::Line {
                        start: point(i),
                        end: point(i + 1),
                        width: 2.0,
                        color,
                    });
                }
                if ui.button("Apply").clicked() {
                    self.logs.push(format!("Gain {:.2}", self.gain));
                }
            });
        } else if *p == panel(4) {
            let out = TreeView::new("files-tree")
                .accessible_label("Source files")
                .open(&mut self.open)
                .selected(&mut self.selected)
                .max_height(ui.available_height())
                .show(ui, &self.files);
            for event in out.events {
                if let TreeEvent::Activated { node } = event {
                    let name = self.files.name(node);
                    if let Some(index) = FILENAMES.iter().position(|file| *file == name) {
                        self.current = index;
                        self.text = format!("// {name}\n\nfn main() {{\n}}\n");
                        self.logs.push(format!("Opened {name}"));
                    }
                }
            }
        } else if *p == panel(5) {
            ScrollArea::vertical().show(ui, |ui| {
                for line in self.logs.iter().rev() {
                    ui.label(line);
                }
            });
        } else if *p == panel(6) {
            ui.add(
                TextEdit::new(&mut self.scratch)
                    .id_source("notes")
                    .multiline()
                    .fill_width()
                    .height(ui.available_height().max(1.0)),
            );
        } else {
            let mut value = self.tasks.iter().filter(|v| **v).count() as f32 / 8.0;
            if ui
                .add(Slider::new(&mut value, 0.0..=1.0).width(ui.available_width()))
                .changed()
            {
                let count = (value * 8.0).round() as usize;
                for (i, done) in self.tasks.iter_mut().enumerate() {
                    *done = i < count;
                }
            }
        }
        if self.logs.len() > 100 {
            self.logs.drain(..self.logs.len() - 100);
        }
    }
}
