//! A drag value that is not being edited: a surface showing the formatted value with its
//! affixes, whose hit region takes drags, clicks and adjustment keys.

use super::{NumberOptions, NumberStyle, Numeric};
use crate::{
    components::appearance::Appearance,
    context::{HitAction, HitRegion, Paint},
    Border, Id, Response, Ui, Vec2,
};

pub(super) fn show<T: Numeric>(
    ui: &mut Ui<'_>,
    id: Id,
    value: T,
    options: &NumberOptions<'_, T>,
    style: &NumberStyle,
) -> Response {
    let rect = ui.allocate_space(Vec2::new(
        style.width.min(ui.available_width()),
        style.height,
    ));
    let response = ui.response(id, rect, options.enabled);
    ui.context.register_hit(HitRegion {
        id,
        window: ui.window,
        rect,
        clip: ui.clip,
        action: if options.enabled {
            HitAction::DragValue
        } else {
            HitAction::Block
        },
    });
    let shown = format!(
        "{}{}{}",
        options.prefix,
        options.display(value),
        options.suffix
    );
    let inner = style.padding.inset(rect);
    let text_height = ui
        .context
        .measure_text(
            &shown,
            style.font_size,
            ui.style().typography.weights.body,
            f32::INFINITY,
        )
        .y;
    let effective = ui.style().clone();
    let mut base = Appearance::new(style.fill, Border::NONE, effective.text_color);
    base.rounding = style.rounding;
    base.blur = 0.0;
    base.opacity = effective.opacity;
    let status = ui.field_status(options.status);
    base.status = ui.status_color(status);
    let mut control = crate::ControlState::from_response(response, false);
    control.status = status;
    let appearance = ui.animate_control(
        response,
        options
            .hover_style
            .unwrap_or(crate::HoverStyle::fill(style.hovered)),
        options.hover_style.is_some(),
        style.surface,
        control,
        base,
        style.hovered,
    );
    let mut paint = Vec::new();
    appearance.paint_shadow(rect, appearance.rounding, &mut paint);
    appearance.paint_body(
        rect,
        appearance.rounding,
        &effective,
        appearance.blur,
        &mut paint,
    );
    let (_, filter) = ui.resolved_blur(appearance.blur, style.surface.has_blur_override());
    ui.context.paint_blur(
        id.with("blur"),
        ui.window,
        ui.clip,
        crate::Blur::new(rect)
            .radius(filter)
            .corner_radius(appearance.rounding),
    );
    ui.context.paint(id.with("body"), ui.window, ui.clip, paint);
    ui.context.paint(
        id.with("text"),
        ui.window,
        ui.clip.intersect(inner),
        vec![Paint::Text {
            text: shown,
            position: Vec2::new(inner.min.x, inner.center().y - text_height * 0.5),
            size: style.font_size,
            weight: ui.style().typography.weights.body,
            wrap_width: f32::INFINITY,
            color: if options.enabled {
                appearance.text_color
            } else {
                ui.style().disabled_text
            },
        }],
    );
    response
}
