use super::{
    heights::{footer_key, Sizes},
    model::{FnModel, ListEntry, ListModel, SliceModel},
    nav::KeyInput,
    paint::Look,
    row::{text_row, Content, ListRow, Pass},
    select::Selector,
    state::ListState,
    ListBox, ListEvent, ListMode, ListOutput,
};
use crate::{
    components::{
        scroll_area::{RowMetrics, RowPass},
        Sense, Ui,
    },
    context::invalid_value,
    Id, Padding, Rect, RowDrag, ScrollArea, ScrollStyle, Shape, Vec2,
};
use std::{collections::HashSet, time::Duration};

/// Entries within this many rows of the end request more data.
const PREFETCH: usize = 5;

impl ListBox<'_> {
    /// Show the list. `content` builds the visible item rows only; headers, separators and
    /// the loading row are drawn by the list.
    pub fn show(
        self,
        ui: &mut Ui<'_>,
        model: &impl ListModel,
        mut content: impl FnMut(&mut Ui<'_>, ListRow<'_>),
    ) -> ListOutput {
        ui.layout_item(|ui| self.run(ui, model, &mut content))
    }
    /// Rows that show the entry's text.
    pub fn show_text(self, ui: &mut Ui<'_>, model: &impl ListModel) -> ListOutput {
        self.show(ui, model, text_row)
    }
    /// `len` entries described by a closure, evaluated for the visible rows (and once per
    /// [`revision`](Self::revision) for all of them).
    pub fn show_rows<'m>(
        self,
        ui: &mut Ui<'_>,
        len: usize,
        entry: impl Fn(usize) -> ListEntry<'m>,
        content: impl FnMut(&mut Ui<'_>, ListRow<'_>),
    ) -> ListOutput {
        let model = FnModel {
            len,
            revision: self.revision,
            entry,
            borrowed: std::marker::PhantomData,
        };
        self.show(ui, &model, content)
    }
    /// A slice, with `entry` naming each item's key, kind and text.
    pub fn show_slice<T>(
        self,
        ui: &mut Ui<'_>,
        items: &[T],
        entry: impl for<'x> Fn(&'x T) -> ListEntry<'x>,
        mut content: impl FnMut(&mut Ui<'_>, ListRow<'_>, &T),
    ) -> ListOutput {
        let model = SliceModel {
            items,
            revision: self.revision,
            entry,
        };
        self.show(ui, &model, |ui, row| {
            let item = &items[row.index];
            content(ui, row, item)
        })
    }

    fn run<M: ListModel>(mut self, ui: &mut Ui<'_>, model: &M, content: Content<'_>) -> ListOutput {
        let id = ui.scope.with(("list_box", self.id));
        let mut state = ui
            .context
            .containers
            .list_boxes
            .remove(&id)
            .unwrap_or_default();
        if state.last_frame == ui.context.frame {
            ui.context
                .report(crate::DiagnosticKind::IdCollision, Some(id), None, || {
                    "duplicate id: two ListBoxes share one id_source".into()
                });
        }
        state.last_frame = ui.context.frame;
        let scale = match ui.context.scale_factor() {
            s if s.is_finite() && s > 0.0 => s,
            _ => 1.0,
        };
        let mut style = ui.style().list_box.clone();
        style.merge(&self.style);
        for (what, value) in [
            ("ListBoxStyle::row_height", &mut style.row_height),
            ("ListBoxStyle::header_height", &mut style.header_height),
            (
                "ListBoxStyle::separator_height",
                &mut style.separator_height,
            ),
        ] {
            if value.is_some_and(|v| !(v.is_finite() && v > 0.0)) {
                invalid_value(
                    what,
                    format!("expected a finite value > 0, got {value:?}; using the default"),
                );
                *value = None;
            }
        }
        let look = Look::resolve(
            ui.style(),
            &style,
            self.density.factor(),
            self.measured,
            scale,
        );
        let len = model.len();
        let sizes = Sizes {
            pitch: look.row,
            header: look.header,
            separator: look.separator,
            measured: self.measured.is_some(),
            footer: self.loading,
        };
        let rebuilt = state.heights.sync(model, sizes);
        if rebuilt {
            state.settle_active(model);
        }
        let mut heights = std::mem::take(&mut state.heights);
        let enabled = self.enabled && ui.enabled;
        let outer = Rect::from_min_size(
            ui.layout.cursor,
            Vec2::new(
                ui.available_width().max(0.0),
                ui.available_height().min(self.height).max(0.0),
            ),
        );
        // The single Tab stop. Registered before the rows, so rows and controls win the pointer.
        let owner = ui.add_enabled_ui(enabled, |ui| {
            ui.interact(outer, (id, "owner"), Sense::FOCUS)
        });
        let focused = owner.has_focus && enabled;
        // Pressing a row clears focus for one pass before the list takes it back; the look
        // (selection color, ring) must not blink in between.
        let held = !focused
            && enabled
            && state.owner_focused
            && ui.context.focused().is_none()
            && ui.context.input().primary_down
            && ui
                .context
                .input()
                .pointer
                .is_some_and(|p| outer.contains(p));
        let shown_focus = focused || held;
        let ring = if focused {
            owner.focus_visible
        } else {
            held && state.ring
        };
        state.ring = ring;
        let mut owned = std::mem::take(&mut state.owned);
        let set: &mut HashSet<Id> = match self.selection.take() {
            Some(set) => set,
            None => &mut owned,
        };
        let mut events = Vec::new();
        let mut sel = Selector {
            model,
            mode: self.mode,
            set,
            state: &mut state,
            events: &mut events,
        };
        if focused && !sel.state.owner_focused && sel.state.active.is_none() {
            let selected = |i: &usize| {
                let e = model.entry(*i);
                e.selectable() && sel.set.contains(&e.key)
            };
            let first = (0..len)
                .find(selected)
                .or_else(|| (0..len).find(|i| model.entry(*i).selectable()));
            if let Some(i) = first {
                sel.state.set_active(i, model);
            }
        }
        sel.state.owner_focused = focused;
        if focused {
            let input = ui.context.input();
            let key_input = KeyInput {
                keys: &input.keys_pressed,
                mods: input.modifiers,
                text: &input.text,
                now: ui.context.frame_time(),
                timeout: style
                    .type_ahead_timeout
                    .unwrap_or(Duration::from_millis(800)),
                follows: self
                    .follows
                    .unwrap_or(matches!(self.mode, ListMode::Single | ListMode::Multiple)),
            };
            let view = sel.state.viewport_height;
            let page = |from: usize, down: bool| {
                let top = heights.top(from);
                heights.row_at(if down { top + view } else { top - view })
            };
            sel.keyboard(&key_input, page);
        }
        let retarget = sel.state.retarget.take();
        let target_key = match self.scroll_to {
            Some(key) => Some((key, 0)),
            None if sel.state.reveal => sel.state.active.map(|key| (key, 0)),
            None => retarget,
        };
        let target = target_key.and_then(|(key, _)| ListState::find(model, key, 0));
        sel.state.reveal = false;

        if self.drag {
            keep_dragged(ui, id, model, sel.state.drag_hint);
        }
        let mut scroll = ScrollArea::vertical()
            .id_source(id)
            .max_height(self.height)
            .overlay_scrollbars(self.overlay_bars)
            .style(style.scroll.unwrap_or(ScrollStyle {
                padding: Padding::default(),
                spacing: 0.0,
                ..ui.style().scroll
            }));
        if let Some(i) = target.filter(|i| *i < heights.count()) {
            let top = heights.top(i);
            scroll = scroll.scroll_to_rect(Rect::from_min_size(
                Vec2::new(0.0, top),
                Vec2::new(1.0, heights.height(i)),
            ));
        } else if let Some((key, within)) = sel.state.scroll_anchor.filter(|_| rebuilt) {
            // The entries changed: keep the row that was at the top where it was.
            let hint = sel.state.scroll_hint;
            if let Some(i) = ListState::find(model, key, hint) {
                scroll = scroll.scroll_offset(Vec2::new(0.0, heights.top(i) + within));
            }
        }
        if style.surface.is_some() || style.border.is_some() {
            let shape = Shape::rect(outer, style.surface.unwrap_or(crate::Color::TRANSPARENT))
                .corner_radius(look.rounding)
                .border(style.border.unwrap_or(crate::Border::NONE));
            ui.paint(shape);
        }

        // The list is one node that scrolls; its rows are options inside it.
        let access = ui.a11y_begin(owner.id, crate::AccessRole::ListBox, |node| {
            node.label(self.label.as_str())
                .multiselectable(matches!(self.mode, ListMode::Multiple | ListMode::Checks))
                .busy(self.loading)
                .disabled(!enabled);
        });
        let plain = if ui.context.a11y_on() {
            heights.plain().to_vec()
        } else {
            Vec::new()
        };
        let mut pass = Pass {
            sel,
            id,
            owner: owner.id,
            look: &look,
            enabled,
            focused: shown_focus,
            focus_visible: ring,
            measured: self.measured.is_some(),
            drag: self.drag,
            loading: self.loading,
            empty: &self.empty,
            built: 0,
            plain,
            active_node: None,
        };
        let out = ui.add_enabled_ui(enabled, |ui| {
            scroll.a11y_hidden().show_rows_laid_out(
                ui,
                &mut heights,
                |index| {
                    if index < len {
                        model.entry(index).key
                    } else {
                        footer_key()
                    }
                },
                |ui, step| match step {
                    RowPass::Row { index, bounds } => pass.row(ui, index, bounds, &mut *content),
                    RowPass::Overlay { visible, rows } => {
                        pass.overlay(ui, visible, rows);
                        None
                    }
                },
            )
        });
        let Pass {
            sel,
            built: rows_built,
            active_node,
            ..
        } = pass;
        ui.context.a11y_scroll(&access, out.id);
        if let Some((index, row)) = access.0.zip(active_node) {
            if let Some(node) = ui.context.a11y_node_mut(index as usize) {
                node.active_descendant(row);
            }
        }
        ui.a11y_end(access, Some(outer));

        // Bookkeeping for the next pass.
        let offset = out.offset.y;
        let first = heights.row_at(offset).min(len.saturating_sub(1));
        let last = heights.row_at(offset + out.viewport.size().y);
        let visible = if len == 0 {
            0..0
        } else {
            first..(last + 1).min(len)
        };
        sel.state.viewport_height = out.viewport.size().y;
        // Measuring moved the row after the scroll position was chosen: aim again.
        if let (Some((key, tries)), Some(i)) = (target_key, target) {
            let (top, bottom) = (heights.top(i), heights.top(i + 1));
            let seen = top >= offset - 0.5 && bottom <= offset + out.viewport.size().y + 0.5;
            if self.measured.is_some() && !seen && tries < 4 {
                sel.state.retarget = Some((key, tries + 1));
                ui.context.request_repaint();
            }
        }
        sel.state.scroll_hint = first;
        sel.state.scroll_anchor = (offset > 0.0 && len > 0)
            .then(|| (model.entry(first).key, offset - heights.top(first)));
        if (out.inner.start..out.inner.end.min(len)).any(|i| heights.stale(i, model.entry(i).key)) {
            heights.invalidate();
            ui.context.request_repaint();
        }
        if self.has_more
            && !self.loading
            && len > 0
            && visible.end + PREFETCH >= len
            && sel.state.loaded_len != Some(len)
        {
            sel.state.loaded_len = Some(len);
            sel.events.push(ListEvent::LoadMore);
        }
        let active = sel.state.active;
        state.heights = heights;
        state.owned = owned;
        if !events.is_empty() {
            ui.context.request_repaint();
        }
        ui.context.containers.list_boxes.insert(id, state);
        ListOutput {
            id: owner.id,
            events,
            active,
            visible,
            len,
            rows_built,
            rebuilt,
            focused,
            viewport: out.viewport,
            scroll_offset: out.offset,
        }
    }
}

/// A row scrolled out of the virtual window is still being dragged while the model has it.
fn keep_dragged(ui: &mut Ui<'_>, list: Id, model: &impl ListModel, hint: usize) {
    let dragged = ui
        .context
        .drag
        .session
        .as_ref()
        .and_then(|session| session.payload.as_ref())
        .and_then(|payload| payload.get::<RowDrag>().copied())
        .filter(|dragged| dragged.owner == list);
    if let Some(dragged) = dragged {
        if ListState::find(model, dragged.row, hint).is_some() {
            ui.keep_drag_source(dragged.row);
        }
    }
}
