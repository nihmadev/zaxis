//! Opening and closing: whether a modal is built this pass, how visible it is, the
//! popup-class layer and the keys delivered to it, and the one close decision.
//!
//! A modal leaves the stack as soon as it closes, so input below is released at
//! once, while its presence animation may keep it mounted and drawn until it fades.
use super::CloseReason;
use crate::{Context, Id, TweenOptions};

/// Escape and default-action (Enter) presses delivered to the modal this pass.
#[derive(Clone, Copy)]
pub(super) struct Keys {
    pub(super) escape: bool,
    pub(super) enter: bool,
}

/// Everything that can close an open modal in one pass.
#[derive(Clone, Copy)]
pub(super) struct CloseInputs {
    pub(super) open: bool,
    pub(super) escape: bool,
    pub(super) overlay: bool,
    pub(super) close_button: bool,
    /// An action or [`crate::Ui::close_modal`] asked for it.
    pub(super) action: bool,
}

/// Which inputs close the modal ([`crate::Modal::dismiss_on_escape`] and
/// [`crate::Modal::dismiss_on_overlay`]); the others always do.
#[derive(Clone, Copy)]
pub(super) struct Dismiss {
    pub(super) escape: bool,
    pub(super) overlay: bool,
}

fn presence(id: Id) -> Id {
    id.with("presence")
}

/// Whether anything is built: the modal is open or still animating out. A modal the
/// application closed leaves the stack here, before anything is built.
pub(super) fn mounted(context: &mut Context, id: Id, open: bool) -> bool {
    if !open && context.modal_is_open(id) {
        context.end_modal(id);
    }
    open || context.animation_status(presence(id)).is_some()
}

/// Visibility of the surface, 0 hidden to 1 shown. `None` once a closing modal has
/// faded out; its animation is removed then.
pub(super) fn visibility(
    context: &mut Context,
    id: Id,
    open: bool,
    motion: &TweenOptions,
) -> Option<f32> {
    let anim = presence(id);
    let motion = crate::components::sanitize::forward("Modal motion", motion.clone());
    let target = if open { 1.0_f32 } else { 0.0 };
    let t = context
        .transition_visible(anim, Some(0.0), target, motion, true)
        .value
        .clamp(0.0, 1.0);
    if !open && t <= 0.0 {
        context.remove_animation(anim);
        return None;
    }
    Some(t)
}

/// Register the modal's layer for this pass (on the stack while open) and take the
/// keys delivered to it. Enter is published for [`crate::Ui::default_action_pressed`].
pub(super) fn begin(context: &mut Context, id: Id, return_focus: Option<Id>, open: bool) -> Keys {
    context.begin_modal(id, return_focus, open);
    context.push_popup_layer(id);
    context.modals.entered = None;
    let (escape, enter) = if open {
        context.take_modal_keys(id)
    } else {
        (false, false)
    };
    if enter {
        context.modals.entered = Some(id);
    }
    Keys { escape, enter }
}

/// The one reason an open modal closes this pass, by priority:
/// Escape, overlay, close button, then an explicit action.
pub(super) fn close_reason(inputs: CloseInputs, dismiss: Dismiss) -> Option<CloseReason> {
    if !inputs.open {
        None
    } else if inputs.escape && dismiss.escape {
        Some(CloseReason::Escape)
    } else if inputs.overlay && dismiss.overlay {
        Some(CloseReason::Overlay)
    } else if inputs.close_button {
        Some(CloseReason::CloseButton)
    } else if inputs.action {
        Some(CloseReason::Action)
    } else {
        None
    }
}

/// Apply the decision: `open` is written once, and the modal leaves the stack (input
/// and focus return below) while its exit may still be drawn. Ends its build scope.
pub(super) fn finish(context: &mut Context, id: Id, open: &mut bool, closed: Option<CloseReason>) {
    if closed.is_some() {
        *open = false;
        context.end_modal(id);
    }
    context.end_modal_build();
}
