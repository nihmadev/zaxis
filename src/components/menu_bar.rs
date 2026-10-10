//! Application menu: a horizontal bar of titles, or one hamburger button, opening
//! dropdown panels with cascading submenus.
use std::hash::Hash;

use super::{
    context_menu::{paint, ContextMenuItem, ContextMenuStyle},
    Popup, Ui,
};
use crate::{Id, Vec2};

mod actions;
mod cascade;
mod nav;
mod trigger;

pub(crate) use nav::MenuBarState;
use nav::{active_id, apply, level, root_or};

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Action(Id),
    Submenu(Vec<MenuItem>),
    Separator,
}

/// One row of a menu: an action, a submenu, or a separator.
///
/// Action IDs are stable and independent of the label; the label may change with the
/// language. The shortcut is shown right-aligned and is display-only: the application
/// binds the key itself (for example with `GlobalShortcut`).
///
/// ```
/// use zaxis::MenuItem;
/// let file = MenuItem::submenu("File", [
///     MenuItem::new("new", "New").shortcut("Ctrl+N"),
///     MenuItem::separator(),
///     MenuItem::new("autosave", "Autosave").checked(true),
/// ]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    kind: Kind,
    text: String,
    shortcut: String,
    checked: bool,
    enabled: bool,
    /// Bound to a registered action: see [`MenuItem::action`].
    action: bool,
}

impl MenuItem {
    fn with_kind(kind: Kind, text: String) -> Self {
        Self {
            enabled: !matches!(kind, Kind::Separator),
            kind,
            text,
            shortcut: String::new(),
            checked: false,
            action: false,
        }
    }
    pub fn new(id: impl Hash, text: impl Into<String>) -> Self {
        Self::with_kind(Kind::Action(Id::new(id)), text.into())
    }
    /// An item that runs a registered action. Its caption, shortcut, enabled and checked
    /// state come from the registry, so it always agrees with the toolbar button and the key.
    /// Choosing it raises the action's event (`ui.actions().triggered(id)`); the item is not
    /// reported in [`MenuBarOutput::selected`]. Pass the value the action was declared with.
    ///
    /// ```
    /// use zaxis::MenuItem;
    /// let save = MenuItem::action("file.save");
    /// ```
    pub fn action(id: impl Hash) -> Self {
        let mut item = Self::with_kind(Kind::Action(crate::actions::action_id(id)), String::new());
        item.action = true;
        item
    }
    /// An item that opens a nested panel. Submenus nest to any depth.
    pub fn submenu(text: impl Into<String>, items: impl IntoIterator<Item = MenuItem>) -> Self {
        Self::with_kind(Kind::Submenu(items.into_iter().collect()), text.into())
    }
    pub fn separator() -> Self {
        Self::with_kind(Kind::Separator, String::new())
    }
    pub fn shortcut(mut self, text: impl Into<String>) -> Self {
        self.shortcut = text.into();
        self
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled && !self.is_separator();
        self
    }
    fn is_separator(&self) -> bool {
        matches!(self.kind, Kind::Separator)
    }
    fn is_submenu(&self) -> bool {
        matches!(self.kind, Kind::Submenu(_)) && self.enabled
    }
    fn children(&self) -> Option<&[MenuItem]> {
        match &self.kind {
            Kind::Submenu(items) => Some(items),
            _ => None,
        }
    }
    fn selectable(&self) -> bool {
        self.enabled && !self.is_separator()
    }
    /// The panel row. Row IDs only need to be unique inside one panel.
    fn row(&self, index: usize) -> ContextMenuItem {
        if self.is_separator() {
            return ContextMenuItem::separator();
        }
        let mut row = ContextMenuItem::new(("menu-item", index), self.text.as_str())
            .right_text(self.shortcut.as_str())
            .checked(self.checked)
            .enabled(self.enabled);
        row.submenu = matches!(self.kind, Kind::Submenu(_));
        row
    }
}

/// Result of one pass of a [`MenuBar`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuBarOutput {
    pub open: bool,
    /// Exactly one activation per gesture. The menu closes before returning it.
    pub selected: Option<Id>,
}

