//! Row dragging for `Grid`, built on the shared drag primitives.
use crate::{DragSource, DropTarget, DropZones, Id, Insertion, Rect, RowDrag, RowMove, Ui};

/// Make each row a source and a before/after target. `rects` are in the grid's
/// build coordinates, like every other paint and hit of the grid.
pub(super) fn rows(
    ui: &mut Ui<'_>,
    grid: Id,
    keys: &[Id],
    rects: &[Rect],
    _width: f32,
) -> Option<RowMove> {
    let mut moved = None;
    for (&row, &rect) in keys.iter().zip(rects) {
        let id = grid.with(("dnd-row", row));
        let response = ui.response(id, rect, true);
        DragSource::new(row, RowDrag { owner: grid, row }).attach(ui, response);
        let out = DropTarget::new(row, move |p: &RowDrag| p.owner == grid)
            .zones(DropZones::rows())
            .attach(ui, response);
        if let Some(drop) = out.dropped {
            moved = Some(RowMove {
                row: drop.payload.row,
                target: row,
                position: drop.insertion.unwrap_or(Insertion::After),
            });
        }
    }
    moved
}
