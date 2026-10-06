//! Pointer dispatch: presses take the capture of the region under the pointer, captured
//! motion and the release go to that region's component, and each component turns them
//! into its own input. A press outside the open popup only closes it.

use super::{interaction::Capture, split::Phase, Context, HitAction, HitRegion, NumberInputEvent};
use crate::Vec2;
use winit::event::ElementState;

impl Context {
    pub(super) fn secondary_button(&mut self, state: ElementState) -> bool {
        if state == ElementState::Released {
            self.input.secondary_down = false;
            self.input.secondary_released = true;
            self.gesture_secondary_release();
            return self.popups.current.is_some();
        }
        if self.input.secondary_down {
            return self.popups.current.is_some();
        }
        self.input.secondary_down = true;
        self.input.secondary_pressed = true;
        self.drag_cancel(crate::components::drag_drop::DragReason::Cancelled);
        if self.popups.current.is_some() {
            self.dismiss_popup(true);
            return true;
        }
        if let Some(pointer) = self.input.pointer {
            if self.tree_context(pointer) {
                return true;
            }
            return self.gesture_secondary_press(pointer);
        }
        false
    }

    pub(super) fn primary_button(&mut self, state: ElementState) -> bool {
        match state {
            ElementState::Pressed => self.primary_press(),
            ElementState::Released => self.primary_release(),
        }
    }

    /// Returns whether the press went to something.
    fn primary_press(&mut self) -> bool {
        if self.input.primary_down {
            return self.interaction.capture.is_some();
        }
        self.input.primary_down = true;
        self.interaction.focus_visible = false;
        self.input.primary_pressed = true;
        self.interaction.keyboard_active = None;
        if self.press_outside_popup() {
            self.dismiss_popup(true);
            // Consume the whole gesture; never capture underlying content.
            return true;
        }
        let hit = self.input.pointer.and_then(|p| self.hit_test(p));
        if hit.is_none_or(|hit| hit.action != HitAction::TextEdit) {
            self.text_fields.clicks.reset();
        }
        self.selection_press_elsewhere(hit);
        self.drag_press(hit);
        let Some(hit) = hit else {
            self.set_focus(None);
            return false;
        };
        if self.windows.contains_key(&hit.window) {
            self.raise_window(hit.window);
        }
        let focus = match hit.action {
            HitAction::TreeRow { tree, .. } => Some(tree),
            _ => hit.action.focusable().then_some(hit.id),
        };
        if !(self.is_modal_layer(hit.window) && focus.is_none()) {
            self.set_focus(focus);
        }
        if hit.action != HitAction::Block {
            let pointer = self.input.pointer.unwrap();
            self.interaction.capture = Some(Capture {
                hit,
                pointer,
                rect: self.windows.get(&hit.window).map_or(hit.rect, |w| w.rect),
            });
            self.gesture_press(hit, pointer);
            self.begin_capture(hit, pointer);
        }
        true
    }

    /// The pointer is outside the open popup, its extra panels and (for a popup with a key
    /// target) its anchor.
    fn press_outside_popup(&self) -> bool {
        self.popups.current.as_ref().is_some_and(|popup| {
            self.input.pointer.is_some_and(|p| {
                !popup.rect.contains(p)
                    && !popup.extra.iter().any(|(_, rect)| rect.contains(p))
                    && (popup.key_target.is_none() || !popup.anchor.contains(p))
            })
        })
    }

    /// The press captured by `hit` starts the gesture of its component.
    fn begin_capture(&mut self, hit: HitRegion, pointer: Vec2) {
        match hit.action {
            HitAction::StaticText | HitAction::Link => self.selection_note_press(hit.id, pointer),
            HitAction::ScrollThumb { area, axis } => self.begin_scroll_drag(area, axis),
            HitAction::ColumnResize { table, column } => self.column_resize_press(table, column),
            HitAction::SplitResize { .. } => self.split_pointer(hit.id, pointer, Phase::Press),
            HitAction::TextEdit => self.text_press(hit.id, pointer),
            HitAction::Slider => self.slider_pointer(hit.id, pointer),
            HitAction::DragValue => {
                let event = NumberInputEvent::Press(pointer, self.input.modifiers);
                self.drag_value_pointer(hit.id, event);
            }
            _ => {}
        }
    }

    /// Returns whether the release ended a drag or a capture.
    fn primary_release(&mut self) -> bool {
        self.input.primary_down = false;
        self.input.primary_released = true;
        if self.drag_release() {
            return true;
        }
        let Some(capture) = self.interaction.capture.take() else {
            return false;
        };
        self.selection_note_release(capture.hit.id);
        if self
            .input
            .pointer
            .and_then(|p| self.hit_test(p))
            .is_some_and(|h| h.id == capture.hit.id)
        {
            self.tree_click(capture.hit);
        }
        self.end_capture(capture);
        self.gesture_release(capture.hit);
        true
    }

    /// The release ends the gesture of the captured component, at the pointer if it is
    /// still over the window.
    fn end_capture(&mut self, capture: Capture) {
        let hit = capture.hit;
        let pointer = self.input.pointer;
        match (hit.action, pointer) {
            (HitAction::SplitResize { .. }, _) => {
                self.split_pointer(hit.id, pointer.unwrap_or(capture.pointer), Phase::Release)
            }
            (HitAction::DragValue, Some(pointer)) => {
                let event = NumberInputEvent::Release(pointer, self.input.modifiers);
                self.drag_value_pointer(hit.id, event);
            }
            (HitAction::ColumnResize { table, column }, Some(pointer)) => {
                self.column_resize_move(table, column, pointer.x - capture.pointer.x)
            }
            (HitAction::ScrollThumb { area, axis }, Some(pointer)) => {
                self.drag_scroll(area, axis, pointer - capture.pointer)
            }
            (HitAction::Slider, Some(pointer)) => self.slider_pointer(hit.id, pointer),
            _ => {}
        }
    }

    pub(super) fn move_pointer(&mut self, pointer: Vec2) {
        self.input.pointer = Some(pointer);
        self.file_hover_pointer(pointer);
        self.update_auto_scroll_pointer(pointer);
        self.drag_move(pointer);
        if let Some(capture) = self.interaction.capture {
            self.gesture_move(pointer);
            self.drag_capture(capture, pointer);
        }
    }

    /// Captured motion goes to the component that holds the capture.
    fn drag_capture(&mut self, capture: Capture, pointer: Vec2) {
        let hit = capture.hit;
        match hit.action {
            HitAction::SplitResize { .. } => self.split_pointer(hit.id, pointer, Phase::Move),
            HitAction::DragValue => {
                let event = NumberInputEvent::Drag(pointer, self.input.modifiers);
                self.drag_value_pointer(hit.id, event);
            }
            HitAction::ColumnResize { table, column } => {
                self.column_resize_move(table, column, pointer.x - capture.pointer.x)
            }
            HitAction::ScrollThumb { area, axis } => {
                self.drag_scroll(area, axis, pointer - capture.pointer)
            }
            HitAction::TextEdit => self.text_drag(hit.id, pointer),
            HitAction::Slider => self.slider_pointer(hit.id, pointer),
            HitAction::Move | HitAction::Resize => self.drag_window(capture, pointer),
            _ => {}
        }
    }
}
