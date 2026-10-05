//! The semantic description of one widget, as collected during a pass.

use super::{
    text::TextRef, AccessActionKind, AccessLive, AccessOrientation, AccessRole, AccessToggled,
};
use crate::{Color, Id, Rect, Vec2};

pub(crate) const DISABLED: u16 = 1 << 0;
pub(crate) const HIDDEN: u16 = 1 << 1;
pub(crate) const READ_ONLY: u16 = 1 << 2;
pub(crate) const REQUIRED: u16 = 1 << 3;
pub(crate) const INVALID: u16 = 1 << 4;
pub(crate) const MODAL: u16 = 1 << 5;
pub(crate) const MULTISELECTABLE: u16 = 1 << 6;
pub(crate) const BUSY: u16 = 1 << 7;
pub(crate) const CLIPS: u16 = 1 << 8;
pub(crate) const HAS_POPUP: u16 = 1 << 9;
/// The node names itself on purpose: no missing-name diagnostic.
pub(crate) const UNNAMED_OK: u16 = 1 << 11;
/// A link the application marked as followed.
pub(crate) const VISITED: u16 = 1 << 12;

/// Range of a slider, spin button, splitter or progress bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Numeric {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,
    pub jump: Option<f64>,
}

/// Offsets of a scrollable container. `id` is the scroll state the actions move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Scroll {
    pub id: Id,
    pub offset: Vec2,
    pub max: Vec2,
}

/// Position of a row, a header or a cell in its table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TablePosition {
    pub row_count: Option<u32>,
    pub column_count: Option<u32>,
    pub row: Option<u32>,
    pub column: Option<u32>,
    pub row_span: Option<u32>,
    pub column_span: Option<u32>,
    /// A column header whose column orders the rows: ascending or not.
    pub sort_ascending: Option<bool>,
}

/// Properties only a few widgets have, kept out of line so a node stays small.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct More {
    pub numeric: Option<Numeric>,
    pub placeholder: Option<String>,
    pub url: Option<String>,
    pub shortcut: Option<String>,
    pub role_description: Option<String>,
    pub orientation: Option<AccessOrientation>,
    pub live: Option<AccessLive>,
    pub scroll: Option<Scroll>,
    pub table: TablePosition,
    pub text: Option<TextRef>,
    /// Where the text of `text` starts relative to the node's top left corner, in logical
    /// pixels: a field scrolls its text without rebuilding the lines.
    pub text_origin: Vec2,
    /// Anchor and focus of the text selection, as byte offsets into the text of `text`.
    pub selection: Option<(usize, usize)>,
    pub labelled_by: Vec<Id>,
    pub described_by: Vec<Id>,
    pub controls: Vec<Id>,
    pub error_message: Option<Id>,
    pub active_descendant: Option<Id>,
    pub color: Option<Color>,
}

/// What assistive technology learns about one widget: role, name, value, state and the
/// actions it accepts. Built-in components describe themselves; a custom widget fills one in
/// through [`Ui::accessible`](crate::Ui::accessible).
#[derive(Clone, Debug, PartialEq)]
pub struct AccessNode {
    pub(crate) id: Id,
    pub(crate) layer: Id,
    pub(crate) role: AccessRole,
    pub(crate) rect: Rect,
    pub(crate) clip: Rect,
    pub(crate) label: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) value: Option<String>,
    pub(crate) toggled: Option<AccessToggled>,
    pub(crate) selected: Option<bool>,
    pub(crate) expanded: Option<bool>,
    pub(crate) flags: u16,
    pub(crate) actions: u16,
    /// Zero-based position and size of the set (list, tree level, radio group).
    pub(crate) set: Option<(u32, u32)>,
    pub(crate) level: Option<u32>,
    /// The hit region a `Click` request activates, exactly as a pointer click would.
    pub(crate) click: Option<Id>,
    /// The hit region that holds keyboard focus for this node, when it is not its own.
    pub(crate) focus: Option<Id>,
    pub(crate) more: Option<Box<More>>,
}

impl AccessNode {
    pub(crate) fn new(id: Id, layer: Id, role: AccessRole) -> Self {
        Self {
            id,
            layer,
            role,
            rect: Rect::default(),
            clip: Rect::default(),
            label: None,
            description: None,
            value: None,
            toggled: None,
            selected: None,
            expanded: None,
            flags: 0,
            actions: 0,
            set: None,
            level: None,
            click: None,
            focus: None,
            more: None,
        }
    }

