//! Middle-button autoscroll. The gesture stays attached to its initial scroll tree.
use super::{Context, Id};
use crate::time::Instant;
use crate::Vec2;
use std::time::Duration;
use winit::event::ElementState;

const DEAD_ZONE: f32 = 8.0;
const SPEED: f32 = 8.0;
const MAX_SPEED: f32 = 1800.0;
const INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Copy)]
pub struct AutoScroll {
    area: Id,
    origin: Vec2,
    held: bool,
    dragged: bool,
    last_tick: Option<Instant>,
}
impl AutoScroll {
    fn velocity(self, pointer: Option<Vec2>) -> Vec2 {
        let Some(pointer) = pointer else {
            return Vec2::ZERO;
        };
        let delta = pointer - self.origin;
        let speed = (delta.abs() - Vec2::splat(DEAD_ZONE)).max(Vec2::ZERO) * SPEED;
        delta.signum() * speed.min(Vec2::splat(MAX_SPEED))
    }
}
impl Context {
    /// Whether a middle-button autoscroll gesture is active, including its dead zone.
    pub fn is_auto_scrolling(&self) -> bool {
        self.scrolling.auto.is_some()
    }

    pub(super) fn middle_button(&mut self, state: ElementState) -> bool {
        self.input.middle_down = state == ElementState::Pressed;
        if let Some(consumed) = self.gesture_middle(state == ElementState::Pressed) {
            return consumed;
        }
        if state == ElementState::Released {
            let Some(auto) = self.scrolling.auto.as_mut() else {
                return false;
            };
            if auto.dragged {
                self.stop_auto_scroll();
            } else {
                auto.held = false;
            }
            return true;
        }
        if self.stop_auto_scroll() {
            return true;
        }
        if !self.input.focused || self.capture.is_some() {
            return false;
        }
        let Some(pointer) = self.input.pointer else {
            return false;
        };
        let Some(window) = self.top_window(pointer) else {
            return false;
        };
        let area = self
            .scrolling
            .previous_order
            .iter()
            .rev()
            .copied()
            .find(|id| {
                let s = &self.scrolling.states[id];
                s.window == window
                    && s.enabled
                    && s.middle_mouse_scroll
                    && s.clip.contains(pointer)
                    && (self.auto_can_move(*id, Vec2::ONE) || self.auto_can_move(*id, -Vec2::ONE))
            });
        let Some(area) = area else {
            return false;
        };
        self.scrolling.auto = Some(AutoScroll {
            area,
            origin: pointer,
            held: true,
            dragged: false,
            last_tick: None,
        });
        true
    }
    pub(super) fn stop_auto_scroll(&mut self) -> bool {
        self.scrolling.auto_deadline = None;
        self.scrolling.auto.take().is_some()
    }
    pub(super) fn update_auto_scroll_pointer(&mut self, pointer: Vec2) {
        if let Some(auto) = self.scrolling.auto.as_mut() {
            auto.dragged |= auto.held && (pointer - auto.origin).abs().max_element() > DEAD_ZONE;
            if self.scrolling.auto_deadline.is_none() {
                auto.last_tick = None;
            }
        }
    }
    fn auto_can_move(&self, id: Id, velocity: Vec2) -> bool {
        let mut current = Some(id);
        while let Some(id) = current {
            let Some(s) = self.scrolling.states.get(&id) else {
                return false;
            };
            if s.enabled && s.middle_mouse_scroll {
                let max = s.max_offset();
                for axis in 0..2 {
                    if s.axes[axis]
                        && ((velocity[axis] > 0.0 && s.offset[axis] < max[axis])
                            || (velocity[axis] < 0.0 && s.offset[axis] > 0.0))
                    {
                        return true;
                    }
                }
            }
            current = s.parent;
        }
        false
    }
    pub(super) fn tick_auto_scroll(&mut self) {
        self.scrolling.auto_deadline = None;
        let Some(mut auto) = self.scrolling.auto else {
            return;
        };
        if !self.scrolling.previous_order.contains(&auto.area) {
            self.stop_auto_scroll();
            return;
        }
        let seconds = auto.last_tick.map_or(0.0, |last| {
            self.frame_time
                .saturating_duration_since(last)
                .min(Duration::from_millis(100))
                .as_secs_f32()
        });
        let velocity = auto.velocity(self.input.pointer);
        if seconds > 0.0 && velocity != Vec2::ZERO {
            self.scroll_from(auto.area, velocity * seconds, true);
        }
        auto.last_tick = Some(self.frame_time);
        self.scrolling.auto = Some(auto);
    }
    pub(super) fn finish_auto_scroll(&mut self) {
        let Some(auto) = self.scrolling.auto else {
            return;
        };
        let visible = self.scrolling.states.get(&auto.area).is_some_and(|s| {
            s.last_frame == self.frame && s.enabled && s.middle_mouse_scroll && !s.clip.is_empty()
        });
        if !visible {
            self.stop_auto_scroll();
            return;
        }
        let velocity = auto.velocity(self.input.pointer);
        if velocity != Vec2::ZERO && self.auto_can_move(auto.area, velocity) {
            self.scrolling.auto_deadline = self.frame_time.checked_add(INTERVAL);
        } else {
            self.scrolling.auto_deadline = None;
            self.scrolling.auto.as_mut().unwrap().last_tick = None;
        }
    }
}
