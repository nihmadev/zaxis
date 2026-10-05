//! The carousel for assistive technology: a region with a page number that steps like the
//! arrow keys, its built pages as groups of which only the current one is exposed, and
//! named buttons for the arrows and the indicator.
use super::api::Carousel;
use crate::{
    accessibility::Scope, AccessActionKind as Kind, AccessOrientation, AccessRole, Id, Ui,
};

impl Carousel {
    /// The name a screen reader speaks for the carousel ("Featured photos"). The carousel
    /// draws no caption of its own, so without this it is announced as an unnamed region.
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

/// What the carousel's own node says: where it is, on which axis, and whether it acts.
pub(super) struct Region<'a> {
    pub(super) label: &'a str,
    pub(super) page: usize,
    pub(super) count: usize,
    pub(super) axis: usize,
    pub(super) enabled: bool,
}

/// Open the carousel's node; pages and controls described until it is closed are inside.
/// Its value is the one-based page number.
pub(super) fn begin(ui: &mut Ui<'_>, id: Id, region: Region<'_>) -> Scope {
    ui.a11y_begin(id, AccessRole::Region, |node| {
        node.label(region.label)
            .numeric(region.page as f64 + 1.0, 1.0, region.count as f64)
            .step(1.0)
            .orientation(if region.axis == 0 {
                AccessOrientation::Horizontal
            } else {
                AccessOrientation::Vertical
            })
            .disabled(!region.enabled)
            .action(Kind::Increment)
            .action(Kind::Decrement)
            .action(Kind::SetValue);
    })
}

/// Open the node of a built page. Sheets and neighbours of the current page are drawn
/// but not exposed.
pub(super) fn page(ui: &mut Ui<'_>, id: Id, page: usize, count: usize, current: bool) -> Scope {
    ui.a11y_begin(id, AccessRole::Group, |node| {
        node.label(format!("Page {}", page + 1))
            .position_in_set(page, count)
            .hidden(!current);
    })
}
