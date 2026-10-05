//! Selection of static text: one range per context, spanning one or several labels.
//!
//! A selection is an anchor and a head, each a byte offset inside a text item named by its
//! stable [`Id`]. Items that belong to the same scope (a `selection_scope`, or the label
//! alone) are ordered the way they were built during the last pass, so a range that starts
//! in one label and ends in another covers everything between them. State lives here, not
//! in the widgets: the keyboard shortcuts act on whichever item holds focus without the
//! widgets having to run first, items that vanish take their state with them, and
//! retained memory is bounded by what is on screen.
//!
//! Only items that were built (drawn) take part in a copy: rows a virtualized list never
//! built have no text to read.

use super::{Context, Id};
use crate::components::text_block::BlockState;
use crate::time::Instant;
use crate::Vec2;
use std::{collections::HashMap, ops::Range};
use winit::keyboard::{KeyCode, ModifiersState};

mod drag;

/// One end of a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Endpoint {
    pub item: Id,
    pub byte: usize,
    /// Position of the item in its scope when it was last seen; orders endpoints.
    pub ord: usize,
}

impl Endpoint {
    fn key(&self) -> (usize, usize) {
        (self.ord, self.byte)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Selection {
    pub scope: Id,
    pub anchor: Endpoint,
    pub head: Endpoint,
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        self.anchor.key() == self.head.key() && self.anchor.item == self.head.item
    }
    /// The endpoints in document order.
    pub fn ordered(&self) -> (Endpoint, Endpoint) {
        if self.anchor.key() <= self.head.key() {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }
}

/// The items of one scope in build order.
#[derive(Default)]
pub(crate) struct Scope {
    /// Complete order of the last finished pass, then of the pass being built.
    pub order: Vec<Id>,
    /// Order of the previous pass while this one is being built.
    pub previous: Vec<Id>,
    pub frame: u64,
    pub separator: String,
    pub copy_urls: bool,
}

/// Counts presses of one item for double and triple clicks.
struct Presses {
    item: Id,
    position: crate::Vec2,
    time: Instant,
    count: u8,
}

#[derive(Default)]
pub(crate) struct SelectionState {
    pub items: HashMap<Id, BlockState>,
    pub scopes: HashMap<Id, Scope>,
    /// Focus targets (an item, or a link inside it) to the item that owns them and its scope.
    pub owners: HashMap<Id, (Id, Id)>,
    pub selection: Option<Selection>,
    pub stack: Vec<Id>,
    pub drag: Option<drag::Drag>,
    presses: Option<Presses>,
    /// A press on selectable text or a link since the last pass: the target and the point.
    pressed: Option<(Id, Vec2, ModifiersState)>,
    /// The button went up on this target since the last pass; its drag still counts once.
    released: Option<Id>,
}

impl Context {
    /// The text covered by the current selection of static text, if any.
    pub fn selected_text(&self) -> Option<String> {
        let selection = self.selection.selection?;
        (!selection.is_empty()).then(|| self.selection_text(&selection))?
    }

    /// Clear the selection of static text.
    pub fn clear_selection(&mut self) {
        if self.selection.selection.take().is_some() {
            self.selection.drag = None;
            self.request_repaint();
        }
    }

    pub(crate) fn selection_scope_id(&self, item: Id) -> Id {
        self.selection.stack.last().copied().unwrap_or(item)
    }

    /// Register `item` in `scope` for this pass; returns its position there.
    pub(crate) fn selection_enter(&mut self, scope: Id, item: Id) -> usize {
        let frame = self.frame;
        let scope = self.selection.scopes.entry(scope).or_default();
        if scope.frame != frame {
            scope.frame = frame;
            scope.previous = std::mem::take(&mut scope.order);
        }
        scope.order.push(item);
        scope.order.len() - 1
    }

    /// The part of `item` (`len` bytes, at `ord` in `scope`) the selection covers.
    pub(crate) fn selection_range(
        &self,
        scope: Id,
        item: Id,
        ord: usize,
        len: usize,
    ) -> Range<usize> {
        let Some(selection) = self.selection.selection.filter(|s| s.scope == scope) else {
            return 0..0;
        };
        let (lo, hi) = selection.ordered();
        let start = if item == lo.item {
            lo.byte.min(len)
        } else if ord > lo.ord {
            0
        } else {
            return 0..0;
        };
        let end = if item == hi.item {
            hi.byte.min(len)
        } else if ord < hi.ord {
            len
        } else {
            return 0..0;
        };
        start.min(end)..end
    }

    /// Replace the selection with an empty one at `endpoint`.
    pub(crate) fn selection_begin(&mut self, scope: Id, endpoint: Endpoint) {
        self.selection.selection = Some(Selection {
            scope,
            anchor: endpoint,
            head: endpoint,
        });
        self.request_repaint();
    }

    /// Select `range` of one item: anchor at its start, head at its end.
    pub(crate) fn selection_set(&mut self, scope: Id, item: Id, ord: usize, range: Range<usize>) {
        let at = |byte| Endpoint { item, byte, ord };
        self.selection.selection = Some(Selection {
            scope,
            anchor: at(range.start),
            head: at(range.end),
        });
        self.request_repaint();
    }

    /// Move the head of the selection of `scope`; without one, a new empty selection starts.
    pub(crate) fn selection_extend(&mut self, scope: Id, head: Endpoint) {
        match self
            .selection
            .selection
            .as_mut()
            .filter(|s| s.scope == scope)
        {
            Some(selection) if selection.head != head => {
                selection.head = head;
                self.request_repaint();
            }
            Some(_) => {}
            None => self.selection_begin(scope, head),
        }
    }

    /// Refresh the order hint of the selection's endpoints that are `item`.
    pub(crate) fn selection_touch(&mut self, item: Id, ord: usize) {
        if let Some(selection) = self.selection.selection.as_mut() {
            for end in [&mut selection.anchor, &mut selection.head] {
                if end.item == item {
                    end.ord = ord;
                }
            }
        }
    }

    /// Whether a press of `item` at the pointer continues a multi-click: 1, 2 or 3.
    pub(crate) fn selection_press(&mut self, item: Id, pointer: Vec2, extend: bool) -> u8 {
        let now = Instant::now();
        let count = self.selection.presses.as_ref().map_or(1, |last| {
            let near = (pointer - last.position).length_squared() <= 16.0;
            let fast =
                now.saturating_duration_since(last.time) <= std::time::Duration::from_millis(500);
            if last.item == item && near && fast && !extend {
                last.count % 3 + 1
            } else {
                1
            }
        });
        self.selection.presses = Some(Presses {
            item,
            position: pointer,
            time: now,
            count,
        });
        count
    }

    /// Select everything in the scope of the focused item. Returns whether there was one.
    pub(crate) fn selection_select_all(&mut self, scope: Id) -> bool {
        let Some(scope_info) = self.selection.scopes.get(&scope) else {
            return false;
        };
        let items: Vec<(usize, Id, usize)> = scope_info
            .order
            .iter()
            .enumerate()
            .filter_map(|(i, id)| Some((i, *id, self.selection.items.get(id)?.text.len())))
            .collect();
        let (Some(first), Some(last)) = (items.first(), items.last()) else {
            return false;
        };
        self.selection.selection = Some(Selection {
            scope,
            anchor: Endpoint {
                item: first.1,
                byte: 0,
                ord: first.0,
            },
            head: Endpoint {
                item: last.1,
                byte: last.2,
                ord: last.0,
            },
        });
        self.request_repaint();
        true
    }

    fn selection_text(&self, selection: &Selection) -> Option<String> {
        let scope = self.selection.scopes.get(&selection.scope)?;
        let (lo, hi) = selection.ordered();
        let first = scope
            .order
            .iter()
            .position(|id| *id == lo.item)
            .unwrap_or(lo.ord);
        let last = scope
            .order
            .iter()
            .position(|id| *id == hi.item)
            .unwrap_or(hi.ord);
        let mut out = String::new();
        let mut any = false;
        for id in scope.order.iter().take(last + 1).skip(first) {
            let Some(item) = self.selection.items.get(id) else {
                continue;
            };
            let start = if *id == lo.item { lo.byte } else { 0 };
            let end = if *id == hi.item {
                hi.byte
            } else {
                item.text.len()
            };
            if any {
                out.push_str(&scope.separator);
            }
            any = true;
            out.push_str(&item.copy_range(start..end.max(start), scope.copy_urls));
        }
        Some(out)
    }

    /// Put the selection on the clipboard. Does nothing, and returns false, when empty.
    pub(crate) fn selection_copy(&mut self) -> bool {
        let Some(text) = self.selected_text().filter(|text| !text.is_empty()) else {
            return false;
        };
        self.copy_text(text).is_ok()
    }

    /// Ctrl/Cmd+A, Ctrl/Cmd+C and Ctrl+Insert while a text item holds focus.
    pub(super) fn selection_key(&mut self, code: KeyCode) -> bool {
        let mods = self.input.modifiers;
        let command = mods.control_key() || mods.super_key();
        let Some((_, scope)) = self
            .focused_widget
            .and_then(|id| self.selection.owners.get(&id).copied())
        else {
            return false;
        };
        match code {
            KeyCode::KeyA if command => self.selection_select_all(scope),
            KeyCode::KeyC | KeyCode::Insert if command && !mods.shift_key() => {
                self.selection_copy()
            }
            _ => false,
        }
    }

    /// Drop state of items and scopes the finished pass did not build, and a selection
    /// whose scope is gone.
    pub(super) fn finish_selection(&mut self) {
        let frame = self.frame;
        let state = &mut self.selection;
        state.items.retain(|_, item| item.last_frame == frame);
        state.scopes.retain(|_, scope| scope.frame == frame);
        let items = &state.items;
        state
            .owners
            .retain(|_, (owner, _)| items.contains_key(owner));
        if state
            .selection
            .is_some_and(|s| !state.scopes.contains_key(&s.scope))
        {
            state.selection = None;
            state.drag = None;
        }
        state.pressed = None;
        state.released = None;
        if state.drag.is_some() && self.capture.is_none() {
            state.drag = None;
        }
    }

    /// Whether the topmost region under the pointer is the link `id` and no other widget
    /// holds the pointer.
    pub(crate) fn link_hovered(&self, id: Id) -> bool {
        self.capture.is_none_or(|capture| capture.hit.id == id)
            && self
                .input
                .pointer
                .and_then(|pointer| self.hit_test(pointer))
                .is_some_and(|hit| hit.id == id)
    }

    /// Where the primary button went down on `id` during the last input, in the widget's own
    /// coordinates; only for the one pass that follows the press.
    pub(crate) fn press_point(&self, id: Id) -> Option<(Vec2, ModifiersState)> {
        let (target, point, mods) = self
            .selection
            .pressed
            .filter(|(target, ..)| *target == id)?;
        let transform = self
            .input_transforms
            .get(&target)
            .copied()
            .unwrap_or_default();
        Some((transform.inverse().point(point), mods))
    }

    /// The pointer went down on selectable text or a link. Recorded when the event arrives,
    /// so a press, drag and release that all precede the next pass are still seen whole.
    pub(super) fn selection_note_press(&mut self, id: Id, point: Vec2) {
        self.selection.pressed = Some((id, point, self.input.modifiers));
    }

    /// The primary button went up while `id` held the pointer.
    pub(super) fn selection_note_release(&mut self, id: Id) {
        if self.selection.drag.as_ref().is_some_and(|d| d.owner == id)
            || self
                .selection
                .pressed
                .is_some_and(|(target, ..)| target == id)
        {
            self.selection.released = Some(id);
        }
    }

    /// Whether the selection of `scope` should look active: the window has focus and one of
    /// the scope's items (or links) has keyboard focus or is being dragged.
    pub(crate) fn selection_active(&self, scope: Id) -> bool {
        self.input.focused
            && (self.selection_dragging(scope)
                || self
                    .focused_widget
                    .and_then(|id| self.selection.owners.get(&id))
                    .is_some_and(|(_, s)| *s == scope))
    }

    /// Whether a selection drag that began on `id` is in progress.
    pub(super) fn selection_holds(&self, id: Id) -> bool {
        self.selection.drag.as_ref().is_some_and(|d| d.owner == id)
    }

    /// Whether a selection drag that started on `id` moved far enough to be a selection
    /// rather than a click.
    pub(crate) fn selection_dragged_from(&self, id: Id) -> bool {
        let Some(drag) = self.selection.drag.as_ref().filter(|d| d.owner == id) else {
            return false;
        };
        self.input
            .pointer
            .is_some_and(|p| (p - drag.origin).length_squared() > 16.0)
    }

    /// Physical pixels per logical pixel.
    pub(crate) fn pixel_scale(&self) -> f32 {
        self.scale
    }

    /// A press that hit nothing selectable clears the selection.
    pub(super) fn selection_press_elsewhere(&mut self, hit: Option<super::HitRegion>) {
        let owned = hit.is_some_and(|hit| self.selection.owners.contains_key(&hit.id));
        if !owned && self.popup.is_none() {
            self.clear_selection();
        }
    }
}
