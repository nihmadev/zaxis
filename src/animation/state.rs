//! Safe type erasure for retained application-defined values and tracks.
use super::{
    Animated, Animation, AnimationOptions, AnimationStatus, Interpolate, Tween, TweenOptions, Wake,
};
use crate::Id;
use std::{
    any::Any,
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Default)]
pub(crate) struct Animations {
    entries: HashMap<Id, Box<dyn Entry>>,
    observed: Vec<Id>,
}

trait Entry {
    fn any_mut(&mut self) -> &mut dyn Any;
    fn begin_pass(&mut self);
    fn last_frame(&self) -> u64;
    fn deadline(&self) -> Option<Instant>;
    fn wants_frame(&self) -> bool;
    fn status(&self) -> AnimationStatus;
    fn control(&mut self, action: Control, now: Instant);
    fn reduce_motion(&mut self, now: Instant);
    fn elapsed(&self, now: Instant) -> Duration;
    fn rate(&self) -> f64;
    fn duration(&self) -> Option<Duration>;
}

#[derive(Clone, Copy)]
pub(crate) enum Control {
    Pause,
    Resume,
    Cancel,
    Finish,
    Seek(Duration),
    Rate(f64),
    Reverse,
}

struct Slot<T> {
    value: T,
    target: Option<T>,
    track: Option<Box<dyn Animation<T>>>,
    options: AnimationOptions,
    status: AnimationStatus,
    started: Instant,
    elapsed: Duration,
    sampled_at: Option<Instant>,
    wake: Wake,
    deadline: Option<Instant>,
    continuous: bool,
    last_frame: u64,
    completion: bool,
    /// Per-channel playback rate; negative plays backwards.
    rate: f64,
    /// Global `MotionStyle::time_scale` last folded into `elapsed`.
    scale: f64,
}

/// Saturating `d * factor` for nonnegative factors.
fn scale_duration(d: Duration, factor: f64) -> Duration {
    let secs = d.as_secs_f64() * factor;
    if secs.is_finite() && secs < 1.0e9 {
        Duration::from_secs_f64(secs.max(0.0))
    } else {
        Duration::MAX
    }
}

impl<T: Clone + 'static> Slot<T> {
    fn new(
        value: T,
        target: Option<T>,
        track: Option<Box<dyn Animation<T>>>,
        options: AnimationOptions,
        now: Instant,
    ) -> Self {
        let status = if track.is_some() {
            AnimationStatus::Running
        } else {
            AnimationStatus::Completed
        };
        Self {
            value,
            target,
            track,
            options,
            status,
            started: now,
            elapsed: Duration::ZERO,
            sampled_at: None,
            wake: Wake::NextFrame,
            deadline: None,
            continuous: false,
            last_frame: 0,
            completion: false,
            rate: 1.0,
            scale: 1.0,
        }
    }
    fn speed(&self) -> f64 {
        self.rate * self.scale
    }
    fn elapsed_at(&self, now: Instant) -> Duration {
        if self.status == AnimationStatus::Running {
            let moved = scale_duration(
                now.saturating_duration_since(self.started),
                self.speed().abs(),
            );
            if self.speed() >= 0.0 {
                self.elapsed.saturating_add(moved)
            } else {
                self.elapsed.saturating_sub(moved)
            }
        } else {
            self.elapsed
        }
    }
    /// Fold the time run so far into `elapsed` before the speed changes.
    fn rebase(&mut self, now: Instant) {
        self.elapsed = self.elapsed_at(now);
        self.started = now;
        self.sampled_at = None;
    }
    fn complete(&mut self, value: T) {
        self.value = value;
        self.track = None;
        self.status = AnimationStatus::Completed;
        self.completion = true;
        self.deadline = None;
        self.continuous = false;
    }
    fn evaluate(&mut self, now: Instant, reduced: bool) {
        if reduced && self.options.decorative {
            if let Some(track) = &self.track {
                self.complete(track.finish());
            }
        }
        if self.status != AnimationStatus::Running || self.sampled_at == Some(now) {
            return;
        }
        let elapsed = self.elapsed_at(now);
        let sample = self.track.as_ref().unwrap().sample(elapsed);
        self.sampled_at = Some(now);
        // Playing backwards ends at the start pose, not at the finish value.
        if sample.completed || (self.speed() < 0.0 && elapsed.is_zero()) {
            self.complete(sample.value);
        } else {
            self.value = sample.value;
            self.wake = sample.wake;
        }
    }
    fn read(
        &mut self,
        now: Instant,
        frame: u64,
        visible: bool,
        reduced: bool,
        interval: Duration,
        scale: f64,
    ) -> Animated<T> {
        if self.scale != scale {
            self.rebase(now);
            self.scale = scale;
        }
        self.evaluate(now, reduced);
        self.last_frame = frame;
        if self.status == AnimationStatus::Running && visible {
            self.continuous = matches!(self.wake, Wake::NextFrame)
                || matches!(self.wake, Wake::After(delay) if delay.is_zero());
            let delay = match self.wake {
                Wake::NextFrame => interval.max(Duration::from_millis(1)),
                Wake::After(delay) if delay.is_zero() => interval.max(Duration::from_millis(1)),
                // Track time to wall time: a slower channel waits longer.
                Wake::After(delay) => scale_duration(delay, 1.0 / self.speed().abs()),
            };
            // A custom zero wake cannot turn an Immediate host into a busy loop.
            self.deadline = now.checked_add(delay);
        }
        Animated {
            value: self.value.clone(),
            status: self.status,
            just_completed: std::mem::take(&mut self.completion),
        }
    }
}

