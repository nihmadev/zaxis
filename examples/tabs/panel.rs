//! The bottom panel: underlined tabs that select only on Enter, Space or a click.
use super::model::{Panel, Workspace};
use zaxis::{
    Button, ButtonVariant, Id, ScrollArea, TabActivation, TabBar, TabVariant, TextEdit, Ui,
};

pub fn show(ui: &mut Ui<'_>, work: &mut Workspace) {
    let mut clear = false;
    TabBar::new(
        "panel",
        &mut work.panel,
        [
            (Panel::Terminal, "Terminal"),
            (Panel::Problems, "Problems"),
            (Panel::Output, "Output"),
        ],
    )
    .variant(TabVariant::Underline)
    .activation(TabActivation::Manual)
    .trailing(|ui| {
        clear = ui
            .add(Button::new("Clear").variant(ButtonVariant::Ghost))
            .clicked();
    })
    .show(ui);
    if clear {
        match work.panel {
            Panel::Terminal => work.terminal.clear(),
            Panel::Output => work.log.clear(),
            Panel::Problems => work.save_all(),
        }
    }
    ui.add_space(4.0);
    match work.panel {
        Panel::Terminal => terminal(ui, work),
        Panel::Problems => problems(ui, work),
        Panel::Output => lines(ui, "output", &work.log),
    }
}

fn lines(ui: &mut Ui<'_>, id: &str, lines: &[String]) {
    ScrollArea::vertical()
        .id_source(id)
        .max_height(ui.available_height())
        .show(ui, |ui| {
            for line in lines {
                ui.label(line.as_str());
            }
        });
}

fn terminal(ui: &mut Ui<'_>, work: &mut Workspace) {
    let height = (ui.available_height() - 40.0).max(20.0);
    ScrollArea::vertical()
        .id_source("terminal")
        .max_height(height)
        .show(ui, |ui| {
            for line in &work.terminal {
                ui.label(line.as_str());
            }
        });
    let entry = ui.add(
        TextEdit::new(&mut work.command)
            .id_source("command")
            .monospace()
            .placeholder("$")
            .width(ui.available_width()),
    );
    if entry.submitted() {
        let line = std::mem::take(&mut work.command);
        work.run(line.trim());
        ui.context().request_focus(entry.id);
    }
}

fn problems(ui: &mut Ui<'_>, work: &mut Workspace) {
    let dirty: Vec<(u32, String)> = work
        .files
        .iter()
        .filter(|f| f.dirty())
        .map(|f| (f.id, f.name.clone()))
        .collect();
    let mut open = None;
    ScrollArea::vertical()
        .id_source("problems")
        .max_height(ui.available_height())
        .show(ui, |ui| {
            for (id, name) in &dirty {
                let row = Button::new(format!("{name}: unsaved changes"))
                    .id_source(Id::new(id))
                    .variant(ButtonVariant::Ghost);
                if ui.add(row).clicked() {
                    open = Some(*id);
                }
            }
        });
    if let Some(id) = open {
        work.open(id);
    }
}
