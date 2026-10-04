//! The content of each window, drawn from the shared application data.

use super::{Confirms, Data};
use crate::{open_animation, open_notes, open_palette, open_settings};
use zaxis::{
    Button, CollapsingHeader, Confirm, Confirmation, Context, Frame, Root, Switch, Theme, Ui,
};

/// A confirmation that closes `frame`'s window once the user agrees to discard changes.
fn discard(ui: &mut Ui<'_>, frame: &mut Frame<'_>, id: &str, open: &mut bool) {
    let answer = Confirm::new(id)
        .title("Discard unsaved changes?")
        .description("Closing now loses the edits made since the last save.")
        .confirm_label("Discard")
        .danger()
        .show(ui, open);
    if answer == Some(Confirmation::Confirmed) {
        frame.close_this_window();
    }
}

pub fn main(
    context: &mut Context,
    frame: &mut Frame<'_>,
    data: &mut Data,
    confirms: &mut Confirms,
) {
    Root::new().show(context, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Inspector").clicked() {
                data.show_inspector = !data.show_inspector;
            }
            if ui.button("Palette").clicked() {
                open_palette(frame);
            }
            if ui.button("Notes").clicked() {
                open_notes(frame);
            }
            if ui.button("Animation").clicked() {
                open_animation(frame);
            }
            if ui.button("Settings").clicked() {
                open_settings(frame);
            }
        });
        ui.separator();
        ui.text_area(&mut data.doc);
        ui.horizontal(|ui| {
            if ui
                .add(Button::new("Save").enabled(data.doc_dirty()))
                .clicked()
            {
                data.saved_doc = data.doc.clone();
            }
            ui.muted(if data.doc_dirty() {
                "Unsaved changes"
            } else {
                "Saved"
            });
        });
        discard(ui, frame, "discard-document", &mut confirms.main);
    });
}

pub fn inspector(context: &mut Context, frame: &mut Frame<'_>, data: &mut Data) {
    Root::new().show(context, |ui| {
        ui.title("Document");
        ui.label(format!("{} characters", data.doc.chars().count()));
        ui.label(format!("{} lines", data.doc.lines().count()));
        ui.label(format!("{} words", data.doc.split_whitespace().count()));
        ui.separator();
        CollapsingHeader::new("window", "This window").show(ui, |ui| {
            let info = frame.info();
            ui.label(format!("{} × {} px", info.size.width, info.size.height));
            ui.label(format!("scale {:.2}", info.scale_factor));
            ui.label(format!("focused: {}", info.focused));
            ui.label(format!("windows open: {}", frame.stats().windows_open));
        });
    });
}

pub fn palette(context: &mut Context, data: &mut Data) {
    Root::new().show(context, |ui| {
        ui.title("Insert");
        for snippet in ["TODO: ", "FIXME: ", "NOTE: "] {
            if ui.button(snippet.trim_end()).clicked() {
                data.doc.push_str(snippet);
            }
        }
        if ui.button("Clear").clicked() {
            data.doc.clear();
        }
    });
}

pub fn settings(context: &mut Context, frame: &mut Frame<'_>, data: &mut Data) {
    Root::new().show(context, |ui| {
        ui.title("Appearance");
        if ui
            .add(Switch::new(&mut data.light, "Light theme"))
            .changed()
        {
            // One call re-themes every window, including this one.
            frame.windows().set_theme(if data.light {
                Theme::light()
            } else {
                Theme::dark()
            });
        }
    });
}

pub fn notes(
    context: &mut Context,
    frame: &mut Frame<'_>,
    data: &mut Data,
    confirms: &mut Confirms,
) {
    Root::new().show(context, |ui| {
        ui.text_area(&mut data.notes);
        ui.horizontal(|ui| {
            if ui
                .add(Button::new("Save").enabled(data.notes_dirty()))
                .clicked()
            {
                data.saved_notes = data.notes.clone();
            }
            if ui.button("Close").clicked() {
                frame.request_close();
            }
        });
        CollapsingHeader::new("details", "Details").show(ui, |ui| {
            ui.label(format!("{} characters", data.notes.chars().count()));
        });
        discard(ui, frame, "discard-notes", &mut confirms.notes);
    });
}

/// Animates continuously while the windows around it stay at rest.
pub fn animation(context: &mut Context) {
    Root::new().show(context, |ui| {
        ui.loader(true);
    });
}
