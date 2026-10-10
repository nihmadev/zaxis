//! Where a paint goes: content of a measured placement or of a scroll area waits until it
//! is placed; everything else is drawn into this pass's elements. Paint order is the
//! order of first records, whichever way a paint arrived.

use super::Paint;
use crate::{
    context::{Context, Id},
    Color, Rect, Shape,
};

impl Context {
    pub(crate) fn record_paint_order(&mut self, id: Id) {
        self.paint_state.record_order(id);
    }

    /// Paint a cached shape behind all panels. Call before building windows.
    /// This provides a backdrop for translucent panels and their blur effects.
    pub fn paint_background(&mut self, shape: impl Into<Shape>) {
        let id = Id::new(("background", self.paint_state.elements.len()));
        self.paint(
            id,
            Id::new("background-layer"),
            self.viewport(),
            vec![Paint::Shape(shape.into())],
        );
    }

    pub(crate) fn paint_blur(&mut self, id: Id, layer: Id, clip: Rect, blur: crate::Blur) {
        if blur.radius <= 0.0 || blur.rect.is_empty() {
            return;
        }
        self.paint(
            id,
            layer,
            clip,
            vec![Paint::Shape(Shape::Rect {
                rect: blur.rect,
                fill: Color::WHITE,
                rounding: blur.rounding,
                border: crate::Border::NONE,
            })],
        );
        self.mark_blur(id, blur.radius);
    }

    /// Make the paint just recorded under `id` a backdrop blur, wherever it went.
    pub(crate) fn mark_blur(&mut self, id: Id, sigma: f32) {
        if let Some(paint) = self
            .placements
            .stack
            .last_mut()
            .and_then(|p| p.paints.last_mut())
            .filter(|p| p.id == id)
        {
            paint.blur = Some(sigma);
        } else if let Some(paint) = self.scrolling.pending.last_mut().filter(|p| p.id == id) {
            paint.blur = Some(sigma);
        } else if let Some(element) = self.paint_state.last_element_mut(id) {
            element.blur = Some(sigma);
        }
    }

    /// Defer `paint` to the open placement or scroll scope that owns `layer`, or give it
    /// back to be drawn now. A scroll-deferred paint takes its order when recorded.
    pub(super) fn route_paint(
        &mut self,
        id: Id,
        layer: Id,
        clip: Rect,
        paint: Vec<Paint>,
    ) -> Option<Vec<Paint>> {
        let paint = self.defer_placement_paint(id, layer, clip, paint).err()?;
        self.record_paint_order(id);
        self.defer_scroll_paint(id, layer, clip, paint).err()
    }
}
