//! Hit regions, pointer capture, and widget interaction queries.

use super::{Context, Id};
use crate::{components::Sense, Rect, Vec2};
use std::collections::{HashMap, HashSet};
use winit::keyboard::KeyCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitAction {
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
    /// Selectable static text: pointer press and drag select, I-beam cursor.
    StaticText,
    /// A hyperlink fragment: click activates, hand cursor. Fragments of one link share an ID.
    Link,
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
    /// Passive: carries the bounds of an accessibility node through placement, scrolling
    /// and visual transforms. Removed from the hit list before it routes any input.
    Semantic,
}

impl HitAction {
    /// Pointer inputs a region reports through `Response`.
    pub(crate) fn sense(self) -> Sense {
        match self {
            Self::Activate | Self::ComboBox => Sense::CLICK | Sense::FOCUS,
            Self::Focus => Sense::FOCUS,
            Self::Slider => Sense::DRAG | Sense::FOCUS,
            Self::TextEdit | Self::DragValue | Self::StaticText => {
                Sense::CLICK | Sense::DRAG | Sense::FOCUS
            }
            // A link has no drag of its own: moving between the lines of one wrapped link
            // between press and release is still a click.
            Self::Link => Sense::CLICK | Sense::FOCUS,
            Self::Interact(sense) => sense,
            _ => Sense::NONE,
        }
    }
    pub fn focusable(self) -> bool {
        matches!(self, Self::Interact(sense) if sense.focus())
            || matches!(
                self,
                Self::Activate
                    | Self::Focus
                    | Self::ComboBox
                    | Self::Tree
                    | Self::Slider
                    | Self::TextEdit
                    | Self::StaticText
                    | Self::Link
                    | Self::DragValue
                    | Self::SplitResize { .. }
            )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HitRegion {
    pub id: Id,
    pub window: Id,
    pub rect: Rect,
    pub clip: Rect,
    pub action: HitAction,
}

impl HitRegion {
    pub(crate) fn translate(&mut self, delta: Vec2) {
        self.rect = self.rect.translate(delta);
        self.clip = self.clip.translate(delta);
    }
}

#[derive(Clone, Copy)]
pub struct Capture {
    pub hit: HitRegion,
    pub pointer: Vec2,
    pub rect: Rect,
}

/// Who receives input: hit regions of this pass and of the last one, the pointer
/// capture, focus and the activations delivered to the pass.
#[derive(Default)]
pub(crate) struct Interaction {
    /// Regions registered by this pass; sorted by layer at its end.
    pub(super) hits: Vec<HitRegion>,
    /// First registration of each hit ID: its order and, unless merely reserved, its window.
    order: HashMap<Id, (usize, Option<Id>)>,
    /// Regions published by the last pass: they route input until this one ends.
    pub(crate) previous_hits: Vec<HitRegion>,
    pub(super) capture: Option<Capture>,
    /// Activations delivered to this pass: pointer release, keyboard, assistive technology.
    pub(super) clicked: HashSet<Id>,
    pub(super) focused: Option<Id>,
    /// Focus came from the keyboard: the focused widget shows its ring.
    pub(super) focus_visible: bool,
    /// The widget held down by Enter or Space; releasing that key clicks it.
    pub(super) keyboard_active: Option<(Id, KeyCode)>,
}

impl Interaction {
    pub(super) fn begin_pass(&mut self) {
        self.hits.clear();
        self.order.clear();
    }

    /// Count `hit` in registration order. Returns whether its ID was already registered
    /// in the same window this pass; another layer may reuse it (popup key targets alias
    /// their trigger).
    fn note(&mut self, hit: &HitRegion) -> bool {
        let order = self.order.len();
        let first = self
            .order
            .entry(hit.id)
            .or_insert((order, Some(hit.window)));
        first.0 != order && first.1 == Some(hit.window)
    }

    /// Give `id` a place in registration order without a region of its own.
    pub(super) fn reserve(&mut self, id: Id) {
        let order = self.order.len();
        self.order.entry(id).or_insert((order, None));
    }

    /// Order hits by layer rank, then registration.
    pub(super) fn sort(&mut self, ranks: &HashMap<Id, usize>) {
        let order = &self.order;
        self.hits.sort_by_key(|hit| {
            (
                ranks.get(&hit.window).copied().unwrap_or(0),
                order.get(&hit.id).map_or(0, |slot| slot.0),
            )
        });
    }

    /// Keep this pass's regions that `keep` accepts; the others leave before routing.
    pub(crate) fn retain_hits(&mut self, keep: impl FnMut(&HitRegion) -> bool) {
        self.hits.retain(keep);
    }

    /// Regions clipped away entirely never route input.
    pub(super) fn drop_unreachable(&mut self) {
        self.hits.retain(|h| !h.rect.intersect(h.clip).is_empty());
    }

    /// This pass's regions route input from now on.
    pub(super) fn publish(&mut self) {
        self.previous_hits = std::mem::take(&mut self.hits);
    }

    /// The action of the first published region of `id`.
    pub(crate) fn action_of(&self, id: Id) -> Option<HitAction> {
        self.previous_hits
            .iter()
            .find(|hit| hit.id == id)
            .map(|hit| hit.action)
    }

    /// Whether `id` published a region with `action`.
    pub(crate) fn published(&self, id: Id, action: HitAction) -> bool {
        self.previous_hits
            .iter()
            .any(|hit| hit.id == id && hit.action == action)
    }

    /// The focused widget, when it published a region with `action`.
    pub(super) fn focused_as(&self, action: HitAction) -> Option<Id> {
        self.focused.filter(|id| self.published(*id, action))
    }

    /// Whether `id` published a region that takes focus.
    pub(super) fn focusable(&self, id: Id) -> bool {
        self.previous_hits
            .iter()
            .any(|hit| hit.id == id && hit.action.focusable())
    }

    /// Drop every region of `layer`, and the capture one of them holds.
    pub(super) fn remove_layer(&mut self, layer: Id) {
        self.hits.retain(|hit| hit.window != layer);
        self.previous_hits.retain(|hit| hit.window != layer);
        if self
            .capture
            .is_some_and(|capture| capture.hit.window == layer)
        {
            self.capture = None;
        }
    }
}

impl Context {
    /// End of pass, against the regions it published: the top modal takes or keeps focus;
    /// a focused widget without a focusable region loses focus; a capture whose region is
    /// gone ends, unless a selection drag or column resize still holds it.
    pub(super) fn settle_focus(&mut self) {
        self.settle_modal_focus();
        if self
            .interaction
            .focused
            .is_some_and(|id| !self.interaction.focusable(id))
        {
            self.set_focus(None);
            self.interaction.keyboard_active = None;
        }
        if self
            .interaction
            .capture
            .is_some_and(|capture| !self.capture_holds(capture))
        {
            self.interaction.capture = None;
            self.gesture_cancel();
        }
    }

    fn capture_holds(&self, capture: Capture) -> bool {
        let hit = capture.hit;
        self.interaction.published(hit.id, hit.action)
            // A selection keeps following the pointer after scrolling carried the text
            // it began in out of sight.
            || self.selection_holds(hit.id)
            || matches!(hit.action, HitAction::ColumnResize { table, column }
                if self.column_resize_holds(table, column, hit.window))
    }

    pub(crate) fn set_focus(&mut self, focus: Option<Id>) {
        if self.interaction.focused == focus {
            return;
        }
        self.text_fields
            .focus_changed(self.interaction.focused, focus);
        self.gestures.focus_changed(self.interaction.focused, focus);
        self.interaction.focused = focus;
    }
    /// Move focus to a widget returned by a component on this pass.
    pub fn request_focus(&mut self, id: Id) {
        self.set_focus(Some(id));
        self.request_repaint();
    }
    /// The focused widget.
    pub(crate) fn focused(&self) -> Option<Id> {
        self.interaction.focused
    }
    pub(crate) fn register_hit(&mut self, hit: HitRegion) {
        if self.interaction.note(&hit) {
            self.note_hit_collision(&hit);
        }
        self.route_hit(hit);
    }
    /// Deliver a hit already counted by `register_hit` to its final list.
    pub(crate) fn route_hit(&mut self, hit: HitRegion) {
        if !self.defer_placement_hit(hit) && !self.defer_scroll_hit(hit) {
            self.interaction.hits.push(hit);
        }
    }
    pub(crate) fn clicked(&self, id: Id) -> bool {
        self.interaction.clicked.contains(&id)
    }
    /// Report a click on `id` to the next pass, as a pointer release inside it would.
    #[cfg_attr(not(feature = "accesskit"), allow(dead_code))]
    pub(crate) fn synthesize_click(&mut self, id: Id) {
        self.interaction.clicked.insert(id);
    }
    /// Show the focus ring, as after keyboard navigation.
    #[cfg_attr(not(feature = "accesskit"), allow(dead_code))]
    pub(crate) fn show_focus(&mut self) {
        self.interaction.focus_visible = true;
    }
    pub(crate) fn active(&self, id: Id) -> bool {
        self.interaction.capture.is_some_and(|c| c.hit.id == id)
            || self
                .interaction
                .keyboard_active
                .is_some_and(|(active, _)| active == id)
    }
    pub(crate) fn has_focus(&self, id: Id) -> bool {
        self.interaction.focused == Some(id)
    }
    pub(crate) fn focus_visible(&self, id: Id) -> bool {
        self.interaction.focus_visible && self.has_focus(id)
    }

    pub(crate) fn hovered(&self, id: Id, window: Id, rect: Rect, clip: Rect) -> bool {
        let (rect, clip) = if self.visuals.depth > 0 {
            self.interaction
                .previous_hits
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
                    .visuals
                    .clips
                    .iter()
                    .filter(|(owner, _)| *owner == window)
                    .all(|(_, clip)| clip.contains(p))
                && self.top_window(p).is_none_or(|top| top == window)
                && self
                    .interaction
                    .capture
                    .is_none_or(|capture| capture.hit.id == id)
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
            .popups
            .current
            .as_ref()
            .filter(|popup| popup.id == window && popup.anchor.contains(pointer))
            .and_then(|popup| popup.key_target)
        {
            let proxy = self
                .interaction
                .previous_hits
                .iter()
                .rev()
                .copied()
                .find(|hit| {
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
        self.interaction
            .previous_hits
            .iter()
            .rev()
            .copied()
            .find(|hit| {
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
