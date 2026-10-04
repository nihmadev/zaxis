use super::{DragOutput, DragSource, DropOutput, DropTarget};
use crate::{Id, Ui};
use std::any::Any;

impl Ui<'_> {
    /// Make freshly built content draggable. The content keeps its clicks and
    /// hover until the pointer travels the drag threshold. `payload` is moved
    /// into the session when a drag begins; use [`DragSource`] for options.
    pub fn drag_source<P: Any, R>(
        &mut self,
        id: Id,
        payload: P,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> DragOutput<R> {
        DragSource::new(id, payload).show(self, build)
    }

    /// Accept payloads of type `P` for which `accept` returns true. The result
    /// reports hover state and, once, the dropped payload.
    pub fn drop_target<P: Any, R>(
        &mut self,
        id: Id,
        accept: impl Fn(&P) -> bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> DropOutput<R, P> {
        DropTarget::new(id, accept).show(self, build)
    }

    /// Application id of the source being dragged, if any.
    pub fn dragging(&self) -> Option<Id> {
        self.context.dragging()
    }

    /// Keep the dragged source alive while its widget is not built, for example
    /// a virtualized row scrolled out of view. Call each pass while the model
    /// still contains the item; otherwise the drag ends as `SourceLost`.
    pub fn keep_drag_source(&mut self, id: Id) {
        self.context.drag_keep_alive(id);
    }
}
