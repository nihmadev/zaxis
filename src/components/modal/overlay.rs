//! The overlay under a modal surface: it dims and blurs everything below and is the
//! click target for dismissal. It is registered before the surface, on the modal's
//! popup-class layer, so the surface and its content win hits in the shared hit test.
use super::Ui;
use crate::{components::Sense, context::Paint, Color, Context, Id, Rect, Shape};

/// Dimming color and backdrop blur of a fully shown overlay.
#[derive(Clone, Copy)]
pub(super) struct Tint {
    pub(super) color: Color,
    pub(super) blur: f32,
}

impl Tint {
    /// The tint at visibility `t` (0 hidden, 1 shown); none at all when another
    /// modal of the stack dims already.
    fn at(self, t: f32, dims: bool) -> Self {
        let mut color = self.color;
        color.0[3] = if dims {
            (f32::from(color.0[3]) * t).round() as u8
        } else {
            0
        };
        let blur = if dims { self.blur * t } else { 0.0 };
        Self { color, blur }
    }
}

/// Paint the overlay of `id` over `viewport` and, while the modal is open, take its
/// clicks. True when the overlay was clicked this pass.
pub(super) fn show(
    root: &mut Ui<'_>,
    id: Id,
    viewport: Rect,
    tint: Tint,
    t: f32,
    open: bool,
) -> bool {
    let tint = tint.at(t, dims(root.context, id));
    paint(root.context, id, viewport, tint);
    // A closing modal registers no overlay hit: input below is released at once.
    open && root.interact(viewport, "overlay", Sense::CLICK).clicked
}

/// Stacked modals share the dimming of the lowest one instead of darkening again.
fn dims(context: &Context, id: Id) -> bool {
    context.modals.stack.first().is_none_or(|m| m.id == id)
}

fn paint(context: &mut Context, id: Id, viewport: Rect, tint: Tint) {
    context.paint(
        id.with("overlay"),
        id,
        viewport,
        vec![Paint::Shape(Shape::rect(viewport, tint.color).into())],
    );
    context.paint_blur(
        id.with("overlay-blur"),
        id,
        viewport,
        crate::Blur::new(viewport).radius(tint.blur),
    );
}
