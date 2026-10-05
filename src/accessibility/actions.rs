//! What the host exchanges with assistive technology: tree updates out, requests in.
//!
//! A request becomes ordinary input for the next pass. A click sets the same `clicked`
//! state a pointer click sets; focus and scrolling go through the paths the keyboard and
//! the wheel use; everything else waits in a queue for the widget it targets, which
//! applies it with its own rules (clamping, steps, undo history). A request for a node
//! that is gone, hidden or disabled does nothing.

use super::{
    node::{DISABLED, HIDDEN},
    tree::AccessStats,
    AccessAction, AccessActionKind as Kind, NO_PARENT,
};
use crate::{Context, EventResponse, Vec2};
use accesskit::{Action, ActionData, ActionRequest, ScrollUnit, TextPosition, TreeUpdate};

/// Logical pixels of one `ScrollUnit::Item` step, about three lines of text.
const ITEM: f32 = 48.0;

const IGNORED: EventResponse = EventResponse {
    consumed: false,
    repaint: false,
};

impl Context {
    /// The changes of the last pass for the platform adapter, or `None` when the tree did
    /// not change. Pass it to `accesskit_winit::Adapter::update_if_active` (or another
    /// AccessKit adapter). The first update after [`Context::set_accessibility_active`]
    /// carries the whole tree. An update that is not taken is not lost: the next one is
    /// then complete again.
    ///
    /// AccessKit types appear in this signature and in
    /// [`Context::on_accessibility_action`] because they are what an adapter consumes;
    /// everything a widget or an application touches uses zaxis' own types.
    pub fn take_accessibility_update(&mut self) -> Option<TreeUpdate> {
        self.a11y.tree.update.take()
    }

    /// Whether the last pass left an update to take.
    pub fn accessibility_update_pending(&self) -> bool {
        self.a11y.tree.update.is_some()
    }

    /// Counters of the accessibility tree: passes, updates and nodes sent.
    #[doc(hidden)]
    pub fn accessibility_stats(&self) -> AccessStats {
        self.a11y.tree.stats
    }

