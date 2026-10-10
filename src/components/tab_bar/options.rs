//! Public value types of the tab strip and the builder's resolved configuration.
use crate::{components::theme::TabsStyle, Id, Response, Vec2};

/// Overall look of a [`TabBar`](crate::TabBar). Layout, keyboard and drag and drop are the
/// same in both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TabVariant {
    /// Browser or editor tabs on a darker strip: the active tab takes the surface of the
    /// page under it and opens into it, a tab under the pointer is a rounded plate.
    #[default]
    Folder,
    /// Plain labels over a hairline; the active one is marked by an accent line.
    Underline,
}

/// How wide the tabs are. Whatever the mode, a row that does not fit shrinks its tabs
/// evenly down to the minimum width (labels get an ellipsis) before it scrolls.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TabWidth {
    /// Each tab fits its icon, label and close button, between the minimum and maximum width.
    #[default]
    Content,
    /// The tabs share the row equally; a tab that needs more keeps its content width.
    Equal,
    /// Every tab is this wide (logical pixels).
    Fixed(f32),
}

/// What moving the keyboard focus between tabs does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabActivation {
    /// The tab that receives focus from the arrow keys, Home or End is selected.
    #[default]
    OnFocus,
    /// Focus moves alone; Enter or Space selects the focused tab.
    Manual,
}

/// The payload of a dragged tab. Bars accept it when `group` is their own, so tabs move
/// between the bars of one group and nowhere else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabDrag {
    pub group: Id,
    /// The bar the tab was picked up from.
    pub bar: Id,
    /// The tab: the identity of its value, `Id::new(&value)`.
    pub tab: Id,
}

/// A request to move a tab. The library never edits the application's model: apply it and
/// the new order is used on the next pass.
///
/// Tabs are named by `Id::new(&value)`, the identity the bars use, and bars by the
/// `id_source` they were created with, as `Id::new(id_source)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabMove {
    pub tab: Id,
    pub from_bar: Id,
    pub to_bar: Id,
    /// The tab of `to_bar` that the moved one goes in front of; `None` puts it last.
    pub before: Option<Id>,
}

impl TabMove {
    /// Where the moved tab goes in the order of the destination bar *without it*.
    pub fn insert_index(&self, order: &[Id]) -> usize {
        let rest = order.iter().filter(|id| **id != self.tab);
        let before = self.before.filter(|before| *before != self.tab);
        let rest: Vec<&Id> = rest.collect();
        before
            .and_then(|before| rest.iter().position(|id| **id == before))
            .unwrap_or(rest.len())
    }

    /// For a move inside one bar: the indices to give [`move_item`](crate::move_item).
    /// `None` when the tab is not in `order` or would stay where it is.
    pub fn reorder(&self, order: &[Id]) -> Option<(usize, usize)> {
        let from = order.iter().position(|id| *id == self.tab)?;
        let to = self.insert_index(order);
        (from != to).then_some((from, to))
    }
}

/// A tab released over no strip at all: where, and which one. The cue to open it in a
/// window of its own, which the application has to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabRelease {
    /// The tab, as `Id::new(&value)`.
    pub tab: Id,
    /// The pointer position in window coordinates.
    pub position: Vec2,
}

/// What one pass of a [`TabBar`](crate::TabBar) reports. Every event arrives once, from the
/// pass that follows the input.
#[derive(Debug)]
pub struct TabBarOutput {
    /// One response per tab, in order; `changed()` marks the tab the user selected.
    pub responses: Vec<Response>,
    /// The whole strip; `changed()` is true when the selection changed.
    pub strip: Response,
    /// Unused header space after the tabs, excluding trailing controls and overflow.
    /// `None` while the row scrolls. Suitable for a custom enclosing-window drag region.
    pub free_space: Option<crate::Rect>,
    /// The tab the user selected (a click, or the focus under `TabActivation::OnFocus`).
    pub selected: Option<Id>,
    /// The tab the user asked to close: its close button, a middle click or Delete.
    pub closed: Option<Id>,
    /// A tab dropped on this bar (or moved with Ctrl+Shift and the arrow keys).
    pub moved: Option<TabMove>,
    /// A tab of this bar was released over no bar at all: the cue to open it in a window
    /// of its own, which the application has to do.
    pub released_outside: Option<TabRelease>,
}

/// Everything the builders set, resolved once per pass.
pub(super) struct Config {
    pub id: Id,
    pub variant: TabVariant,
    pub closable: bool,
    pub width: TabWidth,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub activation: TabActivation,
    pub group: Option<Id>,
    pub reorderable: bool,
    pub keyboard_close: bool,
    pub window_drag: bool,
    pub overflow_menu: bool,
    pub style: TabsStyle,
}

impl Config {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            variant: TabVariant::Folder,
            closable: false,
            width: TabWidth::Content,
            min_width: None,
            max_width: None,
            activation: TabActivation::OnFocus,
            group: None,
            reorderable: false,
            keyboard_close: true,
            window_drag: false,
            overflow_menu: false,
            style: TabsStyle::default(),
        }
    }
}
