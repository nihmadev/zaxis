//! Hit regions, pointer capture, and widget interaction queries.

use super::{Context, Id};
use crate::{components::Sense, Rect, Vec2};
use winit::keyboard::{KeyCode, ModifiersState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HitAction {
    Block,
    ContextMenu,
    Activate,
    /// Custom widget region from `Ui::interact`; the sense picks its behavior.
    Interact(Sense),
    Focus,
    ComboBox,
    Tree,
    TreeRow {
        tree: Id,
        node: Id,
        chevron: bool,
    },
    Slider,
    DragValue,
    SplitResize {
        vertical: bool,
    },
    TextEdit,
    Move,
    Resize,
    ScrollThumb {
        area: Id,
        axis: usize,
    },
    ColumnResize {
        table: Id,
        column: Id,
    },
    /// Passive: drag sources and drop targets never take part in ordinary hit
    /// testing. `slot` indexes the registry published with the same pass.
    DragSource {
        slot: u32,
    },
    DropTarget {
        slot: u32,
    },
}

impl HitAction {
    /// Pointer inputs a region reports through `Response`.
    pub(super) fn sense(self) -> Sense {
        match self {
            Self::Activate | Self::ComboBox => Sense::CLICK | Sense::FOCUS,
            Self::Focus => Sense::FOCUS,
            Self::Slider => Sense::DRAG | Sense::FOCUS,
            Self::TextEdit | Self::DragValue => Sense::CLICK | Sense::DRAG | Sense::FOCUS,
            Self::Interact(sense) => sense,
            _ => Sense::NONE,
        }
    }
    pub(super) fn focusable(self) -> bool {
        matches!(self, Self::Interact(sense) if sense.focus())
            || matches!(
                self,
                Self::Activate
                    | Self::Focus
                    | Self::ComboBox
                    | Self::Tree
                    | Self::Slider
                    | Self::TextEdit
                    | Self::DragValue
                    | Self::SplitResize { .. }
            )
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SliderInput {
    Pointer(Vec2),
    Key(KeyCode),
}

pub(crate) enum NumberInputEvent {
    // 1: press, 0: captured motion, 2: release.
    Pointer(Vec2, u8, ModifiersState),
    Key(KeyCode, ModifiersState),
}

pub(crate) enum TextEditInput {
    Text(String),
    Commit(String),
    Key(KeyCode, ModifiersState),
    // Count is 1..=3 for presses, zero for captured dragging.
    Pointer(Vec2, bool, u8),
    Focus(bool),
    Preedit(String, Option<(usize, usize)>),
}

pub(super) struct ClickSequence {
    pub id: Id,
    pub position: Vec2,
    pub time: std::time::Instant,
    pub count: u8,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HitRegion {
    pub id: Id,
    pub window: Id,
    pub rect: Rect,
    pub clip: Rect,
    pub action: HitAction,
}

#[derive(Clone, Copy)]
pub(super) struct Capture {
    pub(super) hit: HitRegion,
    pub(super) pointer: Vec2,
    pub(super) rect: Rect,
}

impl Context {
    pub(crate) fn set_focus(&mut self, focus: Option<Id>) {
        if self.focused_widget == focus {
            return;
        }
        self.text_click = None;
        for (id, gained) in [(self.focused_widget, false), (focus, true)] {
            if let Some(id) = id {
                self.text_edit_input
                    .entry(id)
                    .or_default()
                    .push(TextEditInput::Focus(gained));
            }
        }
        self.ime_composing = false;
        self.gestures.focus_changed(self.focused_widget, focus);
        self.focused_widget = focus;
    }
    /// Move focus to a widget returned by a component on this pass.
    pub fn request_focus(&mut self, id: Id) {
        self.set_focus(Some(id));
        self.request_repaint();
    }
    pub(crate) fn register_hit(&mut self, hit: HitRegion) {
        let order = self.hit_order.len();
        let first = self
            .hit_order
            .entry(hit.id)
            .or_insert((order, Some(hit.window)));
        // The same ID may reappear in another layer (popup key targets alias their trigger).
        if first.0 != order && first.1 == Some(hit.window) {
            self.note_hit_collision(&hit);
        }
        self.route_hit(hit);
    }
    /// Deliver a hit already counted by `register_hit` to its final list.
    pub(crate) fn route_hit(&mut self, hit: HitRegion) {
        if !self.defer_placement_hit(hit) && !self.defer_scroll_hit(hit) {
            self.hits.push(hit);
        }
    }
    pub(crate) fn clicked(&self, id: Id) -> bool {
        self.clicked.contains(&id)
    }
    pub(crate) fn take_column_resize(&mut self, table: Id, column: Id) -> Option<f32> {
        self.column_resize.remove(&(table, column))
    }
    pub(crate) fn take_slider_input(&mut self, id: Id) -> Vec<SliderInput> {
        let transform = self
            .input_transforms
            .get(&id)
            .copied()
            .unwrap_or_default()
            .inverse();
        self.slider_input
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|input| match input {
                SliderInput::Pointer(p) => SliderInput::Pointer(transform.point(p)),
                input => input,
            })
            .collect()
    }
    /// Pointer position in the field's own (untransformed) coordinates, e.g. for drag auto-scroll.
    pub(crate) fn text_edit_pointer(&self, id: Id) -> Option<Vec2> {
        let transform = self
            .input_transforms
            .get(&id)
            .copied()
            .unwrap_or_default()
            .inverse();
        self.input.pointer.map(|p| transform.point(p))
    }
    pub(crate) fn take_text_edit_input(&mut self, id: Id) -> Vec<TextEditInput> {
        let transform = self
            .input_transforms
            .get(&id)
            .copied()
            .unwrap_or_default()
            .inverse();
        self.text_edit_input
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|input| match input {
                TextEditInput::Pointer(p, extend, count) => {
                    TextEditInput::Pointer(transform.point(p), extend, count)
                }
                input => input,
            })
            .collect()
    }
    pub(crate) fn take_number_input(&mut self, id: Id) -> Vec<NumberInputEvent> {
        let transform = self
            .input_transforms
            .get(&id)
            .copied()
            .unwrap_or_default()
            .inverse();
        self.number_input
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|event| match event {
                NumberInputEvent::Pointer(p, phase, mods) => {
                    NumberInputEvent::Pointer(transform.point(p), phase, mods)
                }
                event => event,
            })
            .collect()
    }
    pub(crate) fn active(&self, id: Id) -> bool {
        self.capture.is_some_and(|c| c.hit.id == id)
            || self.keyboard_active.is_some_and(|(active, _)| active == id)
    }
    pub(crate) fn has_focus(&self, id: Id) -> bool {
        self.focused_widget == Some(id)
    }
    pub(crate) fn focus_visible(&self, id: Id) -> bool {
        self.focus_visible && self.has_focus(id)
    }

    pub(crate) fn hovered(&self, id: Id, window: Id, rect: Rect, clip: Rect) -> bool {
        let (rect, clip) = if self.visual_depth > 0 {
            self.previous_hits
                .iter()
                .find(|hit| hit.id == id)
                .map_or((rect, clip), |hit| (hit.rect, hit.clip))
        } else {
            (rect, clip)
        };
        self.input.pointer.is_some_and(|p| {
            rect.contains(p)
                && self.scroll_clip(window, clip).contains(p)
                && self
                    .visual_clips
                    .iter()
                    .filter(|(owner, _)| *owner == window)
                    .all(|(_, clip)| clip.contains(p))
                && self.top_window(p).is_none_or(|top| top == window)
                && self.capture.is_none_or(|capture| capture.hit.id == id)
                && self
                    .drag
                    .session
                    .as_ref()
                    .is_none_or(|session| session.info.id == id)
        })
    }

    pub(super) fn hit_test(&self, pointer: Vec2) -> Option<HitRegion> {
        let window = self.top_window(pointer)?;
        // A popup's trigger proxy sits in the popup layer under its full-viewport blocker;
        // deferred routing can order it first, so the anchor must win explicitly or a
        // second press on the trigger could never reach it.
        if let Some(target) = self
            .popup
            .as_ref()
            .filter(|popup| popup.id == window && popup.anchor.contains(pointer))
            .and_then(|popup| popup.key_target)
        {
            let proxy = self.previous_hits.iter().rev().copied().find(|hit| {
                hit.id == target
                    && hit.window == window
                    && hit.action == HitAction::ComboBox
                    && hit.rect.contains(pointer)
                    && hit.clip.contains(pointer)
            });
            if proxy.is_some() {
                return proxy;
            }
        }
        self.previous_hits.iter().rev().copied().find(|hit| {
            !matches!(
                hit.action,
                HitAction::ContextMenu
                    | HitAction::DragSource { .. }
                    | HitAction::DropTarget { .. }
            ) && hit.window == window
                && hit.rect.contains(pointer)
                && hit.clip.contains(pointer)
        })
    }
}