impl<T: super::SpringValue> Slot<T> {
    /// Current rate of change in units per second of wall time, from a short
    /// forward difference of the track. Zero when nothing is moving.
    fn velocity(&self, now: Instant) -> T {
        let Some(track) = self
            .track
            .as_ref()
            .filter(|_| self.status == AnimationStatus::Running)
        else {
            return T::zero();
        };
        let at = self.elapsed_at(now);
        let step = Duration::from_millis(2);
        let (a, b) = (
            track.sample(at).value,
            track.sample(at.saturating_add(step)).value,
        );
        b.sub(&a).scale(self.speed() / step.as_secs_f64())
    }
}

/// Sampling parameters are fixed for the whole Context pass.
#[derive(Clone, Copy)]
pub(crate) struct Pass {
    pub now: Instant,
    pub frame: u64,
    pub visible: bool,
    pub reduced: bool,
    pub interval: Duration,
    pub scale: f64,
}

impl Animations {
    pub(crate) fn remove(&mut self, id: Id) {
        self.entries.remove(&id);
    }
    pub(crate) fn hide(&mut self, id: Id) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.begin_pass();
        }
    }
    pub(crate) fn observation(&self) -> usize {
        self.observed.len()
    }
    pub(crate) fn hide_since(&mut self, start: usize) {
        for id in &self.observed[start..] {
            if let Some(entry) = self.entries.get_mut(id) {
                entry.begin_pass();
            }
        }
    }
    fn slot<T: Clone + 'static>(&mut self, id: Id) -> Option<&mut Slot<T>> {
        self.entries.get_mut(&id).map(|entry| {
            entry
                .any_mut()
                .downcast_mut::<Slot<T>>()
                .expect("animation channel type changed: use a distinct Id or restart_animation")
        })
    }
    pub(crate) fn animate<T: Clone + 'static, A: Animation<T>>(
        &mut self,
        id: Id,
        options: AnimationOptions,
        create: impl FnOnce() -> A,
        pass: Pass,
    ) -> Animated<T> {
        if !self.entries.contains_key(&id) {
            self.restart(id, create(), options, pass);
        }
        self.read(id, pass).unwrap()
    }
    pub(crate) fn restart<T: Clone + 'static, A: Animation<T>>(
        &mut self,
        id: Id,
        animation: A,
        options: AnimationOptions,
        pass: Pass,
    ) {
        // Evaluate exactly once at t=0, including zero-duration completion.
        let sample = if pass.reduced && options.decorative {
            super::AnimationSample::completed(animation.finish())
        } else {
            animation.sample(Duration::ZERO)
        };
        let completed = sample.completed;
        let mut slot = Slot::new(
            sample.value,
            None,
            if completed {
                None
            } else {
                Some(Box::new(animation))
            },
            options,
            pass.now,
        );
        slot.sampled_at = Some(pass.now);
        slot.wake = sample.wake;
        slot.completion = completed;
        // A restarted channel must still be read each pass to be considered visible.
        self.entries.insert(id, Box::new(slot));
    }
    pub(crate) fn read<T: Clone + 'static>(&mut self, id: Id, pass: Pass) -> Option<Animated<T>> {
        self.observed.push(id);
        self.slot(id).map(|slot| {
            slot.read(
                pass.now,
                pass.frame,
                pass.visible,
                pass.reduced,
                pass.interval,
                pass.scale,
            )
        })
    }
    pub(crate) fn control(&mut self, id: Id, action: Control, now: Instant) -> bool {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.control(action, now);
            true
        } else {
            false
        }
    }
    pub(crate) fn elapsed(&self, id: Id, now: Instant) -> Option<Duration> {
        self.entries.get(&id).map(|entry| entry.elapsed(now))
    }
    pub(crate) fn rate(&self, id: Id) -> Option<f64> {
        self.entries.get(&id).map(|entry| entry.rate())
    }
    pub(crate) fn duration(&self, id: Id) -> Option<Duration> {
        self.entries.get(&id).and_then(|entry| entry.duration())
    }
    pub(crate) fn status(&self, id: Id) -> Option<AnimationStatus> {
        self.entries.get(&id).map(|entry| entry.status())
    }
    pub(crate) fn begin_pass(&mut self) {
        self.observed.clear();
        for entry in self.entries.values_mut() {
            entry.begin_pass();
        }
    }
    pub(crate) fn finish_pass(&mut self, frame: u64) {
        self.entries.retain(|_, entry| entry.last_frame() == frame);
    }
    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.entries
            .values()
            .filter_map(|entry| entry.deadline())
            .min()
    }
    pub(crate) fn wants_frame(&self) -> bool {
        self.entries.values().any(|entry| entry.wants_frame())
    }
    pub(crate) fn reduce_motion(&mut self, now: Instant) {
        for entry in self.entries.values_mut() {
            entry.reduce_motion(now);
        }
    }
}

mod entry;
mod transitions;
