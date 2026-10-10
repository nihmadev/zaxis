//! The open branch of popups: a root, then a chain of children, each opened from inside
//! the one before. Order and parents come from the UI scope a popup is built in, not from
//! overlap. Closing a popup closes everything opened from it; closing the leaf keeps its
//! ancestors.

use super::{Dismissal, PopupState, Popups, MAX_POPUP_DEPTH};
use crate::{
    context::{Context, Id},
    Rect, Vec2,
};

/// What registering a popup found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Registered {
    /// Opened by this pass.
    Opening,
    /// Open since an earlier pass.
    Kept,
    /// Refused (too deep, or built inside itself); a diagnostic was reported.
    Rejected,
}

/// What a popup tells the branch each pass it is open.
pub(crate) struct Opening {
    pub id: Id,
    /// The layer the popup is built in.
    pub window: Id,
    pub anchor: Rect,
    pub rect: Rect,
    pub return_focus: Option<Id>,
    pub key_target: Option<Id>,
}

impl Popups {
    /// The leaf: the popup that owns keys and the pointer.
    pub(crate) fn top(&self) -> Option<&PopupState> {
        self.branch.last()
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.branch.is_empty()
    }

    pub(crate) fn index_of(&self, id: Id) -> Option<usize> {
        self.branch.iter().position(|popup| popup.id == id)
    }

    /// The popup that owns `layer`: its own layer or one of its extra panels.
    pub(crate) fn index_of_layer(&self, layer: Id) -> Option<usize> {
        self.branch
            .iter()
            .position(|p| p.id == layer || p.extra.iter().any(|(extra, _)| *extra == layer))
    }

    /// Whether `layer` belongs to the leaf, which alone takes keys.
    pub(crate) fn is_top_layer(&self, layer: Id) -> bool {
        self.top()
            .is_some_and(|p| p.id == layer || p.extra.iter().any(|(extra, _)| *extra == layer))
    }

    /// The deepest level a press at `point` lands in: a panel, an extra panel or the
    /// trigger of a popup that has one. `None` is outside the whole branch.
    pub(crate) fn level_at(&self, point: Vec2) -> Option<usize> {
        self.branch.iter().rposition(|popup| {
            popup.rect.contains(point)
                || popup.extra.iter().any(|(_, rect)| rect.contains(point))
                || (popup.key_target.is_some() && popup.anchor.contains(point))
        })
    }
}

impl Context {
    /// Close the leaf popup now, removing its hits and restoring its trigger focus. Its
    /// ancestors stay open. Use [`close_popup_branch`](Self::close_popup_branch) to close
    /// them too.
    pub fn close_popup(&mut self) {
        if let Some(last) = self.popups.branch.len().checked_sub(1) {
            self.dismiss_popups_from(last, true);
        }
    }

    /// Close the whole branch, root and every popup opened from it, restoring the focus
    /// the root's trigger had once.
    pub fn close_popup_branch(&mut self) {
        self.dismiss_popups_from(0, true);
    }

    /// Close the whole branch; focus returns to the root's trigger when `restore_focus`.
    pub(crate) fn dismiss_popup(&mut self, restore_focus: bool) {
        self.dismiss_popups_from(0, restore_focus);
    }

    /// Close popup `id` and everything opened from it, because its own builder was told
    /// to (its `open` flag went false). The builders of the closed popups need no
    /// dismissal to hear of it: they are not told, so they cannot be told twice.
    pub(crate) fn close_popup_of_builder(&mut self, id: Id) {
        let Some(index) = self.popups.index_of(id) else {
            return;
        };
        for closed in self.dismiss_popups_from(index, true) {
            self.popups.dismissed.remove(&closed);
        }
    }

    /// Close the levels from `index` to the leaf. Keys held and queued for them are
    /// dropped, their hits and capture go, and focus returns once to the outermost
    /// closed level's trigger (or the nearest place that can still take it).
    pub(crate) fn dismiss_popups_from(&mut self, index: usize, restore_focus: bool) -> Vec<Id> {
        if index >= self.popups.branch.len() {
            return Vec::new();
        }
        let closed: Vec<PopupState> = self.popups.branch.drain(index..).collect();
        // Keys held for the widgets below were taken under a different layer.
        self.keys.reset();
        for popup in closed.iter().rev() {
            if let Some(target) = popup.key_target {
                self.take_menu_keys(target);
            }
            let dismissal = Dismissal {
                parent: popup.parent,
                frame: self.frame,
            };
            self.popups.dismissed.insert(popup.id, dismissal);
            self.remove_popup_hits(popup.id);
            for (layer, _) in &popup.extra {
                self.remove_popup_hits(*layer);
            }
        }
        if restore_focus {
            let target = self.restore_target(&closed[0]);
            self.set_focus(target);
        }
        self.interaction.keyboard_active = None;
        self.stop_auto_scroll();
        self.request_repaint();
        closed.iter().map(|popup| popup.id).collect()
    }

