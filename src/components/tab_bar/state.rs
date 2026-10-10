//! Retained state of a tab strip, kept in the context while the strip is built. It is the
//! registry that tells a released tab whether it landed on any strip, and remembers what
//! the last pass saw so that a change of selection or focus reveals its tab.

use crate::{Id, Rect};

pub(crate) struct TabBarState {
    pub last_frame: u64,
    pub window: Id,
    /// The visible part of the strip, in screen coordinates.
    pub strip: Rect,
    pub selected: Option<Id>,
    pub focused: Option<Id>,
    /// The overflow menu is open.
    pub menu_open: bool,
}
