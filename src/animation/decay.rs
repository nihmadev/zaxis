use super::{Animation, AnimationSample, SpringValue};
use std::time::Duration;

/// Exponential friction: velocity falls as `e^(-friction * t)`, so the value
/// glides toward a rest position it never quite reaches. The track completes
/// once the remaining distance drops under `distance_threshold`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecayOptions {
    /// Rate of velocity loss per second. Larger stops sooner and shorter.
    pub friction: f64,
    pub distance_threshold: f64,
}
impl Default for DecayOptions {
    fn default() -> Self {
        Self::friction(4.0)
    }
}
impl DecayOptions {
    pub fn friction(friction: f64) -> Self {
        assert!(
            friction.is_finite() && friction > 0.0,
            "decay friction must be finite and positive"
        );
        Self {
            friction,
            distance_threshold: 0.01,
        }
    }
    /// Time for the speed to halve.
    pub fn half_life(half_life: Duration) -> Self {
        assert!(!half_life.is_zero(), "decay half-life must be positive");
        Self::friction(std::f64::consts::LN_2 / half_life.as_secs_f64())
    }
    pub fn threshold(mut self, distance: f64) -> Self {
        assert!(
            distance.is_finite() && distance > 0.0,
            "decay threshold must be finite and positive"
        );
        self.distance_threshold = distance;
        self
    }
}

/// Momentum after a fling: starts at `from` with `velocity` (units per second)
/// and slows down exponentially. Analytic, so O(1) per sample after any
/// suspension. The final rest position is known up front through [`Decay::rest`],
/// which lets the caller clamp or snap it before starting.
pub struct Decay<T> {
    from: T,
    velocity: T,
    options: DecayOptions,
}
impl<T: SpringValue> Decay<T> {
    pub fn new(from: T, velocity: T) -> Self {
        Self::with_options(from, velocity, DecayOptions::default())
    }
    pub fn with_options(from: T, velocity: T, options: DecayOptions) -> Self {
        assert!(
            from.norm().is_finite() && velocity.norm().is_finite(),
            "decay needs finite values"
        );
        Self {
            from,
            velocity,
            options,
        }
    }
    /// Where the motion comes to rest: `from + velocity / friction`.
    pub fn rest(&self) -> T {
        self.from
            .add(&self.velocity.scale(1.0 / self.options.friction))
    }
    fn settle_seconds(&self) -> f64 {
        let o = self.options;
        let distance = self.velocity.norm() / o.friction;
        if distance <= o.distance_threshold {
            0.0
        } else {
            (distance / o.distance_threshold).ln() / o.friction
        }
    }
}
impl<T: SpringValue> Animation<T> for Decay<T> {
    fn sample(&self, elapsed: Duration) -> AnimationSample<T> {
        let t = elapsed.as_secs_f64();
        if t >= self.settle_seconds() {
            return AnimationSample::completed(self.rest());
        }
        let o = self.options;
        let travelled = (1.0 - (-o.friction * t).exp()) / o.friction;
        AnimationSample::running(self.from.add(&self.velocity.scale(travelled)))
    }
    fn finish(&self) -> T {
        self.rest()
    }
    fn duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(self.settle_seconds()))
    }
}
