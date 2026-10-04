//! Autoscroll while dragging near the edge of a scrollable container.
//!
//! Speed is a function of pointer distance inside the edge band, integrated over
//! the pass clock, so it does not depend on how often the mouse reports. Values
//! are logical pixels per second and therefore identical at every DPI.
use super::super::{scroll::ScrollState, Context, Id};
use crate::{Rect, Vec2};
use std::time::Duration;

const MAX_STEP: Duration = Duration::from_millis(100);

/// Signed velocity along one axis for a pointer at `p` inside `lo..hi`.
fn axis_speed(p: f32, lo: f32, hi: f32, edge: f32, min: f32, max: f32) -> f32 {
    // Bands never overlap, so a small container still has a calm middle.
    let zone = edge.min((hi - lo) * 0.5);
    if zone <= 0.0 {
        return 0.0;
    }
    let (depth, sign) = if p < lo + zone {
        ((lo + zone - p) / zone, -1.0)
    } else if p > hi - zone {
        ((p - (hi - zone)) / zone, 1.0)
    } else {
        return 0.0;
    };
    let k = depth.clamp(0.0, 1.0);
    sign * (min + (max - min) * k * k)
}

fn can_move(state: &ScrollState, velocity: Vec2) -> bool {
    let max = state.max_offset();
    (0..2).any(|axis| {
        state.axes[axis]
            && ((velocity[axis] > 0.0 && state.offset[axis] < max[axis])
                || (velocity[axis] < 0.0 && state.offset[axis] > 0.0))
    })
}

impl Context {
    /// Called at the start of every pass, before the UI reads scroll offsets.
    pub(crate) fn tick_drag_autoscroll(&mut self) {
        self.drag.scrolling = false;
        let now = self.frame_time;
        let viewport = self.viewport();
        let Some(session) = self.drag.session.as_mut() else {
            return;
        };
        if !session.started || session.keyboard {
            return;
        }
        let dt = session
            .last_tick
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last))
            .min(MAX_STEP)
            .as_secs_f32();
        session.last_tick = Some(now);
        let resolved = session.info.resolved;
        // Past the native window the pointer counts as sitting on its border.
        let probe = session
            .pointer
            .clamp(viewport.min, viewport.max - Vec2::splat(0.01));
        let window = self.top_window(probe);
        let areas: Vec<Id> = self
            .scrolling
            .previous_order
            .iter()
            .rev()
            .copied()
            .collect();
        for id in areas {
            let Some(state) = self.scrolling.states.get(&id) else {
                continue;
            };
            if !state.enabled || state.clip.is_empty() || window.is_some_and(|w| state.window != w)
            {
                continue;
            }
            let clip = state.clip;
            let reach = Rect::from_min_max(
                clip.min - Vec2::splat(resolved.edge),
                clip.max + Vec2::splat(resolved.edge),
            );
            if !reach.contains(probe) {
                continue;
            }
            let mut velocity = Vec2::ZERO;
            for axis in 0..2 {
                if state.axes[axis] {
                    velocity[axis] = axis_speed(
                        probe[axis],
                        clip.min[axis],
                        clip.max[axis],
                        resolved.edge,
                        resolved.min_speed,
                        resolved.max_speed,
                    );
                }
            }
            // The innermost container that can still move takes the gesture;
            // one stuck at its limit hands it to the container around it.
            if velocity == Vec2::ZERO || !can_move(state, velocity) {
                continue;
            }
            self.scroll_container(id, velocity * dt);
            let state = &self.scrolling.states[&id];
            self.drag.scrolling = can_move(state, velocity);
            return;
        }
    }

    fn scroll_container(&mut self, id: Id, delta: Vec2) {
        let Some(state) = self.scrolling.states.get_mut(&id) else {
            return;
        };
        let old = state.offset;
        state.offset = (old + delta / state.visual_scale).clamp(Vec2::ZERO, state.max_offset());
        let window = state.window;
        if state.offset != old {
            self.invalidate_scroll_hits(window);
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::axis_speed;

    #[test]
    fn speed_grows_toward_the_edge_and_stays_zero_in_the_middle() {
        let speeds: Vec<f32> = [98.0, 90.0, 80.0, 70.0, 50.0]
            .into_iter()
            .map(|p| axis_speed(p, 0.0, 100.0, 32.0, 80.0, 900.0))
            .collect();
        assert_eq!(speeds[4], 0.0);
        assert!(speeds[0] > speeds[1] && speeds[1] > speeds[2] && speeds[2] > speeds[3]);
        assert!(speeds[3] > 0.0 && speeds[0] <= 900.0);
        assert!(axis_speed(2.0, 0.0, 100.0, 32.0, 80.0, 900.0) < -700.0);
        assert!(axis_speed(-50.0, 0.0, 100.0, 32.0, 80.0, 900.0) <= -900.0 + 1e-3);
    }
}
