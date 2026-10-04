//! The three panels: task lists, scene tree and table.
use crate::model::{Data, FileDrag, Objects, Task, TaskDrag, TaskMove, FILES};
use std::collections::HashSet;
use zaxis::{
    vec2, Button, Column, DragEffect, DragSource, DropTarget, DropZones, Id, Insertion, Rect,
    Reorder, RowMove, ScrollArea, Table, TreeEvent, TreeView, Ui,
};

pub struct ListOut {
    pub moves: Vec<TaskMove>,
    pub first: Option<Rect>,
    pub rect: Rect,
}

/// A reorderable list. Rows are sources and before/after targets; the list
/// itself accepts a task dropped anywhere else and appends it.
pub fn task_list(ui: &mut Ui<'_>, key: &'static str, tasks: &[Task]) -> ListOut {
    let mut moves = Vec::new();
    let mut first = None;
    let list = ui.drop_target(
        Id::new(key),
        |_: &TaskDrag| true,
        |ui| {
            ScrollArea::vertical()
                .id_source(key)
                .max_height(230.0)
                .show(ui, |ui| {
                    Reorder::new(key).show(ui, tasks.iter().map(|t| Id::new(t.id)), |r| {
                        for task in tasks {
                            let id = Id::new(task.id);
                            r.item(id, |ui| {
                                let width = ui.available_width();
                                let row = DropTarget::new(id, |_: &TaskDrag| true)
                                    .zones(DropZones::rows())
                                    .show(ui, |ui| {
                                        ui.drag_source(id, TaskDrag(task.id), |ui| {
                                            ui.add(
                                                Button::new(task.title.as_str())
                                                    .min_size(vec2(width, 26.0)),
                                            )
                                        })
                                    });
                                first.get_or_insert(row.response.rect);
                                if let Some(drop) = row.dropped {
                                    moves.push(TaskMove {
                                        task: drop.payload.0,
                                        list: key,
                                        anchor: Some((
                                            task.id,
                                            drop.insertion.unwrap_or(Insertion::After),
                                        )),
                                    });
                                }
                            });
                        }
                    });
                });
        },
    );
    if let Some(drop) = list.dropped {
        moves.push(TaskMove {
            task: drop.payload.0,
            list: key,
            anchor: None,
        });
    }
    ListOut {
        moves,
        first,
        rect: list.response.rect,
    }
}

/// File chips with a custom preview, and an archive that only accepts files:
/// dragging a task over it shows the rejection.
pub fn archive(ui: &mut Ui<'_>, archived: &mut Vec<usize>) {
    ui.horizontal(|ui| {
        for (n, name) in FILES.iter().enumerate() {
            DragSource::new(Id::new(("file", n)), FileDrag(n))
                .effect(DragEffect::Copy)
                .preview(move |ui| {
                    ui.label(*name);
                })
                .show(ui, |ui| ui.button(*name));
        }
    });
    let count = archived.len();
    let out = ui.drop_target(
        Id::new("archive"),
        |file: &FileDrag| !archived.contains(&file.0),
        |ui| {
            ui.label(format!("Archive (files only): {count}"));
            ui.allocate_space(vec2(ui.available_width(), 26.0));
        },
    );
    if let Some(drop) = out.dropped {
        archived.push(drop.payload.0);
    }
}

pub fn tree(
    ui: &mut Ui<'_>,
    objects: &Objects,
    open: &mut HashSet<Id>,
    selected: &mut Option<Id>,
) -> (Vec<TreeEvent>, Rect) {
    let out = TreeView::new("scene")
        .open(open)
        .selected(selected)
        .drag_nodes(true)
        .max_height(ui.available_height())
        .show(ui, objects);
    (out.events, out.viewport)
}

/// Virtualized table with draggable rows that also accepts tasks.
pub fn table(ui: &mut Ui<'_>, data: &mut Data) {
    if let Some(dragged) = ui.dragging() {
        if data.rows.iter().any(|r| Id::new(*r) == dragged) {
            // The row may scroll out of the virtual window; it still exists.
            ui.keep_drag_source(dragged);
        }
    }
    let rows = &data.rows;
    let out = ui.drop_target(
        Id::new("table"),
        |_: &TaskDrag| true,
        |ui| {
            Table::new("rows")
                .columns([
                    Column::remainder("name").min_width(80.0).title("Row"),
                    Column::fixed("id", 70.0).title("Id"),
                ])
                .max_height(300.0)
                .drag_rows(true)
                .show_rows(ui, 26.0, rows.len(), |body, i| {
                    let row = rows[i];
                    body.row_id(Id::new(row), |r| {
                        r.cell(|ui| {
                            ui.label(format!("Row {row}"));
                        });
                        r.cell(|ui| {
                            ui.label(row.to_string());
                        });
                    });
                })
        },
    );
    let moved: Option<RowMove> = out.inner.row_moved;
    let task = out.dropped.map(|d| d.payload.0);
    if let Some(m) = moved {
        let find = |id: Id| data.rows.iter().copied().find(|r| Id::new(*r) == id);
        if let (Some(row), Some(target)) = (find(m.row), find(m.target)) {
            data.move_row(row, target, m.position);
        }
    }
    if let Some(task) = task {
        data.task_to_table(task);
    }
}
