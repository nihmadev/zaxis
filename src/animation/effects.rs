use super::{Animation, AnimationSample};
use std::time::Duration;

/// Constant angular speed in radians. Modulo changes the numeric angle, not the
/// displayed pose at a revolution boundary. Zero period is a static indicator.
pub struct Rotation {
    pub period: Duration,
}
impl Rotation {
    pub fn new(period: Duration) -> Self {
        Self { period }
    }
}
impl Animation<f32> for Rotation {
    fn sample(&self, elapsed: Duration) -> AnimationSample<f32> {
        if self.period.is_zero() {
            return AnimationSample::completed(0.0);
        }
        AnimationSample::running(
            ((elapsed.as_nanos() % self.period.as_nanos()) as f64 / self.period.as_nanos() as f64
                * std::f64::consts::TAU) as f32,
        )
    }
    fn finish(&self) -> f32 {
        0.0
    }
}
/// Quiet sinusoidal alpha. No text movement. Reduced motion uses full opacity.
pub struct Pulse {
    pub period: Duration,
    pub minimum: f32,
}
impl Pulse {
    pub fn new(period: Duration) -> Self {
        Self {
            period,
            minimum: 0.65,
        }
    }
    pub fn minimum(mut self, minimum: f32) -> Self {
        assert!(minimum.is_finite() && (0.0..=1.0).contains(&minimum));
        self.minimum = minimum;
        self
    }
}
impl Animation<f32> for Pulse {
    fn sample(&self, elapsed: Duration) -> AnimationSample<f32> {
        if self.period.is_zero() {
            return AnimationSample::completed(1.0);
        }
        let phase =
            (elapsed.as_nanos() % self.period.as_nanos()) as f64 / self.period.as_nanos() as f64;
        AnimationSample::running(
            self.minimum
                + (1.0 - self.minimum) * (0.5 + 0.5 * (phase * std::f64::consts::TAU).cos()) as f32,
        )
    }
    fn finish(&self) -> f32 {
        1.0
    }
}
