//! Modal stack. Modals are ordinary popup layers (same paint order, same hit
//! test through `top_window`) that additionally own input until they close.
//!
//! Rules, in one place:
//! * The newest open modal is the only input target. Pointer, wheel, keys, text,
//!   IME and shortcuts read through `Context::input` never reach what is under it.
//! * Popups opened from inside a modal draw above it; popups under it are closed
//!   when it opens and cannot be reopened until it closes.
//! * Focus is saved on open, trapped by Tab, and restored on close if the
//!   widget still exists; otherwise focus is cleared.
use super::{Context, HitAction, HitRegion, Id, InputState, WindowState};
use crate::{components::drag_drop::DragReason, Vec2};
use std::collections::{HashMap, HashSet};
use winit::{event::ElementState, keyboard::KeyCode};

pub(crate) struct ModalState {
    pub id: Id,
    pub return_focus: Option<Id>,
    pub pending_focus: bool,
    pub escape: bool,
    pub enter: bool,
    pub last_frame: u64,
}

#[derive(Default)]
pub(crate) struct Modals {
    /// Open modals, oldest first. Closing modals leave the stack immediately.
    pub stack: Vec<ModalState>,
    /// Modals whose content is being built right now, outermost first.
    pub building: Vec<Id>,
    pub close_requested: Option<Id>,
    /// Modal that received an Enter default action this pass.
    pub entered: Option<Id>,
    pub measures: HashMap<Id, crate::components::modal::Measure>,
    depth: HashMap<Id, usize>,
    registered: HashSet<Id>,
    blocked_input: InputState,
}

impl Modals {
    pub(crate) fn begin_pass(&mut self) {
        self.building.clear();
        self.depth.clear();
        self.close_requested = None;
        self.entered = None;
    }
}

impl Context {
    pub(crate) fn modal_active(&self) -> bool {
        !self.modals.stack.is_empty()
    }
    pub(crate) fn is_modal_layer(&self, id: Id) -> bool {
        self.modals.stack.iter().any(|modal| modal.id == id)
    }
    pub(crate) fn top_modal_id(&self) -> Option<Id> {
        self.modals.stack.last().map(|modal| modal.id)
    }
    /// True while the top modal's content (or a popup in it) is being built.
    pub(crate) fn building_top_modal(&self) -> bool {
        self.modals.building.last().copied() == self.top_modal_id()
    }
    /// Popups built outside the top modal are refused while it is open.
    pub(crate) fn modal_blocks_popup(&self) -> bool {
        self.modal_active() && !self.building_top_modal()
    }
    /// Application code outside the top modal sees idle input.
    pub(crate) fn input_blocked(&self) -> bool {
        self.modal_active() && !self.building_top_modal()
    }
    pub(crate) fn blocked_input(&self) -> &InputState {
        &self.modals.blocked_input
    }
    pub(crate) fn modal_is_open(&self, id: Id) -> bool {
        self.is_modal_layer(id)
    }
    /// Whether `modal` was opened by this pass and still waits for initial focus.
    pub(crate) fn modal_opening(&self, modal: Id) -> bool {
        self.modals
            .stack
            .iter()
            .any(|state| state.id == modal && state.pending_focus)
    }
    /// Inside a modal, floating panels fall back to their inline forms.
    pub(crate) fn building_modal(&self) -> bool {
        !self.modals.building.is_empty()
    }

    fn modal_layer_depth(&self) -> usize {
        self.modals.building.last().map_or(0, |id| {
            self.modals
                .stack
                .iter()
                .position(|modal| modal.id == *id)
                .map_or(self.modals.stack.len() + 1, |i| i + 1)
        })
    }
    /// Register a popup-class layer; layers built inside a modal sort above it.
    pub(crate) fn push_popup_layer(&mut self, id: Id) {
        self.popup_layers.push(id);
        let depth = self.modal_layer_depth();
        if depth > 0 {
            self.modals.depth.insert(id, depth);
        }
    }

    /// Start (or continue) building `id`. Returns true on the pass that opens it.
    pub(crate) fn begin_modal(&mut self, id: Id, return_focus: Option<Id>, open: bool) -> bool {
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
        self.modals.registered.insert(id);
        self.modals.building.push(id);
        let frame = self.frame;
        if !open {
            return false;
        }
        if let Some(modal) = self.modals.stack.iter_mut().find(|m| m.id == id) {
            modal.last_frame = frame;
            return false;
        }
        let return_focus = return_focus.or(match &self.popup {
            Some(popup) => popup.return_focus,
            None => self.focused_widget,
        });
        self.cancel_underlying_interactions();
        self.modals.stack.push(ModalState {
            id,
            return_focus,
            pending_focus: true,
            escape: false,
            enter: false,
            last_frame: frame,
        });
        self.request_repaint();
        true
    }
    pub(crate) fn end_modal_build(&mut self) {
        self.modals.building.pop();
    }

