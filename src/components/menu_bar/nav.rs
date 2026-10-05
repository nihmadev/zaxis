//! Menu state and navigation: which panels are open, keyboard and hover transitions.
use super::{cascade, Kind, MenuBar, MenuBarOutput, MenuItem};
use crate::components::{context_menu::ContextMenuItem, Ui};
use crate::winit::keyboard::KeyCode;
use crate::{Id, Vec2};

#[derive(Default)]
pub(crate) struct MenuBarState {
    pub(super) open: bool,
    /// Open title in bar mode.
    pub(super) menu: usize,
    /// Highlighted row at each level; level `n` shows the children of `path[n - 1]`.
    pub(super) path: Vec<usize>,
    /// Number of panels shown, at least one while open.
    pub(super) depth: usize,
    pub(super) keyboard: bool,
    pub(super) pointer: Option<Vec2>,
    pub(crate) last_frame: u64,
}
impl MenuBarState {
    pub(super) fn reset(&mut self, menu: usize) {
        self.menu = menu;
        self.path.clear();
        self.depth = 1;
    }
}

/// Items shown at `level`: the title's children (or the root list when compact), followed
/// down the highlighted path.
pub(super) fn level<'a>(
    root: &'a [MenuItem],
    compact: bool,
    state: &MenuBarState,
    level: usize,
) -> Option<&'a [MenuItem]> {
    if state.path.len() < level {
        return None;
    }
    let mut items = if compact {
        root
    } else {
        root.get(state.menu)?.children()?
    };
    for index in &state.path[..level] {
        items = items.get(*index)?.children()?;
    }
    Some(items)
}

pub(super) fn selectable(items: &[MenuItem]) -> Vec<usize> {
    (0..items.len())
        .filter(|i| items[*i].selectable())
        .collect()
}

pub(super) fn next_menu(items: &[MenuItem], from: usize, forward: bool) -> Option<usize> {
    let count = items.len();
    (1..count)
        .map(|step| {
            if forward {
                (from + step) % count
            } else {
                (from + count - step) % count
            }
        })
        .find(|i| items[*i].is_submenu())
}
pub(super) fn root_or<'a>(
    items: &'a [MenuItem],
    compact: bool,
    state: &MenuBarState,
    index: usize,
) -> &'a [MenuItem] {
    level(items, compact, state, index).unwrap_or(&[])
}

/// Row to draw highlighted: parents of an open submenu always, the deepest panel only
/// while the keyboard drives it (the pointer highlights by hover there, without a lag).
pub(super) fn active_id(
    state: &MenuBarState,
    level: usize,
    rows: &[ContextMenuItem],
) -> Option<Id> {
    if !(state.keyboard || level + 1 < state.depth) {
        return None;
    }
    state
        .path
        .get(level)
        .and_then(|index| rows.get(*index))
        .filter(|row| row.enabled)
        .and_then(|row| row.id)
}

/// Applies hover and clicks of one panel; returns the activated action.
pub(super) fn apply(
    state: &mut MenuBarState,
    level: usize,
    items: &[MenuItem],
    events: &cascade::Events,
    ui: &mut Ui<'_>,
) -> Option<Id> {
    if let Some(index) = events.hovered.filter(|i| *i < items.len()) {
        if state.path.get(level) != Some(&index) {
            state.path.truncate(level);
            state.path.push(index);
            state.depth = level + 1;
            ui.context.request_repaint();
        }
        if items[index].is_submenu() && state.depth == level + 1 {
            state.depth = level + 2;
            ui.context.request_repaint();
        }
    }
    if let Some(index) = events.collapsed {
        if state.path.get(level) == Some(&index) && state.depth > level + 1 {
            state.depth = level + 1;
            ui.context.request_repaint();
        }
    }
    let item = items.get(events.clicked?)?;
    let index = events.clicked?;
    match &item.kind {
        Kind::Submenu(_) => {
            state.path.truncate(level);
            state.path.push(index);
            state.depth = level + 2;
            ui.context.request_repaint();
            None
        }
        Kind::Action(id) => Some(*id),
        Kind::Separator => None,
    }
}

impl MenuBar<'_> {
    /// Keyboard queue of the open menu. Returns whether the highlight moved.
    pub(super) fn keys(
        &self,
        ui: &mut Ui<'_>,
        bar: Id,
        state: &mut MenuBarState,
        output: &mut MenuBarOutput,
    ) -> bool {
        let (items, compact) = (self.items, self.compact);
        let mut moved = false;
        for key in ui.context.combo_input.remove(&bar).unwrap_or_default() {
            let deepest = state.depth.saturating_sub(1);
            let Some(here) = level(items, compact, state, deepest) else {
                break;
            };
            let list = selectable(here);
            let current = state.path.get(deepest).copied();
            let position = current.and_then(|c| list.iter().position(|i| *i == c));
            let enter = |state: &mut MenuBarState| {
                let Some(index) = current.filter(|i| here[*i].is_submenu()) else {
                    return false;
                };
                let first = here[index]
                    .children()
                    .map(selectable)
                    .and_then(|l| l.first().copied());
                state.path.truncate(deepest + 1);
                state.path.extend(first);
                state.depth = deepest + 2;
                true
            };
            match key {
                KeyCode::ArrowDown | KeyCode::ArrowUp | KeyCode::Home | KeyCode::End
                    if !list.is_empty() =>
                {
                    let last = list.len() - 1;
                    let next = match (key, position) {
                        (KeyCode::Home, _) | (KeyCode::ArrowDown, None) => 0,
                        (KeyCode::End, _) | (KeyCode::ArrowUp, None) => last,
                        (KeyCode::ArrowDown, Some(p)) => (p + 1) % list.len(),
                        (_, Some(p)) => (p + last) % list.len(),
                        (_, None) => 0,
                    };
                    state.path.truncate(deepest);
                    state.path.push(list[next]);
                    moved = true;
                }
                KeyCode::ArrowRight => {
                    if !enter(state) {
                        if let (false, Some(next)) = (compact, next_menu(items, state.menu, true)) {
                            self.switch(state, next);
                        }
                    }
                    moved = true;
                }
                KeyCode::ArrowLeft => {
                    if deepest > 0 {
                        state.depth = deepest;
                        state.path.truncate(deepest);
                    } else if let (false, Some(prev)) =
                        (compact, next_menu(items, state.menu, false))
                    {
                        self.switch(state, prev);
                    }
                    moved = true;
                }
                KeyCode::Enter | KeyCode::Space => {
                    if enter(state) {
                        moved = true;
                    } else if let Some(Kind::Action(id)) = current
                        .map(|i| &here[i])
                        .filter(|item| item.selectable())
                        .map(|item| &item.kind)
                    {
                        output.selected = Some(*id);
                    }
                }
                _ => {}
            }
            state.keyboard = true;
        }
        if output.selected.is_some() {
            ui.context.close_popup();
            state.open = false;
        }
        moved
    }

    pub(super) fn switch(&self, state: &mut MenuBarState, menu: usize) {
        state.reset(menu);
        let first = self.items[menu]
            .children()
            .map(selectable)
            .and_then(|list| list.first().copied());
        state.path.extend(first);
    }
}
