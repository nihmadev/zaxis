//! One editor group: a tab strip over the text of its active file. Both groups share a
//! drag group, so a tab can be dragged between them, onto the other group's text, or out
//! of both into a window of its own.
use super::model::{Group, Workspace};
use zaxis::{
    icons, Button, ButtonVariant, Id, TabBar, TabDropZone, TabItem, TabMove, TextEdit, Ui, Vec2,
};

#[derive(Default)]
pub struct Pending {
    pub create: Option<usize>,
    pub close: Option<(usize, Id)>,
    pub moved: Option<TabMove>,
    pub torn: Option<(usize, Id, Vec2)>,
    pub focus: Option<usize>,
    pub send: Option<usize>,
}

fn items(work: &Workspace, group: &Group) -> Vec<TabItem<u32>> {
    group
        .tabs
        .iter()
        .filter_map(|id| work.file(*id))
        .map(|file| {
            // A pinned file is an icon alone; an unsaved one carries a dot.
            let label = match (file.pinned, file.dirty()) {
                (true, _) => String::new(),
                (false, true) => format!("{} ●", file.name),
                (false, false) => file.name.clone(),
            };
            let tip = if file.dirty() {
                format!("{} (unsaved)", file.name)
            } else {
                file.name.clone()
            };
            TabItem::new(file.id, label)
                .icon(file.icon())
                .closable(!file.pinned)
                .enabled(!file.locked)
                .tooltip(tip)
        })
        .collect()
}

pub fn show(ui: &mut Ui<'_>, work: &mut Workspace, index: usize, pending: &mut Pending) {
    let tabs = items(work, &work.groups[index]);
    let name = work.groups[index].name;
    let mut active = work.groups[index].active;
    let (mut create, mut send) = (false, false);
    let out = TabBar::new(name, &mut active, tabs)
        .closable(true)
        .reorderable(true)
        .group("editors")
        .overflow_menu(true)
        .trailing(|ui| {
            ui.horizontal(|ui| {
                send = ui
                    .add(
                        Button::new("")
                            .icon(&icons::ARROW_LEFT_RIGHT)
                            .variant(ButtonVariant::Ghost),
                    )
                    .clicked();
                create = ui
                    .add(
                        Button::new("")
                            .icon(&icons::PLUS)
                            .variant(ButtonVariant::Ghost),
                    )
                    .clicked();
            });
        })
        .show(ui);
    work.groups[index].active = active;
    if create {
        pending.create = Some(index);
    }
    if send {
        pending.send = Some(index);
    }
    if out.strip.changed() || out.responses.iter().any(|r| r.clicked()) {
        pending.focus = Some(index);
    }
    pending.close = pending.close.or(out.closed.map(|key| (index, key)));
    pending.moved = pending.moved.or(out.moved);
    pending.torn = pending
        .torn
        .or(out.released_outside.map(|r| (index, r.tab, r.position)));

    let height = ui.available_height();
    let width = ui.available_width();
    let zone = TabDropZone::new(("body", index), name, "editors").show(ui, |ui| {
        let id = work.groups[index].active;
        let open = work.groups[index].tabs.contains(&id);
        match work.file_mut(id) {
            Some(file) if open => {
                let edit = ui.add(
                    TextEdit::new(&mut file.text)
                        .id_source(("text", id))
                        .multiline()
                        .monospace()
                        .width(width)
                        .height(height),
                );
                if edit.gained_focus() {
                    pending.focus = Some(index);
                }
            }
            _ => {
                ui.allocate_space(zaxis::vec2(width, height));
            }
        }
    });
    pending.moved = pending.moved.or(zone.moved);
}
