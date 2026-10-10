//! The file list on the left: a click opens the file in the focused group.
use super::model::Workspace;
use zaxis::{Button, ButtonVariant, Padding, ScrollArea, Ui};

pub fn show(ui: &mut Ui<'_>, work: &mut Workspace) {
    let active = work.active();
    let mut open = None;
    ScrollArea::vertical()
        .id_source("explorer")
        .max_height(ui.available_height())
        .show(ui, |ui| {
            ui.collapsing("project", "zaxis", |ui| {
                for file in &work.files {
                    let row = Button::new(file.name.as_str())
                        .id_source(file.id)
                        .icon(file.icon())
                        .variant(ButtonVariant::Ghost)
                        .selected(active == Some(file.id))
                        .enabled(!file.locked)
                        .padding(Padding::symmetric(8.0, 4.0))
                        .min_size(zaxis::vec2(ui.available_width(), 0.0));
                    if ui.add(row).clicked() {
                        open = Some(file.id);
                    }
                }
            });
        });
    if let Some(id) = open {
        work.open(id);
    }
}
