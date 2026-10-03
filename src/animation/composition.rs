use super::{Animation, AnimationSample, Wake};
use crate::Id;
use std::time::Duration;

/// A sleeping stage: schedules one future wakeup, never continuous frames.
pub struct Delay<T> {
    value: T,
    duration: Duration,
}
impl<T> Delay<T> {
    pub fn new(value: T, duration: Duration) -> Self {
        Self { value, duration }
    }
}
impl<T: Clone + 'static> Animation<T> for Delay<T> {
    fn sample(&self, elapsed: Duration) -> AnimationSample<T> {
        if elapsed >= self.duration {
            AnimationSample::completed(self.value.clone())
        } else {
            AnimationSample::after(self.value.clone(), self.duration - elapsed)
        }
    }
    fn finish(&self) -> T {
        self.value.clone()
    }
    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
}

struct Stage<T> {
    track: Box<dyn Animation<T>>,
    duration: Option<Duration>,
}
/// Sequential values. A stage with unknown duration blocks subsequent stages;
/// `then_for` bounds custom/infinite tracks. Boundaries clone the exact finish.
pub struct Sequence<T> {
    initial: T,
    stages: Vec<Stage<T>>,
}
impl<T: Clone + 'static> Sequence<T> {
    pub fn new(initial: T) -> Self {
        Self {
            initial,
            stages: Vec::new(),
        }
    }
    pub fn then(mut self, track: impl Animation<T>) -> Self {
        let duration = track.duration();
        self.stages.push(Stage {
            track: Box::new(track),
            duration,
        });
        self
    }
    pub fn then_for(mut self, duration: Duration, track: impl Animation<T>) -> Self {
        self.stages.push(Stage {
            track: Box::new(track),
            duration: Some(duration),
        });
        self
    }
    pub fn delay(self, value: T, duration: Duration) -> Self {
        self.then(Delay::new(value, duration))
    }
}
impl<T: Clone + 'static> Animation<T> for Sequence<T> {
    fn sample(&self, elapsed: Duration) -> AnimationSample<T> {
        let mut remaining = elapsed;
        for (n, stage) in self.stages.iter().enumerate() {
            match stage.duration {
                Some(duration) if remaining >= duration => {
                    remaining -= duration;
                    if !duration.is_zero() && remaining.is_zero() && n + 1 < self.stages.len() {
                        // Exact endpoint at the boundary; wake policy comes from next stage.
                        let next = self.stages[n + 1].track.sample(Duration::ZERO);
                        return AnimationSample {
                            value: stage.track.finish(),
                            completed: false,
                            wake: next.wake,
                        };
                    }
                }
                _ => {
                    let mut sample = stage.track.sample(remaining);
                    if let Some(duration) = stage.duration {
                        let left = duration - remaining;
                        sample.wake = if sample.completed {
                            Wake::After(left)
                        } else {
                            match sample.wake {
                                Wake::After(delay) => Wake::After(delay.min(left)),
                                wake => wake,
                            }
                        };
                        sample.completed = false;
                    } else if sample.completed && n + 1 < self.stages.len() {
                        // Unknown length has no well-defined residual time. Hold
                        // without continuous frames; `then_for` opts into a bound.
                        sample.completed = false;
                        sample.wake = Wake::After(Duration::MAX);
                    }
                    return sample;
                }
            }
        }
        AnimationSample::completed(self.finish())
    }
    fn finish(&self) -> T {
        self.stages
            .last()
            .map_or_else(|| self.initial.clone(), |s| s.track.finish())
    }
    fn duration(&self) -> Option<Duration> {
        self.stages.iter().try_fold(Duration::ZERO, |sum, stage| {
            Some(sum.saturating_add(stage.duration?))
        })
    }
}

