//! Pure gesture mathematics: rubber band, fling decision, velocity, page distance.
use crate::time::Instant;
use crate::{Decay, DecayOptions};
use std::time::Duration;

/// Largest displayed overscroll, in pages.
const RUBBER_LIMIT: f32 = 0.8;
/// Pointer samples older than this do not contribute to the release velocity.
const VELOCITY_WINDOW: Duration = Duration::from_millis(100);
/// A pointer that rested this long before release has no momentum.
const REST: Duration = Duration::from_millis(80);

/// Resistance of an overscroll of `excess` pages (sign kept), `coef` is the initial slope.
pub(crate) fn rubber(excess: f32, coef: f32) -> f32 {
    if excess == 0.0 || coef <= 0.0 {
        return 0.0;
    }
    let magnitude = RUBBER_LIMIT * (1.0 - 1.0 / (excess.abs() * coef / RUBBER_LIMIT + 1.0));
    magnitude.copysign(excess)
}

/// Position shown for a raw drag position; `last` is the final page of a non-looping carousel.
pub(crate) fn displayed(raw: f32, last: Option<usize>, coef: f32) -> f32 {
    match last {
        None => raw,
        Some(last) => {
            let inside = raw.clamp(0.0, last as f32);
            inside + rubber(raw - inside, coef)
        }
    }
}

/// Inverse of [`displayed`] for a position that may be inside the rubber zone.
pub(crate) fn raw_from_displayed(position: f32, last: Option<usize>, coef: f32) -> f32 {
    let Some(last) = last else { return position };
    let inside = position.clamp(0.0, last as f32);
    let shown = position - inside;
    if shown == 0.0 || coef <= 0.0 {
        return inside;
    }
    let ratio = (shown.abs() / RUBBER_LIMIT).min(0.999);
    let excess = RUBBER_LIMIT / coef * (1.0 / (1.0 - ratio) - 1.0);
    inside + excess.copysign(shown)
}

/// Page the carousel settles on after a release: the momentum of the gesture is projected
/// with the engine's [`Decay`] and a projection of at least `commit` pages from `start`
/// moves exactly one page, so a flick never skips pages.
pub(crate) fn settle(
    position: f32,
    velocity: f32,
    start: i64,
    last: Option<usize>,
    friction: f64,
    commit: f32,
) -> i64 {
    let options = DecayOptions::friction(friction.clamp(0.1, 100.0));
    let rest = Decay::with_options(f64::from(position), f64::from(velocity), options).rest();
    let step = rest as f32 - start as f32;
    let delta = if step >= commit {
        1
    } else if step <= -commit {
        -1
    } else {
        0
    };
    let target = start + delta;
    match last {
        Some(last) => target.clamp(0, last as i64),
        None => target,
    }
}

/// Signed number of pages to go from page `from` to page `to`: the shortest way round
/// when `wrap`, the plain difference otherwise.
pub(crate) fn distance(from: usize, to: usize, count: usize, wrap: bool) -> i64 {
    let plain = to as i64 - from as i64;
    if !wrap || count < 2 {
        return plain;
    }
    let n = count as i64;
    let forward = plain.rem_euclid(n);
    if forward * 2 <= n {
        forward
    } else {
        forward - n
    }
}

/// Recent pointer positions (in pages) for the release velocity.
#[derive(Clone, Debug, Default)]
pub struct Velocity {
    samples: Vec<(Instant, f32)>,
}
impl Velocity {
    pub(crate) fn clear(&mut self) {
        self.samples.clear();
    }
    pub(crate) fn push(&mut self, now: Instant, position: f32) {
        self.samples.push((now, position));
        self.samples
            .retain(|(time, _)| now.saturating_duration_since(*time) <= VELOCITY_WINDOW);
    }
    /// Pages per second at `now`; zero after a pause or with a single sample.
    pub(crate) fn pages_per_second(&self, now: Instant) -> f32 {
        let (Some(first), Some(last)) = (self.samples.first(), self.samples.last()) else {
            return 0.0;
        };
        if now.saturating_duration_since(last.0) > REST {
            return 0.0;
        }
        let seconds = last.0.saturating_duration_since(first.0).as_secs_f32();
        if seconds < 1.0e-3 {
            return 0.0;
        }
        let v = (last.1 - first.1) / seconds;
        if v.is_finite() {
            v
        } else {
            0.0
        }
    }
}
