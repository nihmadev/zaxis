//! Property transitions: tween, velocity-preserving tween and spring channels.
use super::*;

impl Animations {
    pub(crate) fn spring<T: crate::animation::SpringValue>(
        &mut self,
        id: Id,
        initial: Option<crate::animation::SpringState<T>>,
        target: T,
        options: crate::animation::SpringOptions,
        pass: Pass,
    ) -> Animated<crate::animation::SpringState<T>> {
        self.observed.push(id);
        self.entries.entry(id).or_insert_with(|| {
            if initial.is_none() {
                let rest = crate::animation::SpringState {
                    value: target.clone(),
                    velocity: T::zero(),
                };
                Box::new(Slot::new(
                    rest.clone(),
                    Some(rest),
                    None,
                    AnimationOptions::default(),
                    pass.now,
                ))
            } else {
                let initial = initial.unwrap_or(crate::animation::SpringState {
                    value: target.clone(),
                    velocity: T::zero(),
                });
                let track = crate::animation::Spring::with_options(
                    initial.value.clone(),
                    target.clone(),
                    options,
                )
                .velocity(initial.velocity.clone());
                let slot = Slot::new(
                    initial,
                    Some(crate::animation::SpringState {
                        value: target.clone(),
                        velocity: T::zero(),
                    }),
                    Some(Box::new(track)),
                    AnimationOptions::default(),
                    pass.now,
                );
                Box::new(slot)
            }
        });
        let slot = self.slot::<crate::animation::SpringState<T>>(id).unwrap();
        if slot
            .target
            .as_ref()
            .is_none_or(|state| state.value != target)
        {
            slot.evaluate(pass.now, pass.reduced);
            let from = slot.value.clone();
            let track =
                crate::animation::Spring::with_options(from.value.clone(), target.clone(), options)
                    .velocity(from.velocity.clone());
            *slot = Slot::new(
                from,
                Some(crate::animation::SpringState {
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
            pass.scale,
        )
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
        self.entries.entry(id).or_insert_with(|| {
            let from = initial.unwrap_or_else(|| target.clone());
            let track: Option<Box<dyn Animation<T>>> = (from != target).then(|| {
                Box::new(Tween::with_options(
                    from.clone(),
                    target.clone(),
                    options.clone(),
                )) as _
            });
            Box::new(Slot::new(
                from,
                Some(target.clone()),
                track,
                AnimationOptions::default(),
                pass.now,
            ))
        });
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
            pass.scale,
        )
    }
    /// Like `transition`, but a retarget keeps the displayed velocity: the new
    /// leg is a Hermite curve that starts with it. From rest the configured
    /// tween (easing, delay) is used unchanged.
    pub(crate) fn transition_smooth<T: crate::animation::SpringValue + Interpolate>(
        &mut self,
        id: Id,
        initial: Option<T>,
        target: T,
        options: TweenOptions,
        pass: Pass,
    ) -> Animated<T> {
        self.observed.push(id);
        self.entries.entry(id).or_insert_with(|| {
            let from = initial.unwrap_or_else(|| target.clone());
            let track: Option<Box<dyn Animation<T>>> = (from != target).then(|| {
                Box::new(Tween::with_options(
                    from.clone(),
                    target.clone(),
                    options.clone(),
                )) as _
            });
            Box::new(Slot::new(
                from,
                Some(target.clone()),
                track,
                AnimationOptions::default(),
                pass.now,
            ))
        });
        let slot = self.slot::<T>(id).unwrap();
        if slot.target.as_ref() != Some(&target) {
            slot.evaluate(pass.now, pass.reduced);
            let from = slot.value.clone();
            let velocity = slot.velocity(pass.now);
            let track: Box<dyn Animation<T>> = if velocity.norm() > 1.0e-6 {
                Box::new(crate::animation::hermite::Hermite::new(
                    from.clone(),
                    target.clone(),
                    velocity,
                    options.duration,
                ))
            } else {
                Box::new(Tween::with_options(from.clone(), target.clone(), options))
            };
            *slot = Slot::new(
                from,
                Some(target),
                Some(track),
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
            pass.scale,
        )
    }
}
