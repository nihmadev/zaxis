use super::{Animation, AnimationSample, Easing, Interpolate};
use std::time::Duration;

/// Number of complete cycles, including the first. Auto-reverse adds a return
/// leg to each cycle. Delay occurs only before the first cycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    Once,
    Count(u32),
    Forever,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TweenOptions {
    pub duration: Duration,
    pub delay: Duration,
    pub easing: Easing,
    pub repeat: Repeat,
    pub auto_reverse: bool,
}

impl TweenOptions {
    pub fn total_duration(&self) -> Option<Duration> {
        if self.duration.is_zero() {
            return Some(self.delay);
        }
        let count = match self.repeat {
            Repeat::Once => 1,
            Repeat::Count(n) => n,
            Repeat::Forever => return None,
        };
        Some(
            self.delay.saturating_add(
                self.duration
                    .saturating_mul(count)
                    .saturating_mul(if self.auto_reverse { 2 } else { 1 }),
            ),
        )
    }
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            delay: Duration::ZERO,
            easing: Easing::Linear,
            repeat: Repeat::Once,
            auto_reverse: false,
        }
    }
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        assert!(repeat != Repeat::Count(0), "repeat count must be positive");
        self.repeat = repeat;
        self
    }
    pub fn auto_reverse(mut self, enabled: bool) -> Self {
        self.auto_reverse = enabled;
        self
    }

    fn phase(&self, elapsed: Duration) -> Phase {
        assert!(
            self.repeat != Repeat::Count(0),
            "repeat count must be positive"
        );
        if elapsed < self.delay {
            return Phase::Delay(self.delay - elapsed);
        }
        if self.duration.is_zero() {
            return Phase::Finished;
        }
        let elapsed = (elapsed - self.delay).as_nanos();
        let leg = self.duration.as_nanos();
        let cycle = leg * if self.auto_reverse { 2 } else { 1 };
        let count = match self.repeat {
            Repeat::Once => Some(1),
            Repeat::Count(n) => Some(n),
            Repeat::Forever => None,
        };
        if count.is_some_and(|n| elapsed >= cycle * u128::from(n)) {
            return Phase::Finished;
        }
        let position = elapsed % cycle;
        let position = if self.auto_reverse && position > leg {
            cycle - position
        } else {
            position
        };
        Phase::Active(Duration::new(
            (position / 1_000_000_000) as u64,
            (position % 1_000_000_000) as u32,
        ))
    }
}

enum Phase {
    Delay(Duration),
    Active(Duration),
    Finished,
}

/// An explicit tween. A zero duration completes at the end of delay, including
/// `Forever`. Completion clones the exact endpoint (the start for auto-reverse).
pub struct Tween<T> {
    from: T,
    to: T,
    options: TweenOptions,
}

impl<T> Tween<T> {
    pub fn new(from: T, to: T, duration: Duration) -> Self {
        Self::with_options(from, to, TweenOptions::new(duration))
    }
    pub fn with_options(from: T, to: T, options: TweenOptions) -> Self {
        Self { from, to, options }
    }
    pub fn easing(mut self, easing: Easing) -> Self {
        self.options.easing = easing;
        self
    }
    pub fn delay(mut self, delay: Duration) -> Self {
        self.options.delay = delay;
        self
    }
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.options = self.options.repeat(repeat);
        self
    }
    pub fn auto_reverse(mut self, enabled: bool) -> Self {
        self.options.auto_reverse = enabled;
        self
    }
}

impl<T: Interpolate> Animation<T> for Tween<T> {
    fn duration(&self) -> Option<Duration> {
        self.options.total_duration()
    }
    fn sample(&self, elapsed: Duration) -> AnimationSample<T> {
        match self.options.phase(elapsed) {
            Phase::Delay(delay) => AnimationSample::after(self.from.clone(), delay),
            Phase::Finished => AnimationSample::completed(self.finish()),
            Phase::Active(position) => {
                let value = if position.is_zero() {
                    self.from.clone()
                } else if position == self.options.duration {
                    self.to.clone()
                } else {
                    self.from.interpolate(
                        &self.to,
                        self.options.easing.sample(
                            (position.as_secs_f64() / self.options.duration.as_secs_f64()) as f32,
                        ),
                    )
                };
                AnimationSample::running(value)
            }
        }
    }
    fn finish(&self) -> T {
        if self.options.auto_reverse {
            self.from.clone()
        } else {
            self.to.clone()
        }
    }
}

/// A timed point. Its easing controls the segment arriving at this point.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe<T> {
    pub at: Duration,
    pub value: T,
    pub easing: Easing,
}
impl<T> Keyframe<T> {
    pub fn new(at: Duration, value: T) -> Self {
        Self {
            at,
            value,
            easing: Easing::Linear,
        }
    }
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

/// At least two strictly increasing points, starting at zero. Invalid input
/// panics at construction. Delay/repetition use the same rules as a tween.
pub struct Keyframes<T> {
    points: Vec<Keyframe<T>>,
    options: TweenOptions,
}
impl<T> Keyframes<T> {
    pub fn new(points: impl IntoIterator<Item = Keyframe<T>>) -> Self {
        let points: Vec<_> = points.into_iter().collect();
        assert!(
            points.len() >= 2 && points[0].at.is_zero(),
            "keyframes need at least two points and must start at zero"
        );
        assert!(
            points.windows(2).all(|p| p[0].at < p[1].at),
            "keyframe times must strictly increase"
        );
        let options = TweenOptions::new(points.last().unwrap().at);
        Self { points, options }
    }
    pub fn delay(mut self, delay: Duration) -> Self {
        self.options.delay = delay;
        self
    }
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.options = self.options.repeat(repeat);
        self
    }
    pub fn auto_reverse(mut self, enabled: bool) -> Self {
        self.options.auto_reverse = enabled;
        self
    }
}
impl<T: Interpolate> Animation<T> for Keyframes<T> {
    fn duration(&self) -> Option<Duration> {
        self.options.total_duration()
    }
    fn sample(&self, elapsed: Duration) -> AnimationSample<T> {
        match self.options.phase(elapsed) {
            Phase::Delay(delay) => AnimationSample::after(self.points[0].value.clone(), delay),
            Phase::Finished => AnimationSample::completed(self.finish()),
            Phase::Active(position) => {
                let end = self.points.partition_point(|p| p.at < position);
                let value = if self.points[end].at == position {
                    self.points[end].value.clone()
                } else {
                    let a = &self.points[end - 1];
                    let b = &self.points[end];
                    a.value.interpolate(
                        &b.value,
                        b.easing.sample(
                            ((position - a.at).as_secs_f64() / (b.at - a.at).as_secs_f64()) as f32,
                        ),
                    )
                };
                AnimationSample::running(value)
            }
        }
    }
    fn finish(&self) -> T {
        if self.options.auto_reverse {
            self.points[0].value.clone()
        } else {
            self.points.last().unwrap().value.clone()
        }
    }
}