    pub(crate) fn more(&mut self) -> &mut More {
        self.more.get_or_insert_with(Default::default)
    }
    pub(crate) fn has(&self, flag: u16) -> bool {
        self.flags & flag != 0
    }
    pub(crate) fn flag(&mut self, flag: u16, on: bool) -> &mut Self {
        if on {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
        self
    }
    pub(crate) fn supports(&self, action: AccessActionKind) -> bool {
        self.actions & (1 << action as u8) != 0
    }

    pub fn role(&mut self, role: AccessRole) -> &mut Self {
        self.role = role;
        self
    }
    /// The name a screen reader speaks. Empty text clears it.
    pub fn label(&mut self, label: impl Into<String>) -> &mut Self {
        let label = label.into();
        self.label = (!label.is_empty()).then_some(label);
        self
    }
    /// Extra detail spoken after the name: a hint, a tooltip, an error.
    pub fn description(&mut self, description: impl Into<String>) -> &mut Self {
        let description = description.into();
        self.description = (!description.is_empty()).then_some(description);
        self
    }
    /// The value as text. For a label, paragraph or heading this is the text itself.
    pub fn value(&mut self, value: impl Into<String>) -> &mut Self {
        self.value = Some(value.into());
        self
    }
    /// Range value of a slider, spin button, splitter or progress bar. Values that are not
    /// finite are dropped rather than published.
    pub fn numeric(&mut self, value: f64, min: f64, max: f64) -> &mut Self {
        if value.is_finite() && min.is_finite() && max.is_finite() {
            self.more().numeric = Some(Numeric {
                value,
                min,
                max,
                step: None,
                jump: None,
            });
        }
        self
    }
    /// Size of one `Increment`/`Decrement`, after [`Self::numeric`].
    pub fn step(&mut self, step: f64) -> &mut Self {
        if let Some(numeric) = self.more().numeric.as_mut() {
            numeric.step = (step.is_finite() && step > 0.0).then_some(step);
        }
        self
    }
    pub fn toggled(&mut self, toggled: impl Into<AccessToggled>) -> &mut Self {
        self.toggled = Some(toggled.into());
        self
    }
    pub fn selected(&mut self, selected: bool) -> &mut Self {
        self.selected = Some(selected);
        self
    }
    pub fn expanded(&mut self, expanded: bool) -> &mut Self {
        self.expanded = Some(expanded);
        self
    }
    pub fn disabled(&mut self, disabled: bool) -> &mut Self {
        self.flag(DISABLED, disabled)
    }
    /// Present in the tree but skipped by assistive technology, together with its children.
    pub fn hidden(&mut self, hidden: bool) -> &mut Self {
        self.flag(HIDDEN, hidden)
    }
    pub fn read_only(&mut self, read_only: bool) -> &mut Self {
        self.flag(READ_ONLY, read_only)
    }
    pub fn required(&mut self, required: bool) -> &mut Self {
        self.flag(REQUIRED, required)
    }
    pub fn invalid(&mut self, invalid: bool) -> &mut Self {
        self.flag(INVALID, invalid)
    }
    pub fn busy(&mut self, busy: bool) -> &mut Self {
        self.flag(BUSY, busy)
    }
    pub fn orientation(&mut self, orientation: AccessOrientation) -> &mut Self {
        self.more().orientation = Some(orientation);
        self
    }
    /// One-based heading or tree level.
    pub fn level(&mut self, level: u32) -> &mut Self {
        self.level = Some(level.max(1));
        self
    }
    /// Zero-based position in a set of `size` items, including items not built this pass.
    pub fn position_in_set(&mut self, index: usize, size: usize) -> &mut Self {
        self.set = Some((
            index.min(u32::MAX as usize) as u32,
            size.min(u32::MAX as usize) as u32,
        ));
        self
    }
    /// Changes of this node's text are announced without moving focus.
    pub fn live(&mut self, live: AccessLive) -> &mut Self {
        self.more().live = Some(live);
        self
    }
    pub fn placeholder(&mut self, text: impl Into<String>) -> &mut Self {
        let text = text.into();
        self.more().placeholder = (!text.is_empty()).then_some(text);
        self
    }
    pub fn url(&mut self, url: impl Into<String>) -> &mut Self {
        self.more().url = Some(url.into());
        self
    }
    pub fn keyboard_shortcut(&mut self, shortcut: impl Into<String>) -> &mut Self {
        let shortcut = shortcut.into();
        self.more().shortcut = (!shortcut.is_empty()).then_some(shortcut);
        self
    }
    /// Accept `action`. Requests arrive from [`Ui::accessible`](crate::Ui::accessible) on the
    /// next pass; `Click` on a region made with [`Ui::interact`](crate::Ui::interact) is
    /// delivered as an ordinary click instead.
    pub fn action(&mut self, action: AccessActionKind) -> &mut Self {
        self.actions |= 1 << action as u8;
        self
    }
    /// Name this node after the text of another widget, for example a caption next to it.
    pub fn labelled_by(&mut self, id: Id) -> &mut Self {
        self.more().labelled_by.push(id);
        self
    }
    pub fn described_by(&mut self, id: Id) -> &mut Self {
        self.more().described_by.push(id);
        self
    }

    /// `Click` requests set `clicked` on the hit region `id`, like a pointer click.
    pub(crate) fn clicks(&mut self, id: Id) -> &mut Self {
        self.click = Some(id);
        self.action(AccessActionKind::Click)
    }
    /// The hit region `id` holds keyboard focus on behalf of this node.
    pub(crate) fn focus_on(&mut self, id: Id) -> &mut Self {
        self.focus = Some(id);
        self
    }
    pub(crate) fn clips(&mut self) -> &mut Self {
        self.flag(CLIPS, true)
    }
    pub(crate) fn modal(&mut self) -> &mut Self {
        self.flag(MODAL, true)
    }
    pub(crate) fn multiselectable(&mut self, on: bool) -> &mut Self {
        self.flag(MULTISELECTABLE, on)
    }
    pub(crate) fn has_popup(&mut self) -> &mut Self {
        self.flag(HAS_POPUP, true)
    }
    /// A nameless node of a role that normally needs a name is intended.
    pub(crate) fn unnamed(&mut self) -> &mut Self {
        self.flag(UNNAMED_OK, true)
    }
    pub(crate) fn scroll(&mut self, id: Id, offset: Vec2, max: Vec2) -> &mut Self {
        self.more().scroll = Some(Scroll { id, offset, max });
        self.clips().action(AccessActionKind::Scroll)
    }
    pub(crate) fn controls(&mut self, id: Id) -> &mut Self {
        self.more().controls.push(id);
        self
    }
    pub(crate) fn error_message(&mut self, id: Id) -> &mut Self {
        self.more().error_message = Some(id);
        self
    }
    pub(crate) fn active_descendant(&mut self, id: Id) -> &mut Self {
        self.more().active_descendant = Some(id);
        self
    }
    pub(crate) fn color(&mut self, color: Color) -> &mut Self {
        self.more().color = Some(color);
        self
    }
    pub(crate) fn jump(&mut self, jump: f64) -> &mut Self {
        if let Some(numeric) = self.more().numeric.as_mut() {
            numeric.jump = (jump.is_finite() && jump > 0.0).then_some(jump);
        }
        self
    }
    pub(crate) fn table(&mut self) -> &mut TablePosition {
        &mut self.more().table
    }
    /// Visual lines of this node's text, placed relative to its rect, and the selection.
    pub(crate) fn text(&mut self, text: TextRef, selection: Option<(usize, usize)>) -> &mut Self {
        let more = self.more();
        more.text = Some(text);
        more.selection = selection;
        self
    }
    pub(crate) fn text_origin(&mut self, origin: Vec2) -> &mut Self {
        self.more().text_origin = origin;
        self
    }
    /// A link the user already followed.
    pub(crate) fn visited(&mut self, visited: bool) -> &mut Self {
        self.flag(VISITED, visited)
    }
    /// The rows are ordered by this column header, ascending or descending.
    pub(crate) fn sort(&mut self, ascending: bool) -> &mut Self {
        self.more().table.sort_ascending = Some(ascending);
        self
    }
}
