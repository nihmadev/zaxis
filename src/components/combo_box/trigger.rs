//! The trigger: its ids, style, place, hit region and accessibility node before the pass's
//! input; its focus, hover, description and open animation after it.

use super::{access, ComboBox, ComboBoxState, ComboBoxStyle, Popup, Response, Ui};
use crate::{
    context::{HitAction, HitRegion},
    Easing, Id, Rect, TweenOptions, Vec2,
};

/// One pass's trigger, resolved before any input is applied.
pub(super) struct Trigger {
    pub(super) id: Id,
    /// The popup, for its dismissals and as the hover layer while it is open.
    pub(super) popup: Id,
    /// The list's accessibility node.
    pub(super) list: Id,
    pub(super) style: ComboBoxStyle,
    /// The label above the trigger and the trigger.
    pub(super) allocation: Rect,
    pub(super) rect: Rect,
    /// Enabled itself and inside an enabled `Ui`.
    pub(super) enabled: bool,
    /// Some of the trigger is inside the clip.
    pub(super) visible: bool,
    /// Enabled, visible and with options: it may open. Otherwise it is forced closed.
    pub(super) usable: bool,
    /// The trigger's accessibility node, completed by [`Trigger::describe`].
    node: usize,
}

impl<T> ComboBox<'_, T> {
    /// Resolves the style and places the trigger: its response, its hit region and the
    /// closed description of its accessibility node.
    pub(super) fn trigger(&self, ui: &mut Ui<'_>, label: &str) -> (Trigger, Response) {
        let id = match self.id {
            Some(id) => ui.scope.with(("combo-box", id)),
            None => ui.auto_id(("combo-box", self.source)),
        };
        let popup = Popup::id(ui, Id::new(id));
        let style = self.resolved_style(&ui.style().combo_box);
        let enabled = self.enabled && ui.enabled;
        let label_height = if label.is_empty() {
            0.0
        } else {
            style.label_height + style.label_gap
        };
        let allocation = ui.allocate_space(Vec2::new(
            style.width.min(ui.available_width()),
            style.trigger_height + label_height,
        ));
        let rect = Rect::from_min_max(
            allocation.min + Vec2::new(0.0, label_height),
            allocation.max,
        );
        let response = ui.response(id, rect, enabled);
        ui.context.register_hit(HitRegion {
            id,
            window: ui.window,
            rect,
            clip: ui.clip,
            action: if enabled {
                HitAction::ComboBox
            } else {
                HitAction::Block
            },
        });
        // Described first, completed once this pass's input has been applied.
        let node = ui.context.a11y_len();
        ui.a11y(id, rect, crate::AccessRole::ComboBox, |node| {
            access::trigger(node, id, label, enabled);
        });
        let visible = !rect.intersect(ui.clip_rect()).is_empty();
        let trigger = Trigger {
            id,
            popup,
            list: id.with("listbox"),
            style,
            allocation,
            rect,
            enabled,
            visible,
            usable: enabled && !self.options.is_empty() && visible,
            node,
        };
        (trigger, response)
    }
}

impl Trigger {
    pub(super) fn motion(&self) -> TweenOptions {
        TweenOptions::new(self.style.animation_duration).easing(Easing::CubicOut)
    }

    /// Progress of the open animation, retargeted to `open`.
    pub(super) fn animate(&self, ui: &mut Ui<'_>, open: bool) -> f32 {
        ui.context
            .transition_visible(
                self.id.with("open"),
                Some(0.0),
                if open { 1.0 } else { 0.0 },
                self.motion(),
                self.visible,
            )
            .value
    }

    /// Turns the open animation back toward closed, for a popup closed after
    /// [`Trigger::animate`] ran in this pass.
    pub(super) fn animate_closed(&self, ui: &mut Ui<'_>) {
        ui.context.transition_visible(
            self.id.with("open"),
            None,
            0.0_f32,
            self.motion(),
            self.visible,
        );
    }

    /// Focus and hover once the input is applied: while the popup is open, the trigger is
    /// hovered on the popup's layer.
    pub(super) fn respond(&self, ui: &mut Ui<'_>, response: &mut Response, open: bool) {
        response.has_focus = ui.context.has_focus(self.id);
        response.focus_visible = ui.context.focus_visible(self.id);
        let layer = if open { self.popup } else { ui.window };
        response.hovered = ui
            .context
            .hovered(self.id, layer, self.rect, ui.clip_rect());
    }

    /// Completes the accessibility node: the chosen label or the placeholder, the open
    /// state, validity and, while open, the list and its highlighted option.
    pub(super) fn describe(
        &self,
        ui: &mut Ui<'_>,
        value: Option<&str>,
        placeholder: &str,
        state: &ComboBoxState,
        invalid: bool,
    ) {
        let Some(node) = ui.context.a11y_node_mut(self.node) else {
            return;
        };
        access::trigger_state(node, value, placeholder, state.open, invalid);
        if state.open {
            access::popup_owner(node, self.id, self.list, state.active);
        }
    }
}
