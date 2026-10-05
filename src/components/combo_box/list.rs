//! The popup and the list in it: the filter field, the highlight kept on a navigable row
//! and moved by the keys, and the rows, only those in view once there are more than
//! `visible_rows`. A highlight that moved, or a list that changed, is scrolled into view.

use super::{
    access, choice::Choice, nav::Navigable, options::OptionsState, paint, trigger::Trigger,
    ComboBoxOption, ComboBoxState, ComboBoxStyle,
};
use crate::components::{Popup, ScrollArea, ScrollStyle, TextEdit, Ui};
use crate::{Id, Padding, Rect, Vec2};
use winit::keyboard::KeyCode;

/// What the list keeps between passes besides the highlight and the query.
#[derive(Default)]
pub(super) struct ListState {
    /// The filter takes focus once the popup shows it.
    pub(super) focus_filter: bool,
    /// The highlighted row and the list's size when last shown: a change scrolls the
    /// highlight into view.
    active_row: Option<usize>,
    size: Vec2,
    options: OptionsState,
}

/// What the list shows.
pub(super) struct List<'s, T> {
    pub(super) trigger: &'s Trigger,
    pub(super) label: &'s str,
    pub(super) options: &'s [ComboBoxOption<T>],
    pub(super) selected: Option<&'s T>,
    pub(super) filterable: bool,
}

/// The popup's measurements for one pass.
struct Geometry {
    /// A row and the gap below it.
    pitch: f32,
    /// The filter field and its margin; zero without a filter.
    filter: f32,
    /// The whole popup.
    height: f32,
}

impl Geometry {
    /// Room for `rows` rows, at least one (the note when nothing matches) and at most
    /// `visible_rows`, under the filter.
    fn new(ui: &Ui<'_>, style: &ComboBoxStyle, rows: usize, filterable: bool) -> Self {
        let count = rows.max(1).min(style.visible_rows.max(1));
        let pitch = style.row_height + style.row_gap;
        let filter = if filterable {
            ui.style().text_edit_height.max(30.0) + 2.0
        } else {
            0.0
        };
        let height = count as f32 * pitch - style.row_gap + style.popup_padding.size().y + filter;
        Self {
            pitch,
            filter,
            height,
        }
    }

    /// The rows' room below the filter.
    fn list_height(&self, style: &ComboBoxStyle) -> f32 {
        (self.height - style.popup_padding.size().y - self.filter).max(0.0)
    }
}

/// The retained state the open list edits.
struct View<'a> {
    query: &'a mut String,
    active: &'a mut Option<Id>,
    list: &'a mut ListState,
}

/// The filter field, when there is one.
#[derive(Default)]
struct Filter {
    /// Its accessibility node.
    node: Option<usize>,
    /// The query changed.
    changed: bool,
}

