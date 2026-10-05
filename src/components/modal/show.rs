//! One pass of a modal, as a sequence of stages:
//!
//! 1. lifecycle: whether it is mounted, its visibility, its layer and keys;
//! 2. look and geometry: the resolved style placed with the previous measurements;
//! 3. overlay: dimming, backdrop blur and the dismissal click target;
//! 4. surface: chrome, input block, parts and close button inside the dialog node;
//! 5. measurements kept for the next pass (a follow-up pass only on change);
//! 6. the one close decision, written to `open` once.
use super::{
    access,
    geometry::{geometry, pose, Measure},
    lifecycle::{self, CloseInputs, Dismiss},
    look::Look,
    overlay,
    surface::{self, Content, Frame},
    Modal, ModalOutput, Ui,
};
use crate::{layout::LayoutCursor, Color, Context, Id, Layout, Rect};

pub(super) fn run<R>(
    modal: Modal,
    ui: &mut Ui<'_>,
    open: &mut bool,
    content: Content<'_, impl FnOnce(&mut Ui<'_>) -> R>,
) -> Option<ModalOutput<R>> {
    let id = ui.scope.with(("modal", modal.source));
    let is_open = *open;
    if !lifecycle::mounted(ui.context, id, is_open) {
        return None;
    }
    let viewport = ui.context.viewport();
    if viewport.is_empty() {
        // A minimized window has no room; keep `open` for when it returns.
        return None;
    }
    // `Style` is large: copy out only what is needed instead of cloning it.
    let look = Look::of(ui.style());
    let t = lifecycle::visibility(ui.context, id, is_open, &look.presence)?;
    let keys = lifecycle::begin(ui.context, id, modal.return_focus, is_open);

    let resolved = look.resolve(modal.style, viewport);
    let previous = ui
        .context
        .modals
        .measures
        .get(&id)
        .copied()
        .unwrap_or_default();
    let metrics = resolved.metrics(modal.anchor, modal.width, modal.max_height);
    let geo = geometry(viewport, &metrics, &previous, resolved.surface.rounding);

    let mut root = layer_root(ui, id, viewport, is_open, resolved.surface.text_color);
    let overlay_clicked = overlay::show(&mut root, id, viewport, resolved.overlay, t, is_open);

    let frame = Frame {
        id,
        rect: geo.surface,
        viewport,
        rounding: geo.rounding,
        body_height: geo.body_height,
        transform: pose(
            modal.anchor,
            geo.surface,
            t,
            resolved.enter_scale,
            resolved.enter_offset,
        ),
        // Hidden until its parts were measured once, so it never shows unplaced.
        opacity: if previous.last_frame != 0 { t } else { 0.0 },
        open: is_open,
        bodyless: modal.bodyless,
        close: modal.close_button.then_some(resolved.close_size),
    };
    let dialog = access::open(root.context, id, modal.role, modal.label.as_deref());
    let built = surface::build(&mut root, &frame, &resolved, content);
    let closing = (!is_open).then_some(id);
    let bounds = (geo.surface, viewport);
    access::close(
        root.context,
        dialog,
        built.titles,
        modal.described,
        bounds,
        closing,
    );
    retain_measure(root.context, id, &previous, built.measure);

    let inputs = CloseInputs {
        open: is_open,
        escape: keys.escape,
        overlay: overlay_clicked,
        close_button: built.close_clicked,
        action: root.context.modals.close_requested == Some(id),
    };
    let dismiss = Dismiss {
        escape: modal.escape,
        overlay: modal.overlay,
    };
    let closed = lifecycle::close_reason(inputs, dismiss);
    lifecycle::finish(root.context, id, open, closed);
    Some(ModalOutput {
        inner: built.inner,
        closed,
        default_action: keys.enter && is_open,
        rect: geo.surface,
    })
}

/// The modal's own root: the whole viewport on its layer, taking input only while
/// open, with the surface's text color for everything inside it (like a popup).
fn layer_root<'u>(
    ui: &'u mut Ui<'_>,
    id: Id,
    viewport: Rect,
    open: bool,
    text_color: Color,
) -> Ui<'u> {
    let mut style = ui.style().clone();
    style.text_color = text_color;
    Ui {
        flow: None,
        context: &mut *ui.context,
        window: id,
        scope: id,
        sequence: 0,
        clip: viewport,
        layout: LayoutCursor::new(viewport, Layout::Vertical, 0.0),
        enabled: open,
        backdrop_blur: 0.0,
        hover_style: ui.hover_style,
        local_style: Some(std::sync::Arc::new(style)),
        local_style_revision: ui.local_style_revision,
    }
}

/// Keep this pass's measurements for the next one. Only a change asks for the
/// follow-up pass that places the surface with them; stable sizes need no frames.
fn retain_measure(context: &mut Context, id: Id, previous: &Measure, next: Measure) {
    let changed = next.changed_from(previous);
    context.modals.measures.insert(id, next);
    if changed {
        context.request_repaint();
    }
}
