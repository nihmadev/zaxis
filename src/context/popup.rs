use super::{Context, Id};
use crate::Rect;

pub(crate) struct PopupState {
    pub id: Id,
    pub owner: Id,
    pub anchor: Rect,
    pub rect: Rect,
    pub return_focus: Option<Id>,
    pub key_target: Option<Id>,
    pub last_frame: u64,
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