/// Values in child order. Completes only when all children complete (empty is
/// immediately complete); an infinite child keeps the composition running.
pub struct Parallel<T> {
    tracks: Vec<Box<dyn Animation<T>>>,
}
impl<T: Clone + 'static> Default for Parallel<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Clone + 'static> Parallel<T> {
    pub fn new() -> Self {
        Self { tracks: Vec::new() }
    }
    pub fn with(mut self, track: impl Animation<T>) -> Self {
        self.tracks.push(Box::new(track));
        self
    }
}
fn combine_wake(a: Option<Wake>, b: Wake) -> Option<Wake> {
    Some(match (a, b) {
        (Some(Wake::NextFrame), _) | (_, Wake::NextFrame) => Wake::NextFrame,
        (Some(Wake::After(a)), Wake::After(b)) => Wake::After(a.min(b)),
        (None, b) => b,
    })
}
impl<T: Clone + 'static> Animation<Vec<T>> for Parallel<T> {
    fn sample(&self, elapsed: Duration) -> AnimationSample<Vec<T>> {
        let mut wake = None;
        let value = self
            .tracks
            .iter()
            .map(|track| {
                let sample = track.sample(elapsed);
                if !sample.completed {
                    wake = combine_wake(wake, sample.wake);
                }
                sample.value
            })
            .collect();
        AnimationSample {
            value,
            completed: wake.is_none(),
            wake: wake.unwrap_or(Wake::NextFrame),
        }
    }
    fn finish(&self) -> Vec<T> {
        self.tracks.iter().map(|track| track.finish()).collect()
    }
    fn duration(&self) -> Option<Duration> {
        self.tracks
            .iter()
            .try_fold(Duration::ZERO, |max, t| Some(max.max(t.duration()?)))
    }
}

/// Open composition with identity-based schedules. Reordering never reschedules
/// existing children. Insert at composition elapsed time; new delays are capped.
/// Remove only on model deletion, not when a virtual row leaves the viewport.
pub struct Stagger<T> {
    children: Vec<(Id, Duration, Box<dyn Animation<T>>)>,
    step: Duration,
    cap: Duration,
}
impl<T: Clone + 'static> Stagger<T> {
    pub fn new(step: Duration) -> Self {
        Self {
            children: Vec::new(),
            step,
            cap: Duration::from_millis(240),
        }
    }
    pub fn max_delay(mut self, cap: Duration) -> Self {
        self.cap = cap;
        self
    }
    pub fn insert(&mut self, id: Id, at: Duration, track: impl Animation<T>) {
        if self.children.iter().any(|(key, _, _)| *key == id) {
            return;
        }
        let delay = self
            .step
            .saturating_mul(self.children.len().min(u32::MAX as usize) as u32)
            .min(self.cap);
        self.children
            .push((id, at.saturating_add(delay), Box::new(track)));
    }
    pub fn remove(&mut self, id: Id) {
        self.children.retain(|(key, _, _)| *key != id);
    }
    pub fn reorder(&mut self, ids: &[Id]) {
        assert!(ids.len() == self.children.len());
        let mut unique = std::collections::HashSet::new();
        assert!(ids
            .iter()
            .all(|id| unique.insert(*id) && self.children.iter().any(|(key, _, _)| key == id)));
        self.children
            .sort_by_key(|(id, _, _)| ids.iter().position(|key| key == id).unwrap());
    }
}
impl<T: Clone + 'static> Animation<Vec<(Id, T)>> for Stagger<T> {
    fn sample(&self, elapsed: Duration) -> AnimationSample<Vec<(Id, T)>> {
        let mut wake = None;
        let value = self
            .children
            .iter()
            .map(|(id, start, track)| {
                let sample = if elapsed < *start {
                    AnimationSample::after(track.sample(Duration::ZERO).value, *start - elapsed)
                } else {
                    track.sample(elapsed - *start)
                };
                if !sample.completed {
                    wake = combine_wake(wake, sample.wake);
                }
                (*id, sample.value)
            })
            .collect();
        AnimationSample {
            value,
            completed: wake.is_none(),
            wake: wake.unwrap_or(Wake::NextFrame),
        }
    }
    fn finish(&self) -> Vec<(Id, T)> {
        self.children
            .iter()
            .map(|(id, _, track)| (*id, track.finish()))
            .collect()
    }
    fn duration(&self) -> Option<Duration> {
        self.children
            .iter()
            .try_fold(Duration::ZERO, |max, (_, start, track)| {
                Some(max.max(start.saturating_add(track.duration()?)))
            })
    }
}
