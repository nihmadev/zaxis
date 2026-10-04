//! In-window drag and drop with typed payloads.
//!
//! [`DragSource`] and [`DropTarget`] wrap freshly built content
//! (`ui.drag_source(id, payload, build)`, `ui.drop_target(id, accept, build)`)
//! or attach to a widget whose [`crate::Response`] is already known
//! (`.attach(ui, response)`). Hit regions, repaint, capture and animation are
//! internal. See the drag-and-drop guide for the lifecycle, target priority,
//! autoscroll, keyboard model and limitations.
mod indicator;
mod preview;
mod source;
pub(crate) mod style;
mod target;
mod types;
mod ui;

pub use source::{DragOutput, DragSource};
pub use style::DragStyle;
pub use target::{DropOutput, DropTarget};
pub use types::{
    move_item, DragEffect, DragEnd, DragReason, DropZones, Dropped, Insertion, PreviewKind,
    RowDrag, RowMove, TreeNodeDrag,
};