    /// Anything the pointer or keyboard was doing under the new modal ends now,
    /// without changing the values it controls.
    fn cancel_underlying_interactions(&mut self) {
        self.dismiss_popup(false);
        self.drag_cancel(DragReason::Cancelled);
        if let Some(capture) = self.capture.take() {
            match capture.hit.action {
                HitAction::SplitResize { .. } => {
                    let pointer = self.input.pointer.unwrap_or(capture.pointer);
                    self.split_pointer(capture.hit.id, pointer, 2);
                }
                HitAction::ColumnResize { table, .. } => {
                    if let Some(state) = self.tables.get_mut(&table) {
                        state.drag = None;
                    }
                }
                _ => {}
            }
        }
        self.gesture_cancel();
        self.stop_auto_scroll();
        self.keyboard_active = None;
        self.text_click = None;
        self.set_focus(None);
    }

    /// Take the Escape and default-action (Enter) presses delivered to `id`.
    pub(crate) fn take_modal_keys(&mut self, id: Id) -> (bool, bool) {
        self.modals
            .stack
            .iter_mut()
            .find(|modal| modal.id == id)
            .map_or((false, false), |modal| {
                (
                    std::mem::take(&mut modal.escape),
                    std::mem::take(&mut modal.enter),
                )
            })
    }

    /// Remove `id` from the stack: its hits and capture go, focus returns to the
    /// widget focused before it opened if that widget is still available.
    pub(crate) fn end_modal(&mut self, id: Id) {
        let Some(position) = self.modals.stack.iter().position(|m| m.id == id) else {
            return;
        };
        let state = self.modals.stack.remove(position);
        self.remove_popup_hits(id);
        self.keyboard_active = None;
        let inside = self.top_modal_id();
        let target = state.return_focus.filter(|focus| {
            self.previous_hits.iter().any(|hit| {
                hit.id == *focus
                    && hit.action.focusable()
                    && inside.is_none_or(|top| hit.window == top)
            })
        });
        self.set_focus(target);
        self.request_repaint();
    }

    /// Escape while a modal is open belongs to the modal and nothing below it.
    pub(super) fn modal_escape(
        &mut self,
        code: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> bool {
        if code != KeyCode::Escape || self.ime_composing {
            return false;
        }
        if let Some(modal) = self.modals.stack.last_mut() {
            if state == ElementState::Pressed && !repeat {
                modal.escape = true;
            }
            return true;
        }
        false
    }
    /// Enter outside a field that handles it activates the modal's default action.
    pub(super) fn modal_default_action(&mut self) -> bool {
        if self.ime_composing {
            return false;
        }
        self.modals.stack.last_mut().is_some_and(|modal| {
            modal.enter = true;
            true
        })
    }
    pub(super) fn modal_tab_scope(&self, hit: &HitRegion) -> bool {
        self.top_modal_id().is_none_or(|top| hit.window == top)
    }

    /// End of pass: close modals that were not built, order the layers.
    pub(super) fn finish_modals(&mut self) {
        let frame = self.frame;
        let stale: Vec<Id> = self
            .modals
            .stack
            .iter()
            .filter(|modal| modal.last_frame != frame)
            .map(|modal| modal.id)
            .collect();
        for id in stale {
            self.end_modal(id);
        }
        self.modals.measures.retain(|_, m| m.last_frame == frame);
        let windows = &mut self.windows;
        self.modals.registered.retain(|id| {
            let live = windows.get(id).is_some_and(|w| w.last_frame == frame);
            if !live {
                windows.remove(id);
            }
            live
        });
        let depth = &self.modals.depth;
        self.popup_layers
            .sort_by_key(|id| depth.get(id).copied().unwrap_or(0));
        self.modals.blocked_input.focused = self.input.focused;
    }

    /// After hits are published: give the top modal its initial focus and keep
    /// focus from escaping it.
    pub(super) fn settle_modal_focus(&mut self) {
        let Some(top) = self.modals.stack.last() else {
            return;
        };
        let (id, pending) = (top.id, top.pending_focus);
        let popup = self.popup.as_ref().map(|popup| popup.id);
        let focused = self.focused_widget.is_some_and(|focus| {
            self.previous_hits.iter().any(|hit| {
                hit.id == focus
                    && hit.action.focusable()
                    && (hit.window == id || Some(hit.window) == popup)
            })
        });
        let mut settled = focused;
        if !focused && (pending || self.focused_widget.is_some()) {
            let first = self
                .previous_hits
                .iter()
                .find(|hit| hit.window == id && hit.action.focusable())
                .map(|hit| hit.id);
            self.set_focus(first);
            self.keyboard_active = None;
            settled = first.is_some();
        }
        // Content that is not interactive yet (first measured pass) keeps waiting.
        if settled {
            if let Some(top) = self.modals.stack.last_mut() {
                top.pending_focus = false;
            }
        }
    }
}