/// A menu bar (`File Edit View`) or, with [`MenuBar::compact`], a hamburger button whose
/// panel lists the same titles as submenus. Panels cascade to the right, switch while the
/// pointer moves between titles, and use the arrow keys (Up/Down move, Right/Enter enter a
/// submenu, Left leaves it or switches the title), Escape and outside presses to close.
///
/// ```no_run
/// # fn menus(ui: &mut zaxis::Ui<'_>) {
/// use zaxis::{MenuBar, MenuItem};
/// let items = [
///     MenuItem::submenu("File", [
///         MenuItem::new("new", "New").shortcut("Ctrl+N"),
///         MenuItem::submenu("Open recent", [MenuItem::new("r1", "notes.txt")]),
///     ]),
///     MenuItem::submenu("Help", [MenuItem::new("about", "About")]),
/// ];
/// if let Some(id) = MenuBar::new("main-menu", &items).compact(true).show(ui).selected {
///     // react to the chosen action
/// }
/// # }
/// ```
pub struct MenuBar<'a> {
    source: Id,
    items: &'a [MenuItem],
    compact: bool,
    style: ContextMenuStyle,
}

impl<'a> MenuBar<'a> {
    pub fn new(source: impl Hash, items: &'a [MenuItem]) -> Self {
        Self {
            source: Id::new(source),
            items,
            compact: false,
            style: Default::default(),
        }
    }
    /// One hamburger button instead of a row of titles.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
    pub fn style(mut self, style: ContextMenuStyle) -> Self {
        self.style = style;
        self
    }

    pub fn show(self, ui: &mut Ui<'_>) -> MenuBarOutput {
        if !actions::any(self.items) {
            return self.show_items(ui);
        }
        let items = actions::resolve(ui, self.items);
        let bar = MenuBar {
            source: self.source,
            items: &items,
            compact: self.compact,
            style: self.style,
        };
        let mut output = bar.show_items(ui);
        actions::raise(ui, &items, &mut output.selected);
        output
    }

    fn show_items(mut self, ui: &mut Ui<'_>) -> MenuBarOutput {
        self.style.normalize();
        let compact = self.compact;
        let bar = ui.scope.with(("menu-bar", self.source));
        let mut state = ui.context.menus.menu_bars.remove(&bar).unwrap_or_default();
        state.last_frame = ui.context.frame;
        let mut output = MenuBarOutput::default();

        let shown = state.open.then_some(if compact { 0 } else { state.menu });
        let trigger::Triggers {
            anchors,
            pressed,
            collapse,
        } = trigger::show(ui, bar, self.items, compact, shown);
        if let Some(index) = pressed {
            let item = &self.items[index];
            if compact && !self.items.is_empty() || item.is_submenu() {
                state.open = true;
                state.reset(if compact { 0 } else { index });
                state.keyboard = false;
                state.pointer = ui.context.input().pointer;
                ui.context.request_repaint();
            } else if let (Kind::Action(id), true) = (&item.kind, item.enabled) {
                output.selected = Some(*id);
            }
        }
        let anchor = anchors
            .get(if compact { 0 } else { state.menu })
            .copied()
            .flatten();
        let valid = anchor.is_some() && (compact || self.items[state.menu].is_submenu());
        state.open &= valid && ui.enabled;
        // A press on the title that owns the open panel closes it.
        if state.open && (ui.context.clicked(bar) || collapse) {
            state.open = false;
            ui.context.close_popup();
        }
        if state.open {
            self.run_open(ui, bar, &mut state, &anchors, anchor.unwrap(), &mut output);
        }
        output.open = state.open;
        ui.context.menus.menu_bars.insert(bar, state);
        output
    }

