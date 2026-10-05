//! Card and slide body: theme surface, soft shadow, rounded fill.
use super::style::Look;
use crate::{
    components::{appearance::Appearance, Ui},
    Border, Color, CornerRadius, Gradient, Id, Interpolate, Rect, Shadow, Vec2,
};

/// The surface of the front card or, with `sheet`, of a sheet behind it.
fn surface(ui: &Ui<'_>, look: &Look, sheet: bool) -> Appearance {
    let style = ui.style();
    let mut body = Appearance::new(Color::TRANSPARENT, Border::NONE, style.text_color);
    body.apply(style.card.surface);
    body.apply(look.style.card);
    if sheet {
        body.apply(look.style.sheet);
    }
    // Without a theme the card surface is empty: fall back to a raised window fill.
    if body.fill.start.0[3] == 0 && body.fill.end.0[3] == 0 {
        body.fill = Gradient::new(style.window_fill, style.window_fill);
    }
    let shadow_set = style.card.surface.shadow.is_some()
        || look.style.card.shadow.is_some()
        || (sheet && look.style.sheet.shadow.is_some());
    if !shadow_set {
        body.shadow = Shadow {
            color: Color::rgba(0, 0, 0, if sheet { 28 } else { 46 }),
            offset: Vec2::new(0.0, if sheet { 3.0 } else { 8.0 }),
            blur_radius: if sheet { 8.0 } else { 20.0 },
            spread: 0.0,
        };
    }
    body.opacity = style.opacity;
    body.ring = None;
    body
}

/// Paint a card at distance `depth` pages behind the front one. The look changes
/// continuously from the front card (0) to a sheet (1), so a card arriving at the front
/// never jumps in shadow or fill.
pub(super) fn paint_card(ui: &mut Ui<'_>, id: Id, rect: Rect, look: &Look, depth: f32) {
    let t = if depth.is_finite() {
        depth.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let front = surface(ui, look, false);
    let body = if t > 0.0 {
        front.interpolate(&surface(ui, look, true), t)
    } else {
        front
    };
    let rounding = CornerRadius::all(look.rounding());
    let mut paint = Vec::new();
    body.paint_shadow(rect, rounding, &mut paint);
    body.paint_body(rect, rounding, ui.style(), 0.0, &mut paint);
    ui.context.paint(id, ui.window, ui.clip, paint);
}
