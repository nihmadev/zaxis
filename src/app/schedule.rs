//! Repaint scheduling for one window and for all windows together.

use std::time::Instant;
use winit::event_loop::ControlFlow;

pub(super) fn repaint_schedule(
    now: Instant,
    visible: bool,
    needs_repaint: bool,
    next_repaint: Option<Instant>,
    retry_at: Option<Instant>,
) -> (bool, ControlFlow) {
    if !visible {
        return (false, ControlFlow::Wait);
    }
    // Back off transient surface failures even when the UI requests another frame.
    let redraw = retry_at.map_or(needs_repaint, |deadline| deadline <= now);
    let deadline = retry_at
        .or(next_repaint)
        .filter(|time| !redraw && *time > now);
    (
        redraw,
        deadline.map_or(ControlFlow::Wait, ControlFlow::WaitUntil),
    )
}

/// The earlier of two wake-ups; `Wait` means "no deadline".
pub(super) fn earliest(a: ControlFlow, b: ControlFlow) -> ControlFlow {
    match (a, b) {
        (ControlFlow::WaitUntil(x), ControlFlow::WaitUntil(y)) => ControlFlow::WaitUntil(x.min(y)),
        (ControlFlow::WaitUntil(x), _) | (_, ControlFlow::WaitUntil(x)) => {
            ControlFlow::WaitUntil(x)
        }
        _ => ControlFlow::Wait,
    }
}

/// Everything the scheduler needs to know about one window.
#[derive(Clone, Copy, Debug)]
pub(super) struct WindowTiming {
    pub visible: bool,
    pub needs_repaint: bool,
    pub next_repaint: Option<Instant>,
    pub retry_at: Option<Instant>,
}

/// The windows to redraw now and the single control flow that wakes the loop for the nearest
/// deadline of any window. Each window is judged on its own timing, so an animation in one
/// window never asks another to redraw, and windows at rest contribute no wake-up at all.
pub(super) fn schedule_all<K>(
    now: Instant,
    windows: impl IntoIterator<Item = (K, WindowTiming)>,
) -> (Vec<K>, ControlFlow) {
    let mut redraw = Vec::new();
    let mut flow = ControlFlow::Wait;
    for (key, timing) in windows {
        let (wants, window_flow) = repaint_schedule(
            now,
            timing.visible,
            timing.needs_repaint,
            timing.next_repaint,
            timing.retry_at,
        );
        if wants {
            redraw.push(key);
        }
        flow = earliest(flow, window_flow);
    }
    (redraw, flow)
}
