//! Pointer presses, capture, dragging, and resizing.

use super::{
    interaction::Capture, Context, HitAction, NumberInputEvent, SliderInput, TextEditInput,
};
use crate::{Rect, Vec2};
use winit::event::ElementState;

impl Context {
    pub(super) fn secondary_button(&mut self, state: ElementState) -> bool {
        if state == ElementState::Released {
            self.input.secondary_down = false;
            self.input.secondary_released = true;
            return self.popup.is_some();
        }
        if self.input.secondary_down {
            return self.popup.is_some();
        }
        self.input.secondary_down = true;
        self.input.secondary_pressed = true;
        self.secondary_target = None;
        if self.popup.is_some() {
            self.dismiss_popup(true);
            return true;
        }
        if let Some(pointer) = self.input.pointer {
            if self.tree_context(pointer) {
                return true;
            }
            let window = self.top_window(pointer);
            if let Some(hit) = self.previous_hits.iter().rev().find(|hit| {
                Some(hit.window) == window
                    && hit.action == HitAction::ContextMenu
                    && hit.rect.contains(pointer)
                    && hit.clip.contains(pointer)
            }) {
                self.secondary_target = Some((hit.id, pointer));
                return true;
            }
        }
        false
    }

    pub(super) fn primary_button(&mut self, state: ElementState) -> bool {
        match state {
            ElementState::Pressed => {
                if self.input.primary_down {
                    return self.capture.is_some();
                }
                self.input.primary_down = true;
                self.focus_visible = false;
                self.input.primary_pressed = true;
                self.keyboard_active = None;
                if self.popup.as_ref().is_some_and(|popup| {
                    self.input.pointer.is_some_and(|p| {
                        !popup.rect.contains(p)
                            && (popup.key_target.is_none() || !popup.anchor.contains(p))
                    })
                }) {
                    self.dismiss_popup(true);
                    // Consume the whole gesture; never capture underlying content.
                    return true;
                }
                let hit = self.input.pointer.and_then(|p| self.hit_test(p));
                if hit.is_none_or(|hit| hit.action != HitAction::TextEdit) {
                    self.text_click = None;
                }
                if let Some(hit) = hit {
                    if self.windows.contains_key(&hit.window) {
                        self.raise_window(hit.window);
                    }
                    let focus = match hit.action {
                        HitAction::TreeRow { tree, .. } => Some(tree),
                        _ => hit.action.focusable().then_some(hit.id),
                    };
                    self.set_focus(focus);
                    if hit.action != HitAction::Block {
                        self.capture = Some(Capture {
                            hit,
                            pointer: self.input.pointer.unwrap(),
                            rect: self.windows.get(&hit.window).map_or(hit.rect, |w| w.rect),
                        });
                        if let HitAction::ScrollThumb { area, axis } = hit.action {
                            self.begin_scroll_drag(area, axis);
                        }
                        if let HitAction::ColumnResize { table, column } = hit.action {
                            if let Some(state) = self.tables.get_mut(&table) {
                                if let Some(&width) = state.current_widths.get(&column) {
                                    state.drag = Some((column, width));
                                }
                            }
                        }
                        if matches!(hit.action, HitAction::SplitResize { .. }) {
                            self.split_pointer(hit.id, self.input.pointer.unwrap(), 1);
                        }
                        if hit.action == HitAction::TextEdit {
                            let pointer = self.input.pointer.unwrap();
                            let now = std::time::Instant::now();
                            let count = self.text_click.as_ref().map_or(1, |last| {
                                if last.id == hit.id
                                    && now.duration_since(last.time)
                                        <= std::time::Duration::from_millis(500)
                                    && (pointer - last.position).length_squared() <= 16.0
                                    && !self.input.modifiers.shift_key()
                                {
                                    last.count % 3 + 1
                                } else {
                                    1
                                }
                            });
                            self.text_click = Some(super::interaction::ClickSequence {
                                id: hit.id,
                                position: pointer,
                                time: now,
                                count,
                            });
                            self.text_edit_input.entry(hit.id).or_default().push(
                                TextEditInput::Pointer(
                                    pointer,
                                    self.input.modifiers.shift_key(),
                                    count,
                                ),
                            );
                        }
                        if hit.action == HitAction::Slider {
                            self.slider_input
                                .entry(hit.id)
                                .or_default()
                                .push(SliderInput::Pointer(self.input.pointer.unwrap()));
                        }
                        if hit.action == HitAction::DragValue {
                            self.number_input.entry(hit.id).or_default().push(
                                NumberInputEvent::Pointer(
                                    self.input.pointer.unwrap(),
                                    1,
                                    self.input.modifiers,
                                ),
                            );
                        }
                    }
                } else {
                    self.set_focus(None);
                }
                hit.is_some()
            }
            ElementState::Released => {
                self.input.primary_down = false;
                self.input.primary_released = true;
                if let Some(capture) = self.capture.take() {
                    if self
                        .input
                        .pointer
                        .and_then(|p| self.hit_test(p))
                        .is_some_and(|h| h.id == capture.hit.id)
                    {
                        self.tree_click(capture.hit);
                    }
                    if matches!(capture.hit.action, HitAction::SplitResize { .. }) {
                        self.split_pointer(
                            capture.hit.id,
                            self.input.pointer.unwrap_or(capture.pointer),
                            2,
                        );
                    }
                    if capture.hit.action == HitAction::DragValue {
                        if let Some(pointer) = self.input.pointer {
                            self.number_input
                                .entry(capture.hit.id)
                                .or_default()
                                .push(NumberInputEvent::Pointer(pointer, 2, self.input.modifiers));
                        }
                    }
                    if let HitAction::ColumnResize { table, column } = capture.hit.action {
                        if let Some(pointer) = self.input.pointer {
                            self.resize_column(table, column, pointer.x - capture.pointer.x);
                        }
                    }
                    if let HitAction::ScrollThumb { area, axis } = capture.hit.action {
                        if let Some(pointer) = self.input.pointer {
                            self.drag_scroll(area, axis, pointer - capture.pointer);
                        }
                    }
                    if capture.hit.action == HitAction::Slider {
                        if let Some(pointer) = self.input.pointer {
                            self.slider_input
                                .entry(capture.hit.id)
                                .or_default()
                                .push(SliderInput::Pointer(pointer));
                        }
                    }
                    if matches!(
                        capture.hit.action,
                        HitAction::Activate | HitAction::ComboBox
                    ) && self
                        .input
                        .pointer
                        .and_then(|p| self.hit_test(p))
                        .is_some_and(|h| h.id == capture.hit.id)
                    {
                        self.clicked.insert(capture.hit.id);
                    }
                    true
                } else {
                    false
                }
            }
        }
    }

