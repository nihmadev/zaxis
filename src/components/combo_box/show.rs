use super::*;

impl<T: Clone + PartialEq> Widget for ComboBox<'_, T> {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        let id = match self.id {
            Some(id) => ui.scope.with(("combo-box", id)),
            None => ui.auto_id(("combo-box", self.source)),
        };
        let popup_id = Popup::id(ui, Id::new(id));
        let mut style = self.style.unwrap_or_else(|| ui.style().combo_box.clone());
        if let Some(width) = self.width {
            style.width = width;
        }
        if let Some(count) = self.visible_rows {
            style.visible_rows = count;
        }
        if let Some(duration) = self.duration {
            style.animation_duration = duration;
        }
        if let Some(padding) = self.padding {
            style.trigger_padding = padding;
        }
        if let Some(height) = self.row_height {
            style.row_height = height;
        }
        if let Some(height) = self.trigger_height {
            style.trigger_height = height;
        }
        if let Some(gap) = self.row_gap {
            style.row_gap = gap;
        }
        if let Some(gap) = self.label_gap {
            style.label_gap = gap;
        }
        if let Some(radius) = self.rounding {
            style.rounding = radius;
        }
        if let Some(padding) = self.popup_padding {
            style.popup_padding = padding;
        }
        if let Some(size) = self.font_size {
            style.font_size = size;
        }
        style.validate();
        self.enabled &= ui.enabled;
        let label = crate::components::visible_label(&self.label);
        let label_height = if label.is_empty() {
            0.0
        } else {
            style.label_height + style.label_gap
        };
        let allocation = ui.allocate_space(Vec2::new(
            style.width.min(ui.available_width()),
            style.trigger_height + label_height,
        ));
        let rect = Rect::from_min_max(
            allocation.min + Vec2::new(0.0, label_height),
            allocation.max,
        );
        let mut response = ui.response(id, rect, self.enabled);
        ui.context.register_hit(HitRegion {
            id,
            window: ui.window,
            rect,
            clip: ui.clip,
            action: if self.enabled {
                HitAction::ComboBox
            } else {
                HitAction::Block
            },
        });
        let existing = ui.context.combo_boxes.remove(&id);
        let mut was_open = existing.as_ref().is_some_and(|state| state.open);
        let mut state = existing.unwrap_or_else(|| ComboBoxState {
            open: self.default_open,
            ..Default::default()
        });
        state.last_frame = ui.context.frame;
        if ui.context.dismissed_popups.remove(&popup_id) {
            state.open = false;
            was_open = false;
        }
        let mut reveal = false;
        let mut options = std::mem::take(&mut state.options);
        let visible = !rect.intersect(ui.clip_rect()).is_empty();
        if !self.enabled || self.options.is_empty() || !visible {
            state.open = false;
        }
        if self.enabled && !self.options.is_empty() && visible && response.clicked() {
            state.open = !state.open;
        }
        let keys = ui.context.combo_input.remove(&id).unwrap_or_default();
        let mut navigation = Vec::new();
        for key in keys {
            if !self.enabled || self.options.is_empty() || !visible {
                break;
            }
            if !state.open {
                state.open = true;
                // Opening arrows start at the selected item (or the first enabled item).
                if matches!(
                    key,
                    KeyCode::ArrowDown | KeyCode::ArrowUp | KeyCode::Enter | KeyCode::Space
                ) {
                    continue;
                }
            }
            navigation.push(key);
        }
        let opening = state.open && !was_open;
        if opening {
            state.focus_filter = self.filterable;
            state.query.clear();
            state.active = self
                .options
                .iter()
                .find(|o| o.enabled && Some(&o.value) == self.selected.as_ref())
                .or_else(|| self.options.iter().find(|o| o.enabled))
                .map(|o| o.id);
            reveal = true;
        }
        if !self.filterable {
            state.query.clear();
        }
        let motion = TweenOptions::new(style.animation_duration).easing(Easing::CubicOut);
        let progress = ui
            .context
            .transition_visible(
                id.with("open"),
                Some(0.0),
                if state.open { 1.0 } else { 0.0 },
                motion.clone(),
                visible,
            )
            .value;
        if state.open || progress > 0.0 {
            reveal |= options.refresh(self.options);
        }
        let count = (if state.open || progress > 0.0 {
            options.matching(self.options, &state.query).len()
        } else {
            0
        })
        .max(1)
        .min(style.visible_rows.max(1));
        let pitch = style.row_height + style.row_gap;
        let filter_height = if self.filterable {
            ui.style().text_edit_height.max(30.0) + 2.0
        } else {
            0.0
        };
        let height =
            count as f32 * pitch - style.row_gap + style.popup_padding.size().y + filter_height;
        let mut popup = Popup::new(id, rect)
            .size(Vec2::new(rect.size().x, height))
            .gap(style.popup_gap)
            .padding(style.popup_padding)
            .rounding(style.rounding)
            .fill(style.popup_fill)
            .border(style.popup_border)
            .return_focus(id);
        popup.key_target = Some(id);
        popup.progress = progress;
        popup.animated = true;
        let mut choice = None;
        let mut active = state.active;
        let mut active_row = state.active_row;
        let mut list_size = state.list_size;
        let mut query = std::mem::take(&mut state.query);
        let open = state.open;
        let mut focus_filter = state.focus_filter;
        popup.show(ui, &mut state.open, |ui| {
            if self.filterable {
                let before = query.clone();
                let r = ui.add(
                    TextEdit::new(&mut query)
                        .id_source("filter")
                        .placeholder("Filter…")
                        .width(ui.available_width())
                        .height(filter_height - 2.0),
                );
                if focus_filter && ui.enabled && !r.rect.intersect(ui.clip_rect()).is_empty() {
                    ui.context.request_focus(r.id);
                    focus_filter = false;
                }
                reveal |= before != query;
            }
            let indices = options.matching(self.options, &query);
            let enabled: Vec<_> = indices
                .iter()
                .copied()
                .filter(|&i| self.options[i].enabled)
                .collect();
            if !enabled.iter().any(|&i| Some(self.options[i].id) == active) {
                active = enabled.first().map(|&i| self.options[i].id);
                reveal = true;
            }
            for key in &navigation {
                if !open {
                    break;
                }
                if matches!(key, KeyCode::Enter | KeyCode::Space) {
                    choice = enabled
                        .iter()
                        .copied()
                        .find(|&i| Some(self.options[i].id) == active);
                    break;
                }
                let current = enabled
                    .iter()
                    .position(|&i| Some(self.options[i].id) == active)
                    .unwrap_or(0);
                let last = enabled.len().saturating_sub(1);
                let next = match key {
                    KeyCode::ArrowDown => (current + 1).min(last),
                    KeyCode::ArrowUp => current.saturating_sub(1),
                    KeyCode::Home => 0,
                    KeyCode::End => last,
                    KeyCode::PageDown => (current + style.visible_rows).min(last),
                    KeyCode::PageUp => current.saturating_sub(style.visible_rows),
                    _ => current,
                };
                active = enabled.get(next).map(|&i| self.options[i].id);
                reveal = true;
            }
            let row = indices
                .iter()
                .position(|&i| Some(self.options[i].id) == active);
            reveal |= active_row != row;
            active_row = row;
            let size = Vec2::new(ui.available_width(), ui.available_height());
            reveal |= list_size != size;
            list_size = size;
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
                .max_height((height - style.popup_padding.size().y - filter_height).max(0.0))
                .show_hints(false)
                .middle_mouse_scroll(false);
            if reveal {
                if let Some(row) = indices
                    .iter()
                    .position(|&i| Some(self.options[i].id) == active)
                {
                    scroll = scroll.scroll_to_rect(Rect::from_min_size(
                        Vec2::new(0.0, row as f32 * pitch),
                        Vec2::new(1.0, style.row_height),
                    ));
                }
            }
            if indices.is_empty() {
                paint::empty(ui, &style);
            } else {
                let mut build_row = |ui: &mut Ui<'_>, row: usize| {
                    let index = indices[row];
                    let option = &self.options[index];
                    let row_id = id.with(("option", option.id));
                    if paint::option(
                        ui,
                        row_id,
                        &option.label,
                        Some(&option.value) == self.selected.as_ref(),
                        Some(option.id) == active,
                        option.enabled,
                        &style,
                    ) {
                        choice = Some(index);
                    }
                };
                if indices.len() <= style.visible_rows {
                    // Measured short lists exclude the final row gap: five rows
                    // fit without a spurious two-pixel scroll range/scrollbar.
                    scroll.show(ui, |ui| {
                        ui.layout.spacing = style.row_gap;
                        for row in 0..indices.len() {
                            build_row(ui, row);
                        }
                    });
                } else {
                    scroll.show_rows(ui, pitch, indices.len(), build_row);
                }
            }
        });
        state.query = query;
        state.focus_filter = state.open && focus_filter;
        state.active = active;
        state.active_row = active_row;
        state.list_size = list_size;
        state.options = options;
        if let Some(index) = choice.filter(|_| state.open) {
            let value = &self.options[index].value;
            if self.selected.as_ref() != Some(value) {
                *self.selected = Some(value.clone());
                response.changed = true;
            }
            state.open = false;
        }
        if !state.open && ui.context.popup.as_ref().is_some_and(|p| p.id == popup_id) {
            ui.context.dismiss_popup(true);
            ui.context.dismissed_popups.remove(&popup_id);
            ui.context
                .transition_visible(id.with("open"), None, 0.0_f32, motion.clone(), visible);
        }
        response.has_focus = ui.context.has_focus(id);
        response.focus_visible = ui.context.focus_visible(id);
        let layer = if state.open { popup_id } else { ui.window };
        response.hovered = ui.context.hovered(id, layer, rect, ui.clip_rect());
        let caption = self
            .options
            .iter()
            .find(|o| Some(&o.value) == self.selected.as_ref())
            .map_or(self.placeholder.as_str(), |o| o.label.as_str());
        paint::trigger(
            ui, response, label, allocation, caption, progress, &style, motion,
        );
        ui.context.combo_boxes.insert(id, state);
        response
    }
}
