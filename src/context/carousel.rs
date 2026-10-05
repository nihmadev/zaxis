//! Wheel routing for carousels. A carousel registers its hit region as a wheel target; a
//! wheel event over it that runs along its axis is queued for the next pass instead of
//! scrolling the enclosing area, unless a scroll area inside the carousel can take it.
use super::{Context, Id};
use crate::Vec2;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(crate) struct WheelTarget {
    axis: usize,
    /// Scroll areas registered at or after this index lie inside the carousel.
    order: usize,
}

#[derive(Default)]
pub(crate) struct CarouselWheel {
    current: HashMap<Id, WheelTarget>,
    previous: HashMap<Id, WheelTarget>,
    input: HashMap<Id, f32>,
}

impl CarouselWheel {
    pub(crate) fn begin_frame(&mut self) {
        self.current.clear();
    }
    pub(crate) fn finish_frame(&mut self) {
        self.previous = std::mem::take(&mut self.current);
        self.input.retain(|id, _| self.previous.contains_key(id));
    }
    pub(crate) fn queued(&self) -> usize {
        self.input.len()
    }
}

impl Context {
    /// Make the hit region `id` receive wheel movement along `axis` (0 horizontal, 1 vertical).
    pub(crate) fn carousel_wheel_register(&mut self, id: Id, axis: usize) {
        let order = self.scrolling.order.len();
        self.carousel_wheel
            .current
            .insert(id, WheelTarget { axis, order });
    }

    /// Wheel distance queued for `id` since the previous pass: positive turns forward.
    pub(crate) fn carousel_wheel_take(&mut self, id: Id) -> f32 {
        self.carousel_wheel.input.remove(&id).unwrap_or(0.0)
    }

    /// Queue `delta` (positive scrolls forward) for the topmost carousel under the pointer.
    /// `inner` is the scroll area under the pointer, if any. Returns whether it was taken.
    pub(super) fn route_carousel_wheel(
        &mut self,
        pointer: Vec2,
        window: Id,
        delta: Vec2,
        inner: Option<Id>,
    ) -> bool {
        let found = self.previous_hits.iter().rev().find_map(|hit| {
            let target = self.carousel_wheel.previous.get(&hit.id)?;
            (hit.window == window && hit.rect.contains(pointer) && hit.clip.contains(pointer))
                .then_some((hit.id, *target))
        });
        let Some((id, target)) = found else {
            return false;
        };
        let (along, across) = (delta[target.axis], delta[1 - target.axis]);
        if along == 0.0 || along.abs() < across.abs() {
            return false;
        }
        if let Some((area_id, area)) =
            inner.and_then(|id| self.scrolling.states.get(&id).map(|area| (id, area)))
        {
            let inside = self
                .scrolling
                .previous_order
                .iter()
                .position(|id| *id == area_id)
                .is_some_and(|index| index >= target.order);
            let max = area.max_offset();
            let room = if along > 0.0 {
                area.offset[target.axis] < max[target.axis]
            } else {
                area.offset[target.axis] > 0.0
            };
            if inside && area.axes[target.axis] && room {
                return false;
            }
        }
        *self.carousel_wheel.input.entry(id).or_default() += along;
        true
    }
}