impl<T: PartialEq> List<'_, T> {
    /// Builds the popup while it is open or animating closed. `reveal` scrolls the
    /// highlight into view; `keys` move it if the popup was open when they arrived.
    /// Returns the choice made in the list.
    pub(super) fn show(
        &self,
        ui: &mut Ui<'_>,
        state: &mut ComboBoxState,
        progress: f32,
        mut reveal: bool,
        keys: &[KeyCode],
    ) -> Option<Choice> {
        let rows = if state.open || progress > 0.0 {
            reveal |= self.observe(ui, &mut state.list.options);
            state
                .list
                .options
                .matching(self.options, &state.query)
                .len()
        } else {
            0
        };
        let geometry = Geometry::new(ui, &self.trigger.style, rows, self.filterable);
        // Keys only move the highlight of a list that was open before the popup is built.
        let keys = if state.open { keys } else { &[] };
        let view = View {
            query: &mut state.query,
            active: &mut state.active,
            list: &mut state.list,
        };
        let choice = self
            .popup(&geometry, progress)
            .show(ui, &mut state.open, |ui| {
                self.content(ui, view, &geometry, keys, reveal)
            })
            .and_then(|shown| shown.inner);
        // A closed popup no longer waits to focus its filter.
        state.list.focus_filter &= state.open;
        choice
    }

    /// The cache follows the options while the list shows; duplicate ids are reported on
    /// every such pass. Returns whether the options changed.
    fn observe(&self, ui: &mut Ui<'_>, cache: &mut OptionsState) -> bool {
        let changed = cache.refresh(self.options);
        if cache.duplicates {
            ui.context.report(
                crate::DiagnosticKind::IdCollision,
                Some(self.trigger.id),
                Some(self.trigger.rect),
                || "duplicate id: two ComboBox options share one id".into(),
            );
        }
        changed
    }

    fn popup(&self, geometry: &Geometry, progress: f32) -> Popup {
        let (trigger, style) = (self.trigger, &self.trigger.style);
        let mut popup = Popup::new(trigger.id, trigger.rect)
            .size(Vec2::new(trigger.rect.size().x, geometry.height))
            .gap(style.popup_gap)
            .padding(style.popup_padding)
            .corner_radius(style.rounding)
            .fill(style.popup_fill)
            .border(style.popup_border)
            .return_focus(trigger.id);
        popup.key_target = Some(trigger.id);
        popup.progress = progress;
        popup.animated = true;
        popup
    }

    /// The popup's content: the filter; the highlight kept on a navigable row, then moved
    /// by the keys; the rows. Returns a click on a row, else Enter or Space.
    fn content(
        &self,
        ui: &mut Ui<'_>,
        view: View<'_>,
        geometry: &Geometry,
        keys: &[KeyCode],
        mut reveal: bool,
    ) -> Option<Choice> {
        let filter = self.filter(ui, view.query, &mut view.list.focus_filter, geometry);
        reveal |= filter.changed;
        let listed = view.list.options.matching(self.options, view.query);
        let navigable = Navigable::new(self.options, listed);
        reveal |= navigable.restore(view.active);
        let keyboard = navigable.navigate(keys, self.trigger.style.visible_rows, view.active);
        reveal |= keyboard.moved;
        let row = listed
            .iter()
            .position(|&i| Some(self.options[i].id) == *view.active);
        reveal |= view.list.active_row != row;
        view.list.active_row = row;
        let size = Vec2::new(ui.available_width(), ui.available_height());
        reveal |= view.list.size != size;
        view.list.size = size;
        // The filter holds focus while it is shown: it says which option is highlighted.
        if let Some(node) = filter.node.and_then(|at| ui.context.a11y_node_mut(at)) {
            access::popup_owner(node, self.trigger.id, self.trigger.list, *view.active);
        }
        let clicked = self.rows(ui, listed, *view.active, geometry, row.filter(|_| reveal));
        clicked
            .map(Choice::Pointer)
            .or(keyboard.chosen.map(Choice::Keyboard))
    }

    /// The filter field above the rows; it takes focus once the popup shows it.
    fn filter(
        &self,
        ui: &mut Ui<'_>,
        query: &mut String,
        focus: &mut bool,
        geometry: &Geometry,
    ) -> Filter {
        if !self.filterable {
            return Filter::default();
        }
        let before = query.clone();
        let at = ui.context.a11y_len();
        let field = ui.add(
            TextEdit::new(query)
                .id_source("filter")
                .placeholder("Filter…")
                .width(ui.available_width())
                .height(geometry.filter - 2.0),
        );
        if *focus && ui.enabled && !field.rect.intersect(ui.clip_rect()).is_empty() {
            ui.context.request_focus(field.id);
            *focus = false;
        }
        Filter {
            node: (ui.context.a11y_len() > at).then_some(at),
            changed: before != *query,
        }
    }

    /// The `listed` options in a scroll area that builds only the rows in view once there
    /// are more than `visible_rows`; `reveal` scrolls to that row. Returns the option
    /// clicked.
    fn rows(
        &self,
        ui: &mut Ui<'_>,
        listed: &[usize],
        active: Option<Id>,
        geometry: &Geometry,
        reveal: Option<usize>,
    ) -> Option<usize> {
        let (trigger, style) = (self.trigger, &self.trigger.style);
        let list = ui.a11y_begin(trigger.list, crate::AccessRole::ListBox, |node| {
            node.label(self.label);
        });
        let scroll_style = ScrollStyle {
            padding: Padding::default(),
            spacing: 0.0,
            bar_margin: 0.0,
            bar_width: 4.0,
            ..ui.style().scroll
        };
        let mut scroll = ScrollArea::vertical()
            .id_source("options")
            .style(scroll_style)
            .overlay_scrollbars(true)
            .max_height(geometry.list_height(style))
            .show_hints(false)
            .middle_mouse_scroll(false)
            .a11y_hidden();
        if let Some(row) = reveal {
            scroll = scroll.scroll_to_rect(Rect::from_min_size(
                Vec2::new(0.0, row as f32 * geometry.pitch),
                Vec2::new(1.0, style.row_height),
            ));
        }
        let mut clicked = None;
        let (area, viewport) = if listed.is_empty() {
            (trigger.list, paint::empty(ui, style))
        } else {
            let mut build_row = |ui: &mut Ui<'_>, row: usize| {
                let index = listed[row];
                let option = &self.options[index];
                if paint::option(
                    ui,
                    trigger.id.with(("option", option.id)),
                    &option.label,
                    Some(&option.value) == self.selected,
                    Some(option.id) == active,
                    option.enabled,
                    (row, listed.len()),
                    style,
                ) {
                    clicked = Some(index);
                }
            };
            if listed.len() <= style.visible_rows {
                // Measured short lists exclude the final row gap: five rows
                // fit without a spurious two-pixel scroll range/scrollbar.
                let shown = scroll.show(ui, |ui| {
                    ui.layout.spacing = style.row_gap;
                    for row in 0..listed.len() {
                        build_row(ui, row);
                    }
                });
                (shown.id, shown.viewport)
            } else {
                let shown = scroll.show_rows(ui, geometry.pitch, listed.len(), build_row);
                (shown.id, shown.viewport)
            }
        };
        ui.context.a11y_scroll(&list, area);
        ui.a11y_end(list, Some(viewport));
        clicked
    }
}
