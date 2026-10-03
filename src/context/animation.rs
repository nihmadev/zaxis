use super::Context;
use crate::{
    animation::{
        state::{Control, Pass},
        Animated, Animation, AnimationOptions, AnimationStatus, Interpolate, TweenOptions,
    },
    Id,
};
use std::time::Instant;

impl Context {
    /// First appearance snaps; retarget preserves sampled position AND velocity.
    pub fn spring_transition<T: crate::SpringValue>(
        &mut self,
        id: Id,
        target: T,
        options: crate::SpringOptions,
    ) -> Animated<crate::SpringState<T>> {
        self.animations
            .spring(id, None, target, options, self.animation_pass(true))
    }
    pub fn spring_transition_from<T: crate::SpringValue>(
        &mut self,
        id: Id,
        initial: crate::SpringState<T>,
        target: T,
        options: crate::SpringOptions,
    ) -> Animated<crate::SpringState<T>> {
        self.animations.spring(
            id,
            Some(initial),
            target,
            options,
            self.animation_pass(true),
        )
    }
    /// Remove a channel now, including its deadline. Reappearance starts fresh.
    pub fn remove_animation(&mut self, id: Id) {
        self.animations.remove(id);
    }
    /// Monotonic time fixed at the start of the current (or latest) UI pass.
    /// Animation controls called outside a pass take effect at this time, too.
    pub fn frame_time(&self) -> Instant {
        self.frame_time
    }

    pub(crate) fn animation_pass(&self, visible: bool) -> Pass {
        Pass {
            now: self.frame_time,
            frame: self.frame,
            visible,
            reduced: self.style.motion.reduced_motion,
            interval: self.style.motion.frame_interval,
        }
    }

    /// First appearance snaps to `target`. Changing the target retweens from the
    /// current value; equal targets (including completed/cancelled) never restart.
    /// Use property channels such as `widget_id.with("color")`. Call only while
    /// visible; Ui helpers additionally check clipping. Options apply on retarget.
    pub fn transition<T: Interpolate>(
        &mut self,
        id: Id,
        target: T,
        options: TweenOptions,
    ) -> Animated<T> {
        self.transition_visible(id, None, target, options, true)
    }
    /// Like `transition`, but the first appearance animates from `initial`.
    pub fn transition_from<T: Interpolate>(
        &mut self,
        id: Id,
        initial: T,
        target: T,
        options: TweenOptions,
    ) -> Animated<T> {
        self.transition_visible(id, Some(initial), target, options, true)
    }
    pub(crate) fn transition_visible<T: Interpolate>(
        &mut self,
        id: Id,
        initial: Option<T>,
        target: T,
        options: TweenOptions,
        visible: bool,
    ) -> Animated<T> {
        self.animations
            .transition(id, initial, target, options, self.animation_pass(visible))
    }

    /// Create once per visible lifetime. The factory is not called again on
    /// subsequent passes, even after completion or cancellation. Restart explicitly.
    pub fn animate<T: Clone + 'static, A: Animation<T>>(
        &mut self,
        id: Id,
        create: impl FnOnce() -> A,
    ) -> Animated<T> {
        self.animate_with(id, AnimationOptions::default(), create)
    }
    pub fn animate_with<T: Clone + 'static, A: Animation<T>>(
        &mut self,
        id: Id,
        options: AnimationOptions,
        create: impl FnOnce() -> A,
    ) -> Animated<T> {
        self.animations
            .animate(id, options, create, self.animation_pass(true))
    }
    /// Replace a channel and reset its clock to zero, clearing pause/cancel/events.
    /// This may deliberately jump to the track's initial value; use transition for
    /// continuity. A subsequent animate/sample call marks the channel visible.
    pub fn restart_animation<T: Clone + 'static>(&mut self, id: Id, animation: impl Animation<T>) {
        self.restart_animation_with(id, animation, AnimationOptions::default());
    }
    pub fn restart_animation_with<T: Clone + 'static>(
        &mut self,
        id: Id,
        animation: impl Animation<T>,
        options: AnimationOptions,
    ) {
        self.animations
            .restart(id, animation, options, self.animation_pass(true));
    }
    /// Read an existing channel and keep it alive for this visible pass.
    pub fn sample_animation<T: Clone + 'static>(&mut self, id: Id) -> Option<Animated<T>> {
        self.animations.read(id, self.animation_pass(true))
    }
    pub fn animation_status(&self, id: Id) -> Option<AnimationStatus> {
        self.animations.status(id)
    }
    /// Freeze value and elapsed time, including any remaining delay. Idempotent.
    pub fn pause_animation(&mut self, id: Id) -> bool {
        self.animations.control(id, Control::Pause, self.frame_time)
    }
    /// Continue from frozen time. Has no effect on completed/cancelled channels.
    pub fn resume_animation(&mut self, id: Id) -> bool {
        self.animations
            .control(id, Control::Resume, self.frame_time)
    }
    /// Freeze the sampled value, discard the track, and suppress completion.
    pub fn cancel_animation(&mut self, id: Id) -> bool {
        self.animations
            .control(id, Control::Cancel, self.frame_time)
    }
    /// Apply the exact final value and emit completion once, including when paused.
    pub fn finish_animation(&mut self, id: Id) -> bool {
        self.animations
            .control(id, Control::Finish, self.frame_time)
    }
}
