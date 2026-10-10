//! The window that rebinds commands: one key box per action, the clashes it makes, and a
//! way back to the defaults.

use zaxis::{vec2, Button, Chord, Context, Id, KeyBox, ScrollArea, Text, Ui, Window};

struct Row {
    id: Id,
    title: String,
    group: String,
    chord: Chord,
    changed: bool,
}

fn rows(ui: &mut Ui<'_>) -> Vec<Row> {
    let actions = ui.actions();
    actions
        .registry()
        .iter()
        .map(|action| Row {
            id: action.id(),
            title: action.title().to_owned(),
            group: action.group_name().to_owned(),
            chord: actions
                .keymap()
                .bindings_of(action.id())
                .first()
                .map(|binding| binding.chord.clone())
                .unwrap_or_default(),
            changed: actions.keymap().is_overridden(action.id()),
        })
        .collect()
}

/// The other action of a clash `id` is in, if any.
fn clash_of(ui: &mut Ui<'_>, id: Id) -> Option<Id> {
    ui.actions().keymap().conflicts().iter().find_map(|c| {
        if c.first == id {
            Some(c.second)
        } else if c.second == id {
            Some(c.first)
        } else {
            None
        }
    })
}

pub fn show(context: &mut Context, open: &mut bool) {
    if !*open {
        return;
    }
    let mut close = false;
    Window::new("Keyboard shortcuts")
        .default_position(vec2(60.0, 60.0))
        .default_size(vec2(520.0, 400.0))
        .show(context, |ui| {
            let rows = rows(ui);
            ScrollArea::vertical()
                .id_source("shortcuts")
                .max_height(ui.available_height() - 44.0)
                .show(ui, |ui| {
                    let mut group = "";
                    for row in &rows {
                        if row.group != group {
                            group = &row.group;
                            ui.heading(group);
                        }
                        let mut chord = row.chord.clone();
                        let response = ui.add(
                            KeyBox::chord(&mut chord, &row.title)
                                .sequence(true)
                                .id_source(row.id)
                                .button_size(vec2(160.0, 24.0)),
                        );
                        if response.changed() {
                            let chord = (!chord.is_empty()).then_some(chord);
                            ui.actions().keymap_mut().rebind(row.id, chord);
                        }
                        let other = clash_of(ui, row.id);
                        if other.is_some() || row.changed {
                            ui.horizontal(|ui| {
                                if let Some(other) = other {
                                    let title = ui
                                        .actions()
                                        .registry()
                                        .get(other)
                                        .map(|a| a.title().to_owned())
                                        .unwrap_or_default();
                                    let color = ui.style().warning;
                                    ui.add(Text::new(format!("also {title}")).color(color));
                                }
                                ui.spacer();
                                if row.changed
                                    && ui.add(Button::new("Reset").id_source(row.id)).clicked()
                                {
                                    ui.actions().keymap_mut().reset(row.id);
                                }
                            });
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.button("Reset all").clicked() {
                    ui.actions().keymap_mut().reset_all();
                }
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
        });
    *open &= !close;
}
