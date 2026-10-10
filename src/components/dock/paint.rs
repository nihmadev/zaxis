use super::style::Look;
use crate::{context::Paint, Id, Rect, Shape, Ui};

pub(super) fn surface(ui: &mut Ui<'_>, id: Id, rect: Rect, look: &Look, clip: Rect) {
    if look.surface.blur > 0.0 {
        ui.context.paint_blur(
            id.with("blur"),
            ui.window,
            clip,
            crate::Blur::new(rect)
                .corner_radius(look.radius)
                .radius(look.surface.blur),
        );
    }
    let mut paint = Vec::new();
    look.surface.paint_shadow(rect, look.radius, &mut paint);
    look.surface
        .paint_body(rect, look.radius, ui.style(), ui.backdrop_blur, &mut paint);
    ui.context.paint(id, ui.window, clip, paint);
}
pub(super) fn preview(ui: &mut Ui<'_>, id: Id, rect: Rect, amount: f32, look: &Look) {
    let color = super::focus_ring::alpha(look.preview, 0.18 * amount);
    ui.context.paint(
        id,
        ui.window,
        ui.clip_rect(),
        vec![Paint::Shape(
            Shape::rect(rect, color)
                .corner_radius(look.radius)
                .border(crate::Border::new(
                    1.5,
                    super::focus_ring::alpha(look.preview, amount),
                ))
                .into(),
        )],
    );
}