    /// Deliver a request from assistive technology. It takes effect on the next pass, like
    /// input: the response asks for a redraw when the request was accepted. Unknown,
    /// hidden and disabled targets and actions a node does not accept are ignored.
    pub fn on_accessibility_action(&mut self, request: &ActionRequest) -> EventResponse {
        if !self.a11y.wanted {
            return IGNORED;
        }
        let tree = &self.a11y.tree;
        let target = request.target_node.0;
        let Some(index) = tree
            .index
            .get(&target)
            .or_else(|| tree.runs.get(&target).map(|(node, _)| node))
            .map(|index| *index as usize)
        else {
            return IGNORED;
        };
        let mut ancestor = index;
        loop {
            let entry = &tree.entries[ancestor];
            if !entry.alive || tree.nodes[ancestor].has(HIDDEN) {
                return IGNORED;
            }
            if entry.parent == NO_PARENT {
                break;
            }
            ancestor = entry.parent as usize;
        }
        let (node, entry) = (&tree.nodes[index], tree.entries[index]);
        if node.has(DISABLED) {
            return IGNORED;
        }
        let id = node.id;
        let scroll = node.more.as_ref().and_then(|more| more.scroll);
        let queued = match (request.action, &request.data) {
            (Action::Focus, _) if entry.focusable => {
                let focus = node.focus.unwrap_or(id);
                self.set_focus(Some(focus));
                self.show_focus();
                None
            }
            (Action::Blur, _) if tree.focus == entry.nid => {
                self.set_focus(None);
                None
            }
            (Action::Click, _) if entry.clickable => {
                let hit = node.click.unwrap_or(id);
                // A widget reports a click only where it is visible, as for the pointer.
                // Scrolled into view first, it is clicked one pass later, once it has been
                // laid out where it can be seen.
                if entry.scrolled && self.reveal_accessible(index) == Some(true) {
                    self.a11y.late_clicks.push(hit);
                } else {
                    self.synthesize_click(hit);
                }
                None
            }
            (Action::Click, _) if node.supports(Kind::Click) => Some(AccessAction::Click),
            (Action::Expand, _) if node.supports(Kind::Expand) => Some(AccessAction::Expand),
            (Action::Collapse, _) if node.supports(Kind::Collapse) => Some(AccessAction::Collapse),
            (Action::Increment, _) if node.supports(Kind::Increment) => {
                Some(AccessAction::Increment)
            }
            (Action::Decrement, _) if node.supports(Kind::Decrement) => {
                Some(AccessAction::Decrement)
            }
            (Action::SetValue, Some(ActionData::Value(value))) if node.supports(Kind::SetValue) => {
                Some(AccessAction::SetValue(value.to_string()))
            }
            (Action::SetValue, Some(ActionData::NumericValue(value)))
                if node.supports(Kind::SetValue) =>
            {
                Some(AccessAction::SetNumericValue(*value))
            }
            (Action::ReplaceSelectedText, Some(ActionData::Value(value)))
                if node.supports(Kind::ReplaceSelectedText) =>
            {
                Some(AccessAction::ReplaceSelectedText(value.to_string()))
            }
            (Action::SetTextSelection, Some(ActionData::SetTextSelection(selection)))
                if node.supports(Kind::SetTextSelection) =>
            {
                let offset = |position: &TextPosition| {
                    let (owner, run) = *tree.runs.get(&position.node.0)?;
                    let text = node.more.as_ref()?.text.as_ref()?;
                    (owner as usize == index)
                        .then(|| text.0.offset(run as usize, position.character_index))?
                };
                match (offset(&selection.anchor), offset(&selection.focus)) {
                    (Some(anchor), Some(focus)) => {
                        Some(AccessAction::SetTextSelection { anchor, focus })
                    }
                    _ => return IGNORED,
                }
            }
            (Action::ShowContextMenu, _) if node.supports(Kind::ShowContextMenu) => {
                Some(AccessAction::ShowContextMenu)
            }
            (Action::ShowTooltip, _) if node.supports(Kind::ShowTooltip) => {
                Some(AccessAction::ShowTooltip)
            }
            (Action::HideTooltip, _) if node.supports(Kind::ShowTooltip) => {
                Some(AccessAction::HideTooltip)
            }
            (Action::ScrollIntoView, _) if node.supports(Kind::ScrollIntoView) => {
                Some(AccessAction::ScrollIntoView)
            }
            (Action::ScrollIntoView, _) if entry.scrolled => {
                if self.reveal_accessible(index).is_none() {
                    return IGNORED;
                }
                None
            }
            (
                Action::ScrollUp | Action::ScrollDown | Action::ScrollLeft | Action::ScrollRight,
                data,
            ) if node.supports(Kind::Scroll) => {
                let direction = match request.action {
                    Action::ScrollUp => Vec2::new(0.0, -1.0),
                    Action::ScrollDown => Vec2::new(0.0, 1.0),
                    Action::ScrollLeft => Vec2::new(-1.0, 0.0),
                    _ => Vec2::new(1.0, 0.0),
                };
                let page = matches!(data, Some(ActionData::ScrollUnit(ScrollUnit::Page)));
                match scroll.and_then(|scroll| self.scrolling.states.get(&scroll.id)) {
                    Some(state) => {
                        let step = if page {
                            state.viewport.size()
                        } else {
                            Vec2::splat(ITEM)
                        };
                        let area = scroll.expect("state implies scroll").id;
                        self.scroll_from(area, direction * step, false);
                        None
                    }
                    None => Some(AccessAction::ScrollBy(
                        direction * if page { 1.0 } else { 0.1 },
                    )),
                }
            }
            (Action::SetScrollOffset, Some(ActionData::SetScrollOffset(point)))
                if node.supports(Kind::Scroll) =>
            {
                let scale = if tree.scale > 0.0 { tree.scale } else { 1.0 };
                let target = Vec2::new(point.x as f32, point.y as f32) / scale;
                if !target.is_finite() {
                    return IGNORED;
                }
                match scroll.and_then(|scroll| {
                    let state = self.scrolling.states.get(&scroll.id)?;
                    Some((scroll.id, state.offset, state.visual_scale))
                }) {
                    Some((area, offset, visual)) => {
                        self.scroll_from(area, (target - offset) * visual, false);
                        None
                    }
                    None => Some(AccessAction::SetScrollOffset(target)),
                }
            }
            _ => return IGNORED,
        };
        if let Some(action) = queued {
            self.a11y.pending.entry(id).or_default().push(action);
        }
        self.request_repaint();
        EventResponse {
            consumed: true,
            repaint: true,
        }
    }

    /// Scroll the containers of node `index` so that it is inside their viewports.
    /// `None` without a scrollable container, otherwise whether anything moved.
    fn reveal_accessible(&mut self, index: usize) -> Option<bool> {
        let tree = &self.a11y.tree;
        let scale = if tree.scale > 0.0 { tree.scale } else { 1.0 };
        let rect = tree.nodes[index].rect;
        let (min, max) = (rect.min / scale, rect.max / scale);
        let mut parent = tree.entries[index].parent;
        let mut area = None;
        while parent != NO_PARENT && area.is_none() {
            let node = &tree.nodes[parent as usize];
            area = node.more.as_ref().and_then(|more| more.scroll);
            parent = tree.entries[parent as usize].parent;
        }
        let (area, view) = area.and_then(|scroll| {
            let state = self.scrolling.states.get(&scroll.id)?;
            Some((scroll.id, state.viewport))
        })?;
        // The smallest movement that shows the node; one larger than the viewport is
        // aligned to its start.
        let along = |low: f32, high: f32, view_low: f32, view_high: f32| {
            if low < view_low || high - low > view_high - view_low {
                low - view_low
            } else if high > view_high {
                high - view_high
            } else {
                0.0
            }
        };
        let delta = Vec2::new(
            along(min.x, max.x, view.min.x, view.max.x),
            along(min.y, max.y, view.min.y, view.max.y),
        );
        let moved =
            delta.is_finite() && delta != Vec2::ZERO && self.scroll_from(area, delta, false);
        Some(moved)
    }
}