    /// Where focus goes when `closed` (the outermost level that closed) does: its
    /// trigger, else the trigger of the popup it was opened from, else what it recorded
    /// (focus then settles by the usual rules).
    fn restore_target(&self, closed: &PopupState) -> Option<Id> {
        let parent = closed
            .parent
            .and_then(|id| self.popups.index_of(id))
            .map(|i| &self.popups.branch[i]);
        [closed.return_focus, parent.and_then(|p| p.key_target)]
            .into_iter()
            .flatten()
            .find(|id| self.can_take_focus(*id))
            .or(closed.return_focus)
    }

    fn can_take_focus(&self, id: Id) -> bool {
        self.interaction.previous_hits.iter().any(|hit| {
            hit.id == id && hit.action.focusable() && !hit.rect.intersect(hit.clip).is_empty()
        })
    }

    /// Whether popup `id` is open anywhere in the branch.
    pub(crate) fn popup_open(&self, id: Id) -> bool {
        self.popups.index_of(id).is_some()
    }

    /// Register popup `opening.id` as open in this pass, below the popup its `window`
    /// belongs to (a root otherwise). Opening a popup closes what was open at its level
    /// and below: another root replaces the whole branch, a sibling replaces its sibling.
    pub(crate) fn register_popup(&mut self, opening: Opening) -> Registered {
        let parent = self.popups.index_of_layer(opening.window);
        let existing = self.popups.index_of(opening.id);
        if let (Some(parent), Some(existing)) = (parent, existing) {
            if parent >= existing {
                self.report_nesting(&opening, "popup built inside itself or a popup it opened");
                return Registered::Rejected;
            }
        }
        // A popup that moved under another parent opens afresh.
        let expected = parent.map(|i| self.popups.branch[i].id);
        let existing = match existing {
            Some(i) if self.popups.branch[i].parent != expected => {
                self.dismiss_popups_from(i, true);
                None
            }
            other => other,
        };
        let frame = self.frame;
        if let Some(i) = existing {
            let popup = &mut self.popups.branch[i];
            popup.anchor = opening.anchor;
            popup.rect = opening.rect;
            popup.key_target = opening.key_target;
            popup.last_frame = frame;
            return Registered::Kept;
        }
        let depth = parent.map_or(0, |i| i + 1);
        if depth >= MAX_POPUP_DEPTH {
            self.report_nesting(&opening, "popups nested too deep; the popup is not opened");
            return Registered::Rejected;
        }
        let owner = parent.map_or(opening.window, |i| self.popups.branch[i].owner);
        self.dismiss_popups_from(depth, true);
        self.popups.branch.push(PopupState {
            id: opening.id,
            parent: expected,
            owner,
            anchor: opening.anchor,
            rect: opening.rect,
            return_focus: opening.return_focus,
            key_target: opening.key_target,
            last_frame: frame,
            extra: Vec::new(),
        });
        Registered::Opening
    }

    fn report_nesting(&mut self, opening: &Opening, message: &'static str) {
        self.report(
            crate::DiagnosticKind::InvalidUsage,
            Some(opening.id),
            Some(opening.anchor),
            || message.into(),
        );
    }

    /// Close what was not built this pass: from the first level that was not built, or
    /// whose owner window was not, down. A leaf that vanished leaves its ancestors open.
    pub(crate) fn retire_popups(&mut self) {
        let (frame, windows) = (self.frame, &self.windows);
        let built = self
            .popups
            .branch
            .iter()
            .take_while(|popup| {
                popup.last_frame == frame
                    && windows
                        .get(&popup.owner)
                        .is_some_and(|w| w.last_frame == frame)
            })
            .count();
        self.dismiss_popups_from(built, true);
    }
}
