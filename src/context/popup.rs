use super::{Context, Id};
use crate::{AccessAction, AccessNode, Rect};

pub struct PopupState {
    pub id: Id,
    pub owner: Id,
    pub anchor: Rect,
    pub rect: Rect,
    pub return_focus: Option<Id>,
    pub key_target: Option<Id>,
    pub last_frame: u64,
    /// Extra panels owned by this popup (cascading submenus): their layer and rect.
    /// Presses inside them are not outside presses, and they win hover routing.
    pub extra: Vec<(Id, Rect)>,
}

/// What a wrapper (a tooltip, a context menu) notes before it builds its widget; see
/// [`Context::a11y_wrap`].
pub(crate) struct Wrapped {
    start: usize,
    requests: Vec<(Id, AccessAction)>,
}

impl Context {
    /// Close the active popup now, removing hits and restoring its trigger focus.
    pub fn close_popup(&mut self) {
        self.dismiss_popup(true);
    }
    pub(crate) fn dismiss_popup(&mut self, restore_focus: bool) {
        if let Some(popup) = self.popup.take() {
            if let Some(target) = popup.key_target {
                self.combo_input.remove(&target);
            }
            self.dismissed_popups.insert(popup.id);
            self.remove_popup_hits(popup.id);
            if restore_focus {
                self.set_focus(popup.return_focus);
            }
            self.keyboard_active = None;
            self.stop_auto_scroll();
            self.request_repaint();
        }
    }

    pub(crate) fn remove_popup_hits(&mut self, id: Id) {
        for placement in &mut self.placements.stack {
            placement.hits.retain(|(hit, _)| hit.window != id);
        }
        self.hits.retain(|hit| hit.window != id);
        self.previous_hits.retain(|hit| hit.window != id);
        self.scrolling.remove_layer_hits(id);
        if self.capture.is_some_and(|capture| capture.hit.window == id) {
            self.capture = None;
        }
    }

    /// The node of widget `id` among those described so far in this pass, newest first:
    /// an overlay attached right after its target (a tooltip, a context menu) finds it at
    /// once.
    pub(crate) fn a11y_node_of(&mut self, id: Id) -> Option<&mut AccessNode> {
        self.a11y.nodes.iter_mut().rev().find(|node| node.id == id)
    }

    /// Requests of assistive technology that `wanted` picks and no widget of this pass has
    /// taken yet, with the node each is for. They are read, not taken: an overlay attached
    /// to a widget shares that widget's node, and so its queue.
    pub(crate) fn a11y_requests(
        &self,
        wanted: impl Fn(&AccessAction) -> bool,
    ) -> Vec<(Id, AccessAction)> {
        let mut found = Vec::new();
        for (id, requests) in &self.a11y.pending {
            found.extend(
                requests
                    .iter()
                    .filter(|request| wanted(request))
                    .map(|request| (*id, request.clone())),
            );
        }
        found
    }

    /// Before a wrapper builds the widget it wraps: where that widget's nodes will start,
    /// and the requests `wanted` picks. The widget may take every request for its node,
    /// the wrapper's included, so they are read first.
    pub(crate) fn a11y_wrap(&self, wanted: impl Fn(&AccessAction) -> bool) -> Wrapped {
        Wrapped {
            start: self.a11y_len(),
            requests: self.a11y_requests(wanted),
        }
    }

    /// After the wrapped widget was built: the index of its first node and the requests
    /// that were waiting for that node, or `None` when it described nothing.
    pub(crate) fn a11y_wrapped(&mut self, wrapped: Wrapped) -> Option<(usize, Vec<AccessAction>)> {
        let id = self.a11y_node_mut(wrapped.start)?.id;
        let requests = wrapped.requests.into_iter();
        Some((
            wrapped.start,
            requests
                .filter(|(node, _)| *node == id)
                .map(|(_, request)| request)
                .collect(),
        ))
    }

    pub(crate) fn layer_rank(&self, id: Id) -> usize {
        if let Some(rank) = self.popup_layers.iter().position(|layer| *layer == id) {
            self.layers.len() + 1 + rank
        } else {
            self.layers
                .iter()
                .position(|layer| *layer == id)
                .map_or(0, |rank| rank + 1)
        }
    }
}
