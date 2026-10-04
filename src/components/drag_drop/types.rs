//! Public value types shared by drag sources, drop targets and containers.
use crate::{Id, Layout, Vec2};

/// Where a dropped item lands relative to the target element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Insertion {
    Before,
    /// Into the target as a child or container.
    Inside,
    After,
}

/// What the drag intends to do. It only selects the cursor; the application
/// decides how to apply the drop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DragEffect {
    #[default]
    Move,
    Copy,
}

/// Why a drag session ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DragReason {
    /// Released over a target that accepted the payload.
    Dropped,
    /// Released over nothing or over a target that rejected the payload.
    Cancelled,
    Escape,
    /// The source was removed, disabled or never reached the UI again.
    SourceLost,
    FocusLost,
}

/// Final result of one session. Reported once, to the source and through
/// [`crate::Context::drag_end`], on the pass after the session ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragEnd {
    pub reason: DragReason,
    /// Application key of the source.
    pub source: Id,
    /// Application key of the target that received the drop, if any.
    pub target: Option<Id>,
    /// Pointer position in window coordinates when the session ended.
    pub position: Vec2,
    pub insertion: Option<Insertion>,
}

/// A completed drop, delivered once to the accepting target. The payload is
/// owned by the application and moves back out of the session untouched.
#[derive(Clone, Debug, PartialEq)]
pub struct Dropped<P> {
    pub source: Id,
    pub target: Id,
    /// Pointer position in window coordinates.
    pub position: Vec2,
    /// Pointer position relative to the target's top-left corner.
    pub local: Vec2,
    pub insertion: Option<Insertion>,
    pub effect: DragEffect,
    pub payload: P,
}

impl<P> Dropped<P> {
    /// Source and destination indices for a reorder of `order`, ready for
    /// [`move_item`]: remove at `from`, insert at `to`. The target must be a row
    /// of the same list; `None` if either id is missing or nothing moves.
    pub fn reorder(&self, order: &[Id]) -> Option<(usize, usize)> {
        let from = order.iter().position(|id| *id == self.source)?;
        let at = order.iter().position(|id| *id == self.target)?;
        let slot = match self.insertion? {
            Insertion::Before => at,
            Insertion::After | Insertion::Inside => at + 1,
        };
        let to = if from < slot { slot - 1 } else { slot };
        (from != to).then_some((from, to))
    }
}

/// Apply the indices returned by [`Dropped::reorder`] to an application list.
pub fn move_item<T>(items: &mut Vec<T>, from: usize, to: usize) {
    if from < items.len() && to < items.len() {
        let item = items.remove(from);
        items.insert(to, item);
    }
}

/// Insertion zones inside a target, along one axis of its rectangle.
/// `edge` is the fraction of the extent that selects `Before` and `After`;
/// the middle selects `Inside` when enabled, otherwise the halves split evenly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropZones {
    pub layout: Layout,
    pub edge: f32,
    pub inside: bool,
}

impl DropZones {
    /// Before the upper half, after the lower half of a vertical list row.
    pub const fn rows() -> Self {
        Self {
            layout: Layout::Vertical,
            edge: 0.5,
            inside: false,
        }
    }
    /// Before the left half, after the right half of a horizontal item.
    pub const fn columns() -> Self {
        Self {
            layout: Layout::Horizontal,
            ..Self::rows()
        }
    }
    /// Quarter / half / quarter zones with `Inside` for containers and tree branches.
    pub const fn tree() -> Self {
        Self {
            layout: Layout::Vertical,
            edge: 0.25,
            inside: true,
        }
    }
    pub const fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }
    /// Fraction of the extent for each of `Before` and `After`, clamped to `0..=0.5`.
    pub fn edge(mut self, edge: f32) -> Self {
        self.edge = if edge.is_finite() {
            edge.clamp(0.0, 0.5)
        } else {
            0.5
        };
        self
    }
    /// Disable for leaves: `Inside` is never reported.
    pub const fn inside(mut self, inside: bool) -> Self {
        self.inside = inside;
        self
    }
    /// Zone for a position relative to the target's top-left corner.
    pub fn classify(self, local: Vec2, size: Vec2) -> Insertion {
        let (offset, extent) = match self.layout {
            Layout::Vertical => (local.y, size.y),
            Layout::Horizontal => (local.x, size.x),
        };
        let t = if extent > 0.0 {
            (offset / extent).clamp(0.0, 1.0)
        } else {
            0.5
        };
        if self.inside && self.edge < 0.5 {
            if t < self.edge {
                Insertion::Before
            } else if t > 1.0 - self.edge {
                Insertion::After
            } else {
                Insertion::Inside
            }
        } else if t < 0.5 {
            Insertion::Before
        } else {
            Insertion::After
        }
    }
}

/// How the dragged item is shown under the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreviewKind {
    /// Semi-transparent copy of what the source painted when the drag began.
    #[default]
    Snapshot,
    /// Application content built by the source's `preview` closure.
    Custom,
    None,
}

/// Built-in payload for tree node drags. Accept it from your own targets to
/// move nodes into lists or other widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeNodeDrag {
    pub tree: Id,
    pub node: Id,
}

/// Built-in payload for table and grid row drags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowDrag {
    pub owner: Id,
    pub row: Id,
}

/// A row move requested by dragging inside a table or grid. The library never
/// reorders the model; apply it and the new order is used on the next pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowMove {
    pub row: Id,
    pub target: Id,
    pub position: Insertion,
}
