//! The drop zone, the list of accepted files and the preview.
use super::model::{human_size, kind, Action, Model, Preview};
use zaxis::{
    vec2, Card, Context, DropTarget, FileFilter, Id, Image, ImageFit, Padding, ScrollArea,
    TextEdit, Ui,
};

pub fn toolbar(ui: &mut Ui<'_>, model: &mut Model) {
    let mut chosen = None;
    ui.horizontal(|ui| {
        for action in Action::ALL {
            let button = ui.button(action.label());
            if button.clicked() && !model.dialog_open(action) {
                chosen = Some(action);
            }
        }
    });
    if let Some(action) = chosen {
        model.open_dialog(ui.context(), action);
    }
}

/// A zone that takes files and folders from the file manager. It highlights while they are
/// dragged over it, in the same way an in-window drag does.
pub fn drop_zone(ui: &mut Ui<'_>, c_model: &mut Model) {
    let hovering = !ui.context().hovered_files().is_empty();
    let out = DropTarget::files(Id::new("zone"))
        .accepts_files(FileFilter::any().directories(true))
        .show(ui, |ui| {
            Card::new("zone-card")
                .padding(Padding::all(28.0))
                .show(ui, |ui| {
                    centered(ui, |ui| upload_icon(ui, hovering));
                    centered(ui, |ui| {
                        ui.label(if hovering {
                            "Release to add"
                        } else {
                            "Drop files or folders here"
                        });
                    });
                });
        });
    if !out.dropped_files.is_empty() {
        c_model.add(ui.context(), out.dropped_files);
    }
}

pub fn list(ui: &mut Ui<'_>, model: &mut Model) {
    let mut chosen = None;
    ScrollArea::vertical()
        .id_source("files")
        .max_height(160.0)
        .show(ui, |ui| {
            for (index, file) in model.files.iter().enumerate() {
                ui.horizontal(|ui| {
                    let name = if file.name().is_empty() {
                        "(unnamed)"
                    } else {
                        file.name()
                    };
                    if ui.button(name).clicked() {
                        chosen = Some(index);
                    }
                    ui.label(format!("{}  {}", human_size(file), kind(file)));
                });
            }
        });
    if let Some(index) = chosen {
        model.select(ui.context(), index);
    }
}

pub fn preview(ui: &mut Ui<'_>, model: &mut Model) {
    if let Some(error) = &model.error {
        ui.label(error.clone());
    }
    match &model.preview {
        Preview::None | Preview::Loading(..) => {}
        Preview::Folder => {
            ui.label("A folder. The library lists nothing inside it.");
        }
        Preview::Failed(reason) => {
            ui.label(reason.clone());
        }
        Preview::Image(source) => {
            ui.add(
                Image::new(source.clone())
                    .max_size(vec2(420.0, 260.0))
                    .fit(ImageFit::Contain)
                    .corner_radius(8.0),
            );
        }
        Preview::Text => {
            ui.add(
                TextEdit::new(&mut model.text)
                    .id_source("text")
                    .multiline()
                    .rows(8.0),
            );
        }
    }
}

pub fn pump(c: &mut Context, model: &mut Model) {
    model.pump(c);
}

/// One row with its content in the middle of the zone.
fn centered(ui: &mut Ui<'_>, content: impl FnOnce(&mut Ui<'_>)) {
    ui.horizontal(|ui| {
        ui.spacer();
        content(ui);
        ui.spacer();
    });
}

/// The upload icon, shown only while files are being dragged over the window, in the accent
/// color. The row keeps its height when idle so the zone does not jump. It needs the
/// `bundled-icons` feature; without it the zone shows only its text and outline.
#[cfg(feature = "bundled-icons")]
fn upload_icon(ui: &mut Ui<'_>, hovering: bool) {
    if !hovering {
        ui.allocate_space(vec2(32.0, 32.0));
        return;
    }
    let tint = ui.style().accent;
    ui.add(
        Image::new(&zaxis::icons::UPLOAD)
            .size(vec2(32.0, 32.0))
            .tint(tint),
    );
}

#[cfg(not(feature = "bundled-icons"))]
fn upload_icon(_ui: &mut Ui<'_>, _hovering: bool) {}
