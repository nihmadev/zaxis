//! One photo of the `Images` variant: Skeleton while loading, a fade-in, an error block.
use crate::{
    components::{Image, ImageFit, Skeleton, Ui},
    context::Paint,
    Border, Color, CornerRadius, DiagnosticKind, Id, ImageSource, ImageState, Rect, Shape, Vec2,
};

use super::region::with_region;

/// Draw `source` covering `rect` with rounded corners.
///
/// A photo that is still loading shows a Skeleton; once it is ready it fades in over the
/// Skeleton, so there is no flash between the two. A photo that failed to load shows a muted
/// block with a short message and is reported through diagnostics, never a panic.
pub(super) fn photo(ui: &mut Ui<'_>, scope: Id, rect: Rect, source: ImageSource, rounding: f32) {
    if rect.size().min_element() <= 0.0 {
        return;
    }
    let state = ui.context.image_state(source.clone());
    if let ImageState::Error(error) = &state {
        let message = format!("image failed: {error}");
        ui.context
            .report(DiagnosticKind::External, Some(scope), Some(rect), || {
                message
            });
        return failure(ui, scope, rect, rounding);
    }
    let motion = ui.style().motion.presence.clone();
    let fade = ui
        .transition(
            ("fade", scope),
            if state.is_ready() { 1.0_f32 } else { 0.0 },
            motion,
        )
        .value
        .clamp(0.0, 1.0);
    if fade < 1.0 {
        with_region(ui, scope.with("skeleton"), rect, rect, false, |u| {
            u.add(
                Skeleton::new(rect.size().y)
                    .width(rect.size().x)
                    .corner_radius(rounding)
                    .id_source(scope),
            );
        });
    }
    with_region(ui, scope.with("image"), rect, rect, false, |u| {
        u.add(
            Image::new(source)
                .id_source(scope)
                .size(rect.size())
                .fit(ImageFit::Cover)
                .corner_radius(rounding)
                .opacity(fade)
                .placeholder(false)
                .decorative(),
        );
    });
}

fn failure(ui: &mut Ui<'_>, scope: Id, rect: Rect, rounding: f32) {
    let style = ui.style().clone();
    let text = "Image unavailable";
    let size = style.font_size;
    let weight = style.typography.weights.control;
    let measured = ui.context.measure_text(text, size, weight, f32::INFINITY);
    let offset = ui.context.centered_line_offset(text, size, weight);
    let position = rect.center() - measured * 0.5 + Vec2::new(0.0, offset);
    ui.context.paint(
        scope.with("failed"),
        ui.window,
        ui.clip,
        vec![
            Paint::Shape(Shape::Rect {
                rect,
                fill: style.button_fill,
                rounding: CornerRadius::all(rounding),
                border: Border::NONE,
            }),
            Paint::Text {
                text: text.to_owned(),
                position,
                size,
                weight,
                wrap_width: f32::INFINITY,
                color: Color(style.muted_text.0),
            },
        ],
    );
}
