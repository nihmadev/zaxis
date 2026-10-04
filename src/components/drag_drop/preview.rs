//! Application-built drag preview. It lives in the overlay layer next to the
//! snapshot preview: no hits, no clipping by the source's window, no input.
use crate::{
    context::drag::preview::layer_id, layout::LayoutCursor, Align, Layout, Rect, Transform, Ui,
    Vec2,
};

pub(super) fn build_custom(ui: &mut Ui<'_>, build: Box<dyn FnOnce(&mut Ui<'_>) + '_>) {
    let Some(session) = ui.context.drag.session.as_ref().filter(|s| s.started) else {
        return;
    };
    let resolved = session.info.resolved;
    let origin = if session.keyboard {
        session.pointer + Vec2::splat(12.0)
    } else {
        session.pointer - session.grab
    };
    let scale = ui.context.scale_factor();
    let origin = (origin * scale).round() / scale;
    let layer = layer_id();
    let viewport = ui.context.viewport();
    let bounds = Rect::from_min_size(origin, resolved.preview_max_size);
    let spacing = ui.style().spacing.max(0.0);
    let mut child = Ui {
        flow: None,
        context: &mut *ui.context,
        window: layer,
        scope: layer.with("custom"),
        sequence: 0,
        clip: bounds.intersect(viewport),
        layout: LayoutCursor::new(bounds, Layout::Vertical, spacing),
        enabled: false,
        backdrop_blur: 0.0,
        hover_style: None,
        local_style: ui.local_style.clone(),
        local_style_revision: ui.local_style_revision,
    };
    child.begin_layout(Align::Start);
    child.visual(
        "opacity",
        Transform::IDENTITY,
        resolved.preview_opacity,
        |ui| build(ui),
    );
    child.finish_layout();
    let frame = child.context.frame;
    child.context.drag.custom_preview = frame;
}