    pub(super) fn move_pointer(&mut self, pointer: Vec2) {
        self.input.pointer = Some(pointer);
        self.update_auto_scroll_pointer(pointer);
        if let Some(capture) = self.capture {
            if matches!(capture.hit.action, HitAction::SplitResize { .. }) {
                self.split_pointer(capture.hit.id, pointer, 0);
            }
            if capture.hit.action == HitAction::DragValue {
                self.number_input
                    .entry(capture.hit.id)
                    .or_default()
                    .push(NumberInputEvent::Pointer(pointer, 0, self.input.modifiers));
            }
            if let HitAction::ColumnResize { table, column } = capture.hit.action {
                self.resize_column(table, column, pointer.x - capture.pointer.x);
            }
            if let HitAction::ScrollThumb { area, axis } = capture.hit.action {
                self.drag_scroll(area, axis, pointer - capture.pointer);
            }
            if capture.hit.action == HitAction::TextEdit {
                if (pointer - capture.pointer).length_squared() > 16.0 {
                    self.text_click = None;
                }
                self.text_edit_input
                    .entry(capture.hit.id)
                    .or_default()
                    .push(TextEditInput::Pointer(pointer, true, 0));
            }
            if capture.hit.action == HitAction::Slider {
                self.slider_input
                    .entry(capture.hit.id)
                    .or_default()
                    .push(SliderInput::Pointer(pointer));
            }
            if let Some(window) = self.windows.get_mut(&capture.hit.window) {
                let delta = pointer - capture.pointer;
                match capture.hit.action {
                    HitAction::Move => {
                        let max = (self.logical_size - Vec2::new(48.0, 32.0)).max(Vec2::ZERO);
                        window.rect = Rect::from_min_size(
                            (capture.rect.min + delta).clamp(Vec2::ZERO, max),
                            capture.rect.size(),
                        );
                    }
                    HitAction::Resize => {
                        window.rect = Rect::from_min_size(
                            capture.rect.min,
                            (capture.rect.size() + delta).max(window.min_size),
                        )
                    }
                    _ => {}
                }
            }
        }
    }
    fn resize_column(&mut self, table: super::Id, column: super::Id, delta: f32) {
        if let Some((dragged, width)) = self.tables.get(&table).and_then(|state| state.drag) {
            if dragged == column {
                self.column_resize
                    .insert((table, column), (width + delta).max(0.0));
            }
        }
    }
}
