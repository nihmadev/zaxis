use super::{Model, CATEGORIES};
use zaxis::{
    Align, Button, ButtonVariant, CloseReason, ComboBox, ComboBoxOption, Confirm, Confirmation,
    Dialog, DialogAction, Modal, ModalAnchor, TextEdit, Ui,
};

pub fn begin_edit(model: &mut Model, id: Option<u32>) {
    let Some(item) = id.and_then(|id| model.items.iter().find(|item| item.id == id)) else {
        return;
    };
    model.draft_name = item.name.clone();
    model.draft_category = Some(item.category);
    model.target = Some(item.id);
    model.edit_open = true;
}

pub fn list(ui: &mut Ui<'_>, model: &mut Model) {
    let mut edit = None;
    let mut delete = None;
    for item in &model.items {
        ui.push_id(item.id, |ui| {
            ui.horizontal_aligned(Align::Center, |ui| {
                ui.label(format!("{}  ·  {}", item.name, CATEGORIES[item.category]));
                ui.spacer();
                if ui.button("Edit").clicked() {
                    edit = Some(item.id);
                }
                if ui
                    .add(Button::new("Delete").variant(ButtonVariant::Soft))
                    .clicked()
                {
                    delete = Some(item.id);
                }
            });
        });
    }
    if model.items.is_empty() {
        ui.muted("Nothing left.");
    }
    if let Some(id) = edit {
        begin_edit(model, Some(id));
    }
    if let Some(id) = delete {
        model.target = Some(id);
        model.delete_open = true;
    }
}

pub fn dialogs(ui: &mut Ui<'_>, model: &mut Model) {
    delete(ui, model);
    edit(ui, model);
    terms(ui, model);
    nested(ui, model);
    danger(ui, model);
    sheet(ui, model);
}

fn target_name(model: &Model) -> String {
    model
        .target
        .and_then(|id| model.items.iter().find(|item| item.id == id))
        .map_or_else(String::new, |item| item.name.clone())
}

/// Plain confirmation: Escape cancels, the overlay does not close it.
fn delete(ui: &mut Ui<'_>, model: &mut Model) {
    let name = target_name(model);
    let confirm = Confirm::new("delete-item")
        .title(format!("Delete \"{name}\"?"))
        .description("The item is removed from the list. This cannot be undone.")
        .confirm_label("Delete")
        .danger()
        .dismiss_on_escape(true);
    match confirm.show(ui, &mut model.delete_open) {
        Some(Confirmation::Confirmed) => {
            if let Some(id) = model.target.take() {
                model.items.retain(|item| item.id != id);
                model.status = format!("Deleted \"{name}\".");
            }
        }
        Some(Confirmation::Cancelled(reason)) => {
            model.status = format!("Delete cancelled ({reason:?}).");
        }
        None => {}
    }
}

/// A form: TextEdit and ComboBox inside the body; Save writes the draft back.
fn edit(ui: &mut Ui<'_>, model: &mut Model) {
    let options: Vec<_> = CATEGORIES
        .iter()
        .enumerate()
        .map(|(i, name)| ComboBoxOption::new(i, i, *name))
        .collect();
    let (name, category) = (&mut model.draft_name, &mut model.draft_category);
    let output = Dialog::new("edit-item", "Edit item")
        .description("Rename the item and choose where it belongs.")
        .action(DialogAction::new("Cancel"))
        .action(DialogAction::new("Save").primary())
        .show(ui, &mut model.edit_open, |ui| {
            ui.add(TextEdit::new(name).id_source("name").width(300.0));
            ui.add(
                ComboBox::new(category, &options)
                    .id_source("category")
                    .label("Category")
                    .width(300.0),
            );
        });
    if let Some(output) = output {
        let saved = output.action == Some(1);
        if saved && !model.draft_name.trim().is_empty() {
            let (target, name) = (model.target, model.draft_name.trim().to_owned());
            if let Some(item) = model.items.iter_mut().find(|item| Some(item.id) == target) {
                item.name = name;
                item.category = model.draft_category.unwrap_or(item.category);
                model.status = format!("Saved \"{}\".", item.name);
            }
        }
    }
}

/// Long body: it scrolls while the title and the action stay in place.
fn terms(ui: &mut Ui<'_>, model: &mut Model) {
    let output = Dialog::new("terms", "Terms of use")
        .description("Scroll to read all sections.")
        .action(DialogAction::new("Decline"))
        .action(DialogAction::new("Accept").primary())
        .show(ui, &mut model.long_open, |ui| {
            for section in 1..=24 {
                ui.heading(format!("Section {section}"));
                ui.muted(
                    "The parties agree that the software is provided as is and that                      these terms may be updated from time to time.",
                );
            }
        });
    if output.is_some_and(|output| output.action == Some(1)) {
        model.status = "Terms accepted.".into();
    }
}

/// A second dialog opened from inside the first; Escape closes one at a time.
fn nested(ui: &mut Ui<'_>, model: &mut Model) {
    let (advanced, notifications) = (&mut model.advanced_open, &mut model.notifications);
    Dialog::new("project", "Project settings")
        .action(DialogAction::new("Close"))
        .show(ui, &mut model.nested_open, |ui| {
            ui.checkbox(notifications, "Notify me about changes");
            if ui.button("Advanced…").clicked() {
                *advanced = true;
            }
            Dialog::new("advanced", "Advanced")
                .description("Settings for experienced users.")
                .action(DialogAction::new("Done").primary())
                .show(ui, advanced, |ui| {
                    ui.label("Nothing dangerous here.");
                });
        });
}

/// Closes only by an explicit choice: not by Escape, not by the overlay.
fn danger(ui: &mut Ui<'_>, model: &mut Model) {
    let confirm = Confirm::new("reset")
        .title("Delete all items?")
        .description("Every item is removed. This cannot be undone.")
        .confirm_label("Delete everything")
        .danger();
    match confirm.show(ui, &mut model.danger_open) {
        Some(Confirmation::Confirmed) => {
            model.items.clear();
            model.status = "All items deleted.".into();
        }
        Some(Confirmation::Cancelled(CloseReason::Action)) => {
            model.status = "Nothing was deleted.".into();
        }
        _ => {}
    }
}

/// The same modal anchored to an edge: a sheet that slides in.
fn sheet(ui: &mut Ui<'_>, model: &mut Model) {
    let count = model.items.len();
    Modal::new("sheet")
        .anchor(ModalAnchor::Right)
        .width(320.0)
        .show(ui, &mut model.sheet_open, |ui| {
            ui.heading("Summary");
            ui.muted(format!("{count} items"));
            for item in 0..count {
                ui.label(format!("Item {}", item + 1));
            }
        });
}
