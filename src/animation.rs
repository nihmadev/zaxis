//! Time-based motion with retained channels, extensible curves and value types.
//!
//! Call [`crate::Context::transition`] every visible UI pass with a stable property
//! ID. The first call snaps to the target; later target changes tween from the
//! currently displayed value. Use [`crate::Context::animate`] for explicit tracks.
//! See the animation guide and `custom_animation` example for external types.

mod composition;
mod curves;
mod decay;
mod easing;
mod effects;
mod hermite;
mod interpolate;
mod path;
mod spring;
pub(crate) mod state;
pub mod testing;
mod timeline;

pub use composition::{Delay, Parallel, Sequence, Stagger};
pub use decay::{Decay, DecayOptions};
pub use easing::Easing;
pub use effects::{Pulse, Rotation};
pub use interpolate::{Interpolate, Oklab};
pub use path::{Path, PathFollow, PathPose};
pub use spring::{Spring, SpringOptions, SpringState, SpringValue};
pub use timeline::{Keyframe, Keyframes, Repeat, Tween, TweenOptions};

use std::time::Duration;

/// A pure function of elapsed monotonic time. No registration is required.
/// `sample` may be called at arbitrary times, including after a long suspension.
/// `finish` supplies the exact final state for reduced motion or explicit finish.
pub trait Animation<T>: 'static {
    fn sample(&self, elapsed: Duration) -> AnimationSample<T>;
    fn finish(&self) -> T;
    /// Exact timeline length, when known. `None` is unbounded/unspecified.
    /// Compositions can explicitly bound a custom track with `then_for`.
    fn duration(&self) -> Option<Duration> {
        None
    }
}

/// When a visible, unpaused track needs another sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake {
    /// Use `Style::motion.frame_interval` (clamped to at least one millisecond).
    NextFrame,
    /// Sleep this long from the current sample; useful for delay/procedural timers.
    After(Duration),
}

/// A custom track's value and scheduling request at a particular elapsed time.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationSample<T> {
    pub value: T,
    pub completed: bool,
    pub wake: Wake,
}

impl<T> AnimationSample<T> {
    pub fn running(value: T) -> Self {
        Self {
            value,
            completed: false,
            wake: Wake::NextFrame,
        }
    }
    pub fn completed(value: T) -> Self {
        Self {
            value,
            completed: true,
            wake: Wake::NextFrame,
        }
    }
    pub fn after(value: T, delay: Duration) -> Self {
        Self {
            value,
            completed: false,
            wake: Wake::After(delay),
        }
    }
}

/// Adapt an application closure into a track, with an explicit final value.
pub struct Procedural<T, F> {
    final_value: T,
    sample: F,
}

impl<T, F> Procedural<T, F> {
    pub fn new(final_value: T, sample: F) -> Self {
        Self {
            final_value,
            sample,
        }
    }
}

impl<T: Clone + 'static, F: Fn(Duration) -> AnimationSample<T> + 'static> Animation<T>
    for Procedural<T, F>
{
    fn sample(&self, elapsed: Duration) -> AnimationSample<T> {
        (self.sample)(elapsed)
    }
    fn finish(&self) -> T {
        self.final_value.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationStatus {
    Running,
    Paused,
    Completed,
    Cancelled,
}

/// The retained channel's result. `just_completed` is consumed by the first read,
/// even if a channel is read multiple times in the same UI pass.
#[derive(Clone, Debug, PartialEq)]
pub struct Animated<T> {
    pub value: T,
    pub status: AnimationStatus,
    pub just_completed: bool,
}

impl<T> Animated<T> {
    pub fn running(&self) -> bool {
        self.status == AnimationStatus::Running
    }
    pub fn completed(&self) -> bool {
        self.status == AnimationStatus::Completed
    }
}

/// Decorative tracks respect the global reduced-motion switch by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationOptions {
    pub decorative: bool,
}

impl Default for AnimationOptions {
    fn default() -> Self {
        Self { decorative: true }
    }
}

/// Built-in transition settings, also used by the public animation API.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionStyle {
    pub reduced_motion: bool,
    pub hover: TweenOptions,
    pub expand: TweenOptions,
    pub page: TweenOptions,
    /// Timer cadence for Immediate/custom hosts. The built-in Vsync runner uses
    /// presentation cadence for continuous tracks. Progress is time-based.
    pub frame_interval: Duration,
    pub spring: SpringOptions,
    pub presence: TweenOptions,
    pub reorder: TweenOptions,
    pub highlight: TweenOptions,
    pub slide_distance: f32,
    pub loader_size: f32,
    pub loader_stroke: f32,
    pub cycle_period: Duration,
    pub indicator_thickness: f32,
    /// Global speed of every animation channel: 0.25 for slow motion while
    /// debugging, 2.0 for double speed. Non-positive or non-finite values mean 1.
    pub time_scale: f32,
}

impl Default for MotionStyle {
    fn default() -> Self {
        Self {
            reduced_motion: false,
            hover: TweenOptions::new(Duration::from_millis(160)).easing(Easing::QuadOut),
            expand: TweenOptions::new(Duration::from_millis(160)).easing(Easing::QuadOut),
            page: TweenOptions::new(Duration::from_millis(280)).easing(Easing::QuintOut),
            frame_interval: Duration::from_millis(16),
            spring: SpringOptions::default(),
            presence: TweenOptions::new(Duration::from_millis(240)).easing(Easing::CubicOut),
            reorder: TweenOptions::new(Duration::from_millis(200)).easing(Easing::CubicOut),
            highlight: TweenOptions::new(Duration::from_millis(500)).easing(Easing::SineOut),
            slide_distance: 8.0,
            loader_size: 24.0,
            loader_stroke: 2.0,
            cycle_period: Duration::from_millis(1200),
            indicator_thickness: 2.0,
            time_scale: 1.0,
        }
    }
}