    fn run_open(
        &self,
        ui: &mut Ui<'_>,
        bar: Id,
        state: &mut MenuBarState,
        anchors: &[Option<crate::Rect>],
        anchor: crate::Rect,
        output: &mut MenuBarOutput,
    ) {
        let (items, compact, style) = (self.items, self.compact, &self.style);
        let pointer = ui.context.input().pointer;
        let pointer_moved = pointer != state.pointer;
        state.pointer = pointer;
        if pointer_moved {
            state.keyboard = false;
            // Moving across the titles switches the open menu.
            let over = pointer.and_then(|p| {
                anchors
                    .iter()
                    .position(|rect| rect.is_some_and(|r| r.contains(p)))
            });
            if let Some(index) = over.filter(|i| !compact && *i != state.menu) {
                if items[index].is_submenu() {
                    state.reset(index);
                }
            }
        }
        let moved = self.keys(ui, bar, state, output);
        let anchor = if compact {
            anchor
        } else {
            anchors[state.menu].unwrap_or(anchor)
        };
        let Some(root_items) = level(items, compact, state, 0) else {
            state.open = false;
            return;
        };
        let rows = cascade::rows(root_items);
        let metrics = paint::measure(ui, &rows, style);
        let surface = paint::popup_style(ui, style);
        let padding = style.padding;
        let active = active_id(state, 0, &rows);
        let panel_id = |level: usize| match level {
            0 => bar.with(("level", 0)),
            level => bar.with(("panel", level)),
        };
        // The row the keyboard is on, for the node that holds focus while the menu is open.
        let deepest = state.depth.saturating_sub(1);
        let keyboard_row = state.path.get(deepest).filter(|_| state.keyboard);
        let access = cascade::Access {
            label: if compact {
                "Menu"
            } else {
                items[state.menu].text.as_str()
            },
            focus: Some(bar),
            active: keyboard_row
                .map(|index| panel_id(deepest).with(("item", Id::new(("menu-item", *index))))),
            open: state.path.first().copied().filter(|_| state.depth > 1),
        };
        let reveal = moved
            .then(|| state.path.first().copied().filter(|_| state.depth == 1))
            .flatten();
        let mut popup = Popup::new(bar, anchor)
            .size(Vec2::new(
                metrics.width + padding.size().x,
                metrics.height + padding.size().y,
            ))
            .gap(if compact { 4.0 } else { 2.0 })
            .padding(padding)
            .style(surface);
        popup.key_target = Some(bar);
        let popup_id = Popup::id(ui, Id::new(bar));
        let mut events = cascade::Events::default();
        let root = popup.show(ui, &mut state.open, |ui| {
            events = cascade::body(
                ui,
                panel_id(0),
                &rows,
                &metrics,
                style,
                active,
                reveal,
                pointer_moved,
                access,
            );
        });
        let Some(root) = root else { return };
        let mut parent = root.rect;
        let mut extra = Vec::new();
        let mut level_index = 0;
        loop {
            if let Some(selected) = apply(
                state,
                level_index,
                root_or(items, compact, state, level_index),
                &events,
                ui,
            ) {
                output.selected = Some(selected);
                ui.context.close_popup();
                // The panels built before the choice was read close in this pass as well.
                for (layer, _) in &extra {
                    ui.context.remove_popup_hits(*layer);
                }
                state.open = false;
                return;
            }
            level_index += 1;
            if level_index >= state.depth {
                break;
            }
            let Some(items_here) = level(items, compact, state, level_index) else {
                state.depth = level_index;
                break;
            };
            let row = state.path.get(level_index - 1).and_then(|index| {
                events
                    .submenu_rows
                    .iter()
                    .find(|(i, _)| i == index)
                    .map(|(_, rect)| *rect)
            });
            let Some(row) = row else {
                state.depth = level_index;
                break;
            };
            let rows = cascade::rows(items_here);
            let metrics = paint::measure(ui, &rows, style);
            let size = Vec2::new(
                metrics.width + padding.size().x,
                metrics.height + padding.size().y,
            );
            let rect = cascade::submenu_rect(parent, row, size, padding, ui.context.viewport());
            let layer = panel_id(level_index);
            let active = active_id(state, level_index, &rows);
            let parent_row = state.path[level_index - 1];
            let access = cascade::Access {
                label: root_or(items, compact, state, level_index - 1)
                    .get(parent_row)
                    .map_or("", |item| item.text.as_str()),
                open: state
                    .path
                    .get(level_index)
                    .copied()
                    .filter(|_| state.depth > level_index + 1),
                ..Default::default()
            };
            let reveal = moved
                .then(|| {
                    state
                        .path
                        .get(level_index)
                        .copied()
                        .filter(|_| level_index + 1 == state.depth)
                })
                .flatten();
            events = cascade::panel(ui, layer, rect, surface, padding, |ui| {
                cascade::body(
                    ui,
                    layer,
                    &rows,
                    &metrics,
                    style,
                    active,
                    reveal,
                    pointer_moved,
                    access,
                )
            });
            extra.push((layer, rect));
            parent = rect;
        }
        if let Some(index) = ui.context.popups.index_of(popup_id) {
            ui.context.popups.branch[index].extra = extra;
        }
    }
}

impl Ui<'_> {
    /// A horizontal menu bar; see [`MenuBar`].
    pub fn menu_bar(&mut self, source: impl Hash, items: &[MenuItem]) -> MenuBarOutput {
        MenuBar::new(source, items).show(self)
    }
}
