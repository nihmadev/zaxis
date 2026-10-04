use super::{Animation, AnimationSample, SpringValue};
use std::time::Duration;

/// Cubic Hermite leg that starts at `from` with a given velocity and comes to
/// rest at `to`. Used to retarget a running transition without a velocity jump.
pub(crate) struct Hermite<T> {
    from: T,
    to: T,
    /// Start tangent: velocity (units/s) times the leg length in seconds.
    tangent: T,
    duration: Duration,
}
impl<T: SpringValue> Hermite<T> {
    pub(crate) fn new(from: T, to: T, velocity: T, duration: Duration) -> Self {
        let tangent = velocity.scale(duration.as_secs_f64());
        Self {
            from,
            to,
            tangent,
            duration,
        }
    }
}
impl<T: SpringValue> Animation<T> for Hermite<T> {
    fn sample(&self, elapsed: Duration) -> AnimationSample<T> {
        if elapsed >= self.duration {
            return AnimationSample::completed(self.to.clone());
        }
        if elapsed.is_zero() {
            return AnimationSample::running(self.from.clone());
        }
        let p = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        let (p2, p3) = (p * p, p * p * p);
        let start = 2.0 * p3 - 3.0 * p2 + 1.0;
        let slope = p3 - 2.0 * p2 + p;
        let end = -2.0 * p3 + 3.0 * p2;
        AnimationSample::running(
            self.from
                .scale(start)
                .add(&self.tangent.scale(slope))
                .add(&self.to.scale(end)),
        )
    }
    fn finish(&self) -> T {
        self.to.clone()
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
}
