//! Drag and drop of tabs on the shared drag primitives: every tab is a [`DragSource`] and a
//! before/after [`DropTarget`] for tabs of its group, and the free space after the last tab
//! is a target for the end of the row.

use super::options::TabDrag;
use crate::{
    DragEnd, DragSource, DropTarget, DropZones, Dropped, Id, Insertion, Rect, Response, Ui,
};

/// What one tab saw of drag and drop in this pass.
#[derive(Default)]
pub(super) struct TabDnd {
    /// This tab is being dragged (the source reports it once the drag has begun).
    pub dragging: bool,
    /// The drag that ended on this pass, if it was this tab's.
    pub finished: Option<DragEnd>,
    /// Where on this tab a dragged tab of the group would land.
    pub hover: Option<Insertion>,
    pub dropped: Option<Dropped<TabDrag>>,
}

pub(super) struct Group {
    pub group: Id,
    pub bar: Id,
}

/// Make `response` (the tab `key`) a source of `TabDrag` and a before/after target.
pub(super) fn tab(
    ui: &mut Ui<'_>,
    key: Id,
    response: Response,
    group: &Group,
    source: bool,
) -> TabDnd {
    let mut dnd = TabDnd::default();
    let payload = TabDrag {
        group: group.group,
        bar: group.bar,
        tab: key,
    };
    let out = DragSource::new(key, payload)
        .enabled(source)
        .attach(ui, response);
    dnd.dragging = out.active && !out.started;
    dnd.finished = out.finished;
    if out.active {
        // The source stays alive while it is scrolled out of view of its strip.
        ui.keep_drag_source(key);
    }
    let own = group.group;
    let target = DropTarget::new(key, move |p: &TabDrag| p.group == own)
        .zones(DropZones::columns())
        .indicator(false)
        .attach(ui, response);
    if target.acceptable {
        dnd.hover = target.insertion;
    }
    dnd.dropped = target.dropped;
    dnd
}

/// The free space after the last tab, a target for "to the end of the row".
pub(super) fn tail(ui: &mut Ui<'_>, rect: Rect, group: &Group) -> (bool, Option<Dropped<TabDrag>>) {
    let id = ui.scope.with(("tab-tail", group.bar));
    let response = ui.response(id, rect, true);
    let own = group.group;
    let target = DropTarget::new(id, move |p: &TabDrag| p.group == own)
        .indicator(false)
        .attach(ui, response);
    (target.acceptable, target.dropped)
}
