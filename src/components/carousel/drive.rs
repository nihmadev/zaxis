//! From input to position: gestures, keys, wheel, clicks, autoplay and the spring.
use super::{
    motion::{displayed, raw_from_displayed, settle},
    state::{CarouselState, Drag, Model},
    style::Look,
};
use crate::{components::Ui, Id, Rect, Response, SpringOptions, SpringState};
use std::time::Duration;
use winit::keyboard::KeyCode;

/// What a pass needs to know about the carousel besides its state.
pub(super) struct Params<'a> {
    pub(super) id: Id,
    /// The hit region that receives the wheel.
    pub(super) root: Id,
    pub(super) look: &'a Look,
    pub(super) wrap: bool,
    pub(super) axis: usize,
    /// Pixels of pointer travel along the axis that move the position by one page.
    pub(super) extent: f32,
    /// The whole carousel, for hover and visibility.
    pub(super) rect: Rect,
    pub(super) autoplay: Option<Duration>,
    pub(super) enabled: bool,
    pub(super) wheel: bool,
    pub(super) spring: SpringOptions,
}

#[derive(Default)]
pub(super) struct Driven {
    pub(super) position: f32,
    pub(super) settled: bool,
    pub(super) changed: bool,
    pub(super) autoplayed: bool,
    pub(super) activated: Option<usize>,
}

enum Nav {
    Step(i64),
    First,
    Last,
    /// A one-based page number from assistive technology; out of range goes to the nearest.
    Number(f64),
}

/// Navigation requested by keys while the carousel has focus.
fn keys(ui: &Ui<'_>, axis: usize) -> Vec<Nav> {
    let (back, forward) = if axis == 0 {
        (KeyCode::ArrowLeft, KeyCode::ArrowRight)
    } else {
        (KeyCode::ArrowUp, KeyCode::ArrowDown)
    };
    let pressed = &ui.context.input().keys_pressed;
    [
        (back, Nav::Step(-1)),
        (forward, Nav::Step(1)),
        (KeyCode::PageUp, Nav::Step(-1)),
        (KeyCode::PageDown, Nav::Step(1)),
        (KeyCode::Home, Nav::First),
        (KeyCode::End, Nav::Last),
    ]
    .into_iter()
    .filter(|(code, _)| pressed.contains(code))
    .map(|(_, nav)| nav)
    .collect()
}

/// Whole pages the wheel turned: one per `wheel_step` of travel, then a pause so that
/// trackpad inertia does not run through the pages.
fn wheel(ui: &mut Ui<'_>, p: &Params<'_>, state: &mut CarouselState) -> i64 {
    let delta = ui.context.carousel_wheel_take(p.root);
    let now = ui.context.frame_time();
    if delta == 0.0 || !p.enabled {
        return 0;
    }
    if state.wheel_until.is_some_and(|until| now < until) {
        state.wheel = 0.0;
        return 0;
    }
    if delta.signum() != state.wheel.signum() {
        state.wheel = 0.0;
    }
    state.wheel += delta;
    if state.wheel.abs() < p.look.wheel_step() {
        return 0;
    }
    let step = state.wheel.signum() as i64;
    state.wheel = 0.0;
    state.wheel_until = now.checked_add(p.look.wheel_lock());
    step
}

pub(super) fn go(state: &mut CarouselState, model: &Model<'_>, wrap: bool, nav: i64) -> bool {
    state.go(state.target + nav, model, wrap)
}

fn apply(state: &mut CarouselState, model: &Model<'_>, wrap: bool, nav: Nav) -> bool {
    let n = model.count;
    match nav {
        Nav::Step(step) => go(state, model, wrap, step),
        Nav::First => go(
            state,
            model,
            wrap,
            super::motion::distance(state.page, 0, n, wrap),
        ),
        Nav::Last => go(
            state,
            model,
            wrap,
            super::motion::distance(state.page, n - 1, n, wrap),
        ),
        Nav::Number(number) => {
            let page = (number.round() - 1.0).clamp(0.0, (n - 1) as f64) as usize;
            let step = super::motion::distance(state.page, page, n, wrap);
            go(state, model, wrap, step)
        }
    }
}

