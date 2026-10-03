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

pub(crate) struct Animations {
    entries: HashMap<Id, Box<dyn Entry>>,
    observed: Vec<Id>,
}
impl Default for Animations {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            observed: Vec::new(),
        }
    }
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
}

#[derive(Clone, Copy)]
pub(crate) enum Control {
    Pause,
    Resume,
    Cancel,
    Finish,
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
        }
    }
    fn elapsed_at(&self, now: Instant) -> Duration {
        if self.status == AnimationStatus::Running {
            self.elapsed
                .saturating_add(now.saturating_duration_since(self.started))
        } else {
            self.elapsed
        }
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
        let sample = self.track.as_ref().unwrap().sample(self.elapsed_at(now));
        self.sampled_at = Some(now);
        if sample.completed {
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
    ) -> Animated<T> {
        self.evaluate(now, reduced);
        self.last_frame = frame;
        if self.status == AnimationStatus::Running && visible {
            self.continuous = matches!(self.wake, Wake::NextFrame)
                || matches!(self.wake, Wake::After(delay) if delay.is_zero());
            let delay = match self.wake {
                Wake::NextFrame => interval.max(Duration::from_millis(1)),
                Wake::After(delay) if delay.is_zero() => interval.max(Duration::from_millis(1)),
                Wake::After(delay) => delay,
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

impl<T: Clone + 'static> Entry for Slot<T> {
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn begin_pass(&mut self) {
        self.deadline = None;
        self.continuous = false;
    }
    fn last_frame(&self) -> u64 {
        self.last_frame
    }
    fn deadline(&self) -> Option<Instant> {
        self.deadline
    }
    fn wants_frame(&self) -> bool {
        self.continuous
    }
    fn status(&self) -> AnimationStatus {
        self.status
    }
    fn control(&mut self, action: Control, now: Instant) {
        let was_active = self.track.is_some();
        self.evaluate(now, false);
        match action {
            Control::Pause if self.status == AnimationStatus::Running => {
                self.elapsed = self.elapsed_at(now);
                self.status = AnimationStatus::Paused;
            }
            Control::Resume if self.status == AnimationStatus::Paused => {
                self.started = now;
                self.sampled_at = None;
                self.status = AnimationStatus::Running;
            }
            Control::Cancel if was_active => {
                self.track = None;
                self.status = AnimationStatus::Cancelled;
                self.completion = false;
            }
            Control::Finish if self.track.is_some() => {
                self.complete(self.track.as_ref().unwrap().finish())
            }
            _ => {}
        }
        self.deadline = None;
        self.continuous = false;
    }
    fn reduce_motion(&mut self, now: Instant) {
        self.evaluate(now, true);
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
}

impl Animations {
    pub(crate) fn spring<T: super::SpringValue>(
        &mut self,
        id: Id,
        initial: Option<super::SpringState<T>>,
        target: T,
        options: super::SpringOptions,
        pass: Pass,
    ) -> Animated<super::SpringState<T>> {
        self.observed.push(id);
        if !self.entries.contains_key(&id) {
            if initial.is_none() {
                let rest = super::SpringState {
                    value: target.clone(),
                    velocity: T::zero(),
                };
                self.entries.insert(
                    id,
                    Box::new(Slot::new(
                        rest.clone(),
                        Some(rest),
                        None,
                        AnimationOptions::default(),
                        pass.now,
                    )),
                );
            } else {
                let initial = initial.unwrap_or(super::SpringState {
                    value: target.clone(),
                    velocity: T::zero(),
                });
                let track =
                    super::Spring::with_options(initial.value.clone(), target.clone(), options)
                        .velocity(initial.velocity.clone());
                let slot = Slot::new(
                    initial,
                    Some(super::SpringState {
                        value: target.clone(),
                        velocity: T::zero(),
                    }),
                    Some(Box::new(track)),
                    AnimationOptions::default(),
                    pass.now,
                );
                self.entries.insert(id, Box::new(slot));
            }
        }
        let slot = self.slot::<super::SpringState<T>>(id).unwrap();
        if slot
            .target
            .as_ref()
            .is_none_or(|state| state.value != target)
        {
            slot.evaluate(pass.now, pass.reduced);
            let from = slot.value.clone();
            let track = super::Spring::with_options(from.value.clone(), target.clone(), options)
                .velocity(from.velocity.clone());
            *slot = Slot::new(
                from,
                Some(super::SpringState {
                    value: target,
                    velocity: T::zero(),
                }),
                Some(Box::new(track)),
                AnimationOptions::default(),
                pass.now,
            );
        }
        slot.read(
            pass.now,
            pass.frame,
            pass.visible,
            pass.reduced,
            pass.interval,
        )
    }
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
    pub(crate) fn transition<T: Interpolate>(
        &mut self,
        id: Id,
        initial: Option<T>,
        target: T,
        options: TweenOptions,
        pass: Pass,
    ) -> Animated<T> {
        self.observed.push(id);
        if !self.entries.contains_key(&id) {
            let from = initial.unwrap_or_else(|| target.clone());
            let track: Option<Box<dyn Animation<T>>> = (from != target).then(|| {
                Box::new(Tween::with_options(
                    from.clone(),
                    target.clone(),
                    options.clone(),
                )) as _
            });
            self.entries.insert(
                id,
                Box::new(Slot::new(
                    from,
                    Some(target.clone()),
                    track,
                    AnimationOptions::default(),
                    pass.now,
                )),
            );
        }
        let slot = self.slot::<T>(id).unwrap();
        if slot.target.as_ref() != Some(&target) {
            slot.evaluate(pass.now, pass.reduced);
            let from = slot.value.clone();
            *slot = Slot::new(
                from.clone(),
                Some(target.clone()),
                Some(Box::new(Tween::with_options(from, target, options))),
                AnimationOptions::default(),
                pass.now,
            );
        }
        slot.read(
            pass.now,
            pass.frame,
            pass.visible,
            pass.reduced,
            pass.interval,
        )
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
