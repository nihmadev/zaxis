//! Retained window bounds and layer ordering.

use super::{Context, Id};
use crate::{Rect, Vec2};

pub struct WindowState {
    pub root: bool,
    pub rect: Rect,
    pub displayed_rect: Rect,
    pub min_size: Vec2,
    pub last_frame: u64,
    /// Stays above windows that are not `on_top`, however they were raised.
    pub on_top: bool,
}

impl Context {
    pub(crate) fn root_state(&mut self, id: Id) {
        let rect = self.viewport();
        self.windows.insert(
            id,
            WindowState {
                root: true,
                rect,
                displayed_rect: rect,
                min_size: Vec2::ZERO,
                last_frame: self.frame,
                on_top: false,
            },
        );
        self.layers.retain(|layer| *layer != id);
        self.layers.insert(0, id);
    }

    pub(crate) fn raise_window(&mut self, id: Id) {
        if self.windows.get(&id).is_some_and(|state| state.root) {
            return;
        }
        self.layers.retain(|layer| *layer != id);
        self.layers.push(id);
        self.stack_on_top();
    }

    /// Keep `on_top` windows after all others, each group in its own order.
    fn stack_on_top(&mut self) {
        let windows = &self.windows;
        let top = |id: &Id| windows.get(id).is_some_and(|window| window.on_top);
        self.layers.sort_by_key(|id| top(id));
    }

    pub(crate) fn set_window_on_top(&mut self, id: Id, on_top: bool) {
        if let Some(state) = self
            .windows
            .get_mut(&id)
            .filter(|state| state.on_top != on_top)
        {
            state.on_top = on_top;
            self.stack_on_top();
        }
    }

    pub(crate) fn front_window(&self) -> Option<Id> {
        self.layers.iter().rev().copied().find(|id| {
            self.windows.get(id).is_some_and(|window| {
                self.visible_windows.contains(id) || window.last_frame == self.frame
            })
        })
    }

    pub(crate) fn window_state(&mut self, id: Id, initial: Rect, min_size: Vec2) -> Rect {
        if !self.windows.contains_key(&id) {
            self.layers.push(id);
            self.stack_on_top();
        }
        let state = self.windows.entry(id).or_insert(WindowState {
            root: false,
            rect: initial,
            displayed_rect: initial,
            min_size,
            last_frame: self.frame,
            on_top: false,
        });
        state.min_size = min_size;
        let size = state.rect.size().max(min_size);
        let max_position = (self.logical_size - Vec2::new(48.0, 32.0)).max(Vec2::ZERO);
        let position = state.rect.min.clamp(Vec2::ZERO, max_position);
        state.rect = Rect::from_min_size(position, size);
        state.last_frame = self.frame;
        state.rect
    }

    /// A captured title bar or resize grip follows the pointer: the window moves (kept
    /// reachable inside the viewport) or resizes (not below its minimum size).
    pub(super) fn drag_window(&mut self, capture: super::interaction::Capture, pointer: Vec2) {
        let Some(window) = self.windows.get_mut(&capture.hit.window) else {
            return;
        };
        let delta = pointer - capture.pointer;
        match capture.hit.action {
            super::HitAction::Move => {
                let max = (self.logical_size - Vec2::new(48.0, 32.0)).max(Vec2::ZERO);
                window.rect = Rect::from_min_size(
                    (capture.rect.min + delta).clamp(Vec2::ZERO, max),
                    capture.rect.size(),
                );
            }
            super::HitAction::Resize => {
                window.rect = Rect::from_min_size(
                    capture.rect.min,
                    (capture.rect.size() + delta).max(window.min_size),
                )
            }
            _ => {}
        }
    }

    pub(super) fn top_window(&self, pointer: Vec2) -> Option<Id> {
        if let Some(popup) = &self.popups.current {
            if self.viewport().contains(pointer) {
                if let Some((layer, _)) =
                    popup.extra.iter().rev().find(|(_, r)| r.contains(pointer))
                {
                    return Some(*layer);
                }
                return Some(popup.id);
            }
        }
        if let Some(modal) = self.modals.stack.last() {
            if self.viewport().contains(pointer) {
                return Some(modal.id);
            }
        }
        self.layers.iter().rev().copied().find(|id| {
            self.windows.get(id).is_some_and(|w| {
                (self.visible_windows.contains(id) || w.last_frame == self.frame)
                    && w.displayed_rect.contains(pointer)
            })
        })
    }
}