/// One pass of input and motion. `response` is the carousel's own region.
pub(super) fn drive(
    ui: &mut Ui<'_>,
    p: &Params<'_>,
    state: &mut CarouselState,
    model: &Model<'_>,
    response: Response,
    shift: f32,
) -> Driven {
    let now = ui.context.frame_time();
    let n = model.count;
    let last = (!p.wrap).then(|| n - 1);
    let coef = p.look.rubber();
    let mut out = Driven::default();
    let channel = p.id.with("position");
    let mut release = None;

    if p.enabled {
        // Swipe: the position follows the pointer, a release decides by distance and speed.
        if response.drag_started() && state.drag.is_none() {
            let raw = raw_from_displayed(state.position, last, coef);
            state.drag = Some(Drag {
                raw,
                start: state.target,
            });
            state.velocity.clear();
        }
        if let Some(drag) = state.drag.as_mut() {
            let travel = response.drag_delta()[p.axis];
            if travel != 0.0 {
                drag.raw -= travel / p.extent.max(1.0);
                state.velocity.push(now, drag.raw);
            }
            if response.drag_stopped() {
                let shown = displayed(drag.raw, last, coef);
                let speed = state.velocity.pages_per_second(now);
                let start = drag.start;
                let target = settle(
                    shown,
                    speed,
                    start,
                    last,
                    p.look.friction(),
                    p.look.commit(),
                );
                state.drag = None;
                out.changed |= state.go(target, model, p.wrap);
                release = Some(SpringState {
                    value: shown,
                    velocity: speed,
                });
            }
        } else {
            // A press-and-release that was not a swipe: keys, wheel and clicks.
            if response.has_focus {
                for nav in keys(ui, p.axis) {
                    out.changed |= apply(state, model, p.wrap, nav);
                }
            }
            // Requests from assistive technology turn pages the way the arrow keys do.
            for request in ui.context.take_access_actions(p.root) {
                let nav = match request {
                    crate::AccessAction::Increment => Nav::Step(1),
                    crate::AccessAction::Decrement => Nav::Step(-1),
                    crate::AccessAction::SetNumericValue(n) if n.is_finite() => Nav::Number(n),
                    _ => continue,
                };
                out.changed |= apply(state, model, p.wrap, nav);
            }
            if p.wheel {
                let steps = wheel(ui, p, state);
                if steps != 0 {
                    out.changed |= go(state, model, p.wrap, steps);
                }
            }
            if response.clicked() {
                let input = ui.context.input();
                let by_pointer = input.primary_released;
                let pointer = input.pointer;
                if by_pointer {
                    let hit = pointer.and_then(|pointer| {
                        state
                            .layers
                            .iter()
                            .rev()
                            .find(|(_, rect)| rect.contains(pointer))
                            .map(|l| l.0)
                    });
                    match hit {
                        Some(k) if k != state.target => {
                            out.changed |= state.go(k, model, p.wrap);
                        }
                        Some(_) => out.activated = Some(state.page),
                        None => {}
                    }
                } else {
                    out.activated = Some(state.page);
                }
            }
        }
    } else {
        state.drag = None;
        ui.context.carousel_wheel_take(p.root);
    }

    // Autoplay waits while anything holds the carousel and restarts its interval afterwards.
    let visible = !p.rect.intersect(ui.clip_rect()).is_empty();
    let reduced = ui.style().motion.reduced_motion;
    let held = response.hovered
        || response.has_focus
        || state.drag.is_some()
        || !visible
        || reduced
        || !p.enabled
        || !state.settled
        || n < 2
        || (!p.wrap && state.page == n - 1);
    match p.autoplay {
        Some(interval) if !held => {
            let due = *state.autoplay_at.get_or_insert(now + interval);
            if now >= due {
                state.autoplay_at = None;
                if go(state, model, p.wrap, 1) {
                    out.changed = true;
                    out.autoplayed = true;
                }
            } else {
                ui.context.request_repaint_after(due - now);
            }
        }
        _ => state.autoplay_at = None,
    }

    // Position: pointer-driven during a swipe, otherwise the spring toward the target.
    let target = state.target as f32;
    let pass = ui.context.animation_pass(visible);
    if let Some(drag) = state.drag {
        ui.context.remove_animation(channel);
        out.position = displayed(drag.raw, last, coef);
        out.settled = false;
    } else {
        let mut initial = release;
        if shift != 0.0 {
            let current = ui.context.sample_animation::<SpringState<f32>>(channel);
            let (value, velocity) =
                current.map_or((state.position, 0.0), |c| (c.value.value, c.value.velocity));
            initial = Some(SpringState {
                value: value + shift,
                velocity,
            });
        }
        if initial.is_some() {
            ui.context.remove_animation(channel);
        }
        let moving = ui
            .context
            .animations
            .spring(channel, initial, target, p.spring, pass);
        out.position = moving.value.value;
        out.settled = moving.completed();
        // A looping carousel counts on past the ends; fold it back once it rests.
        if out.settled && p.wrap && state.target.unsigned_abs() >= 2 * n as u64 {
            let folded = state.target.rem_euclid(n as i64);
            ui.context.remove_animation(channel);
            state.target = folded;
            state.from = folded;
            out.position = folded as f32;
        }
    }
    if out.changed {
        ui.context.request_repaint();
    }
    state.position = out.position;
    if out.settled {
        state.from = state.target;
    }
    state.settled = out.settled;
    out
}
