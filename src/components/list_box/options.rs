use super::{ListBoxStyle, ListDensity};
use crate::{Id, Insertion, Rect, Vec2};
use std::{collections::HashSet, hash::Hash, ops::Range};

/// What a click or key press selects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListMode {
    /// Rows are only activated.
    None,
    #[default]
    Single,
    /// Ctrl/Cmd toggles a row, Shift selects a range from the anchor, Ctrl+A selects all.
    Multiple,
    /// Like `Multiple`, with a check box on every row; a plain click toggles the row.
    Checks,
}

/// One user action reported by a [`ListBox`](crate::ListBox) pass; each input yields one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListEvent {
    /// The selection set you passed in was changed; read it for the new contents.
    SelectionChanged,
    /// Enter or a double click on a row.
    Activated(Id),
    /// Secondary click on a row; open your context menu.
    Context(Id),
    /// A row was dropped (see [`ListBox::drag_rows`](crate::ListBox::drag_rows)). The list
    /// never changes your model; move the row and the new order shows on the next pass.
    Moved {
        key: Id,
        target: Id,
        position: Insertion,
    },
    /// The viewport reached the end and [`ListBox::has_more`](crate::ListBox::has_more) is set.
    LoadMore,
}

#[derive(Debug)]
pub struct ListOutput {
    /// Focus-engine owner: all rows share this single Tab stop.
    pub id: Id,
    pub events: Vec<ListEvent>,
    /// The keyboard-active row (stable key), if any.
    pub active: Option<Id>,
    /// Entries intersecting the viewport.
    pub visible: Range<usize>,
    pub len: usize,
    /// Rows built (and drawn) on this pass, including the small margin around the viewport.
    pub rows_built: usize,
    /// The geometry index was rebuilt on this pass (new revision, length or sizes).
    pub rebuilt: bool,
    pub focused: bool,
    pub viewport: Rect,
    pub scroll_offset: Vec2,
}
impl ListOutput {
    pub fn selection_changed(&self) -> bool {
        self.events.contains(&ListEvent::SelectionChanged)
    }
    pub fn activated(&self) -> Option<Id> {
        self.events.iter().find_map(|e| match e {
            ListEvent::Activated(key) => Some(*key),
            _ => None,
        })
    }
    pub fn context(&self) -> Option<Id> {
        self.events.iter().find_map(|e| match e {
            ListEvent::Context(key) => Some(*key),
            _ => None,
        })
    }
    pub fn moved(&self) -> Option<(Id, Id, Insertion)> {
        self.events.iter().find_map(|e| match e {
            ListEvent::Moved {
                key,
                target,
                position,
            } => Some((*key, *target, *position)),
            _ => None,
        })
    }
    pub fn wants_more(&self) -> bool {
        self.events.contains(&ListEvent::LoadMore)
    }
}

/// A virtualized, keyboard-driven list. Rows are described by a [`ListModel`](crate::ListModel)
/// and built only near the viewport; selection and the active row follow stable keys.
pub struct ListBox<'a> {
    pub(super) id: Id,
    pub(super) mode: ListMode,
    pub(super) selection: Option<&'a mut HashSet<Id>>,
    pub(super) follows: Option<bool>,
    pub(super) enabled: bool,
    pub(super) drag: bool,
    pub(super) height: f32,
    pub(super) measured: Option<f32>,
    pub(super) density: ListDensity,
    pub(super) style: ListBoxStyle,
    pub(super) scroll_to: Option<Id>,
    pub(super) revision: u64,
    pub(super) loading: bool,
    pub(super) has_more: bool,
    pub(super) overlay_bars: bool,
    pub(super) empty: String,
    pub(super) label: String,
}
impl<'a> ListBox<'a> {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            mode: ListMode::Single,
            selection: None,
            follows: None,
            enabled: true,
            drag: false,
            height: 300.0,
            measured: None,
            density: ListDensity::Normal,
            style: ListBoxStyle::default(),
            scroll_to: None,
            revision: 0,
            loading: false,
            has_more: false,
            overlay_bars: false,
            empty: String::new(),
            label: String::new(),
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Id::new(source);
        self
    }
    pub fn mode(mut self, mode: ListMode) -> Self {
        self.mode = mode;
        self
    }
    /// The application's selection, by stable key. Single mode keeps at most one key.
    /// Keys that no longer exist stay in the set; the list never prunes your state.
    pub fn selection(mut self, selected: &'a mut HashSet<Id>) -> Self {
        self.selection = Some(selected);
        self
    }
    /// Arrow keys select the row they land on (default for `Single` and `Multiple`;
    /// hold Ctrl to move without selecting).
    pub fn selection_follows_focus(mut self, follows: bool) -> Self {
        self.follows = Some(follows);
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Rows can be dragged before or after other rows; the result is [`ListEvent::Moved`].
    /// Ctrl+Space picks up the active row for keyboard moves.
    pub fn drag_rows(mut self, enabled: bool) -> Self {
        self.drag = enabled;
        self
    }
    #[track_caller]
    pub fn max_height(mut self, height: f32) -> Self {
        self.height = crate::components::sanitize::length("ListBox::max_height", height);
        self
    }
    /// Fixed row height (the default is the style's `row_height`).
    #[track_caller]
    pub fn row_height(mut self, height: f32) -> Self {
        if let Some(height) = crate::components::sanitize::positive("ListBox::row_height", height) {
            self.style.row_height = Some(height);
            self.measured = None;
        }
        self
    }
    /// Rows take the height of their content. Unmeasured rows count as `estimate` tall, so the
    /// scroll bar stays steady; measured heights are cached by key.
    #[track_caller]
    pub fn measured_rows(mut self, estimate: f32) -> Self {
        if let Some(h) = crate::components::sanitize::positive("ListBox::measured_rows", estimate) {
            self.measured = Some(h);
        }
        self
    }
    pub fn density(mut self, density: ListDensity) -> Self {
        self.density = density;
        self
    }
    pub fn style(mut self, style: ListBoxStyle) -> Self {
        self.style.merge(&style);
        self
    }
    /// Scroll so the row is visible on this pass. Pass only when requested.
    pub fn scroll_to_key(mut self, key: impl Hash) -> Self {
        self.scroll_to = Some(Id::new(key));
        self
    }
    /// Entries changed without a length change (re-sort, equal-size filter).
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }
    /// A spinner row follows the last entry while more data is being fetched.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    /// More entries exist: scrolling near the end emits [`ListEvent::LoadMore`] once per length.
    pub fn has_more(mut self, more: bool) -> Self {
        self.has_more = more;
        self
    }
    pub fn overlay_scrollbars(mut self, overlay: bool) -> Self {
        self.overlay_bars = overlay;
        self
    }
    /// Shown, centered, when there are no entries (for example "No matches").
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }
    /// The name a screen reader speaks for the list ("Files", "Search results"). The list
    /// draws no caption of its own, so without this it is announced as an unnamed list.
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}
