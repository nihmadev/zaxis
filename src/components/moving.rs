//! Elements that glide to a new place instead of jumping: the shared core of
//! [`crate::Reorder`] rows and standalone [`Ui::shared`] elements.
use super::{effects::EffectState, Ui};
use crate::{Id, Rect, Transform, TweenOptions, Vec2};
use std::hash::Hash;

impl Ui<'_> {
    /// Identity of the scroll area (or window) whose coordinate system an element
    /// lives in. Two elements in the same scope can be compared in content space.
    fn motion_scope(&self) -> Id {
        self.context
            .scrolling
            .stack
            .last()
            .copied()
            .filter(|&n| self.context.scrolling.scopes[n].window == self.window)
            .map_or(self.window, |n| self.context.scrolling.scopes[n].id)
    }

    /// Lay `build` out at the cursor and animate it from where it was displayed
    /// last pass to where it is now. `key` identifies the element across passes.
    ///
    /// Positions are normalised by the scroll content origin, so scrolling never
    /// counts as movement. An element that moved to another scroll area or window
    /// continues from its on-screen position. One that was not built for more
    /// than a pass (virtualised away, or hidden) starts at its new place.
    pub(super) fn moving_item<R>(
        &mut self,
        key: Id,
        motion: TweenOptions,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        let (inner, size, placement) = self.measure_effect(key, true, build);
        let rect = Rect::from_min_size(self.layout.cursor, size);
        let origin = self.content_origin();
        let scope = self.motion_scope();
        let target = rect.min - origin;
        let previous = self.context.effect_states.get(&key);
        let absent = previous.is_none_or(|s| s.built_frame + 1 < self.context.frame);
        let old = match previous {
            Some(s) if !absent && s.scope == Some(scope) => Some(s.position),
            // Another scroll area: the previous screen position, re-expressed here.
            Some(s) if !absent => Some(s.position + s.origin - origin),
            _ => None,
        };
        // A running channel is in the old scope's coordinates: start a new one
        // from the converted position instead of retargeting it.
        let crossed = previous.is_some_and(|s| s.scope != Some(scope));
        if absent || crossed {
            self.context.remove_animation(key);
        }
        let shown = self
            .context
            .transition_visible(key, old, target, motion, false);
        let delta = shown.value - target;
        let clip = self.clip_rect();
        let visible =
            !rect.translate(delta).intersect(clip).is_empty() || !rect.intersect(clip).is_empty();
        if visible {
            let pass = self.context.animation_pass(true);
            self.context.animations.read::<Vec2>(key, pass);
        } else {
            self.context.hide_placement_animations(&placement);
        }
        let mut state = EffectState::new(self.context.frame);
        state.position = shown.value;
        state.origin = origin;
        state.scope = Some(scope);
        self.context.effect_states.insert(key, state);
        self.context
            .place_visual(placement, Transform::translation(delta), 1.0, clip, true);
        self.allocate_space(size);
        inner
    }

    /// An element with a global identity that glides to wherever it is built
    /// next, **including into a different container**: move a card from one list
    /// to another and it travels across instead of vanishing and reappearing.
    /// Widgets inside keep their identity and state (focus, text cursor) across
    /// the move because their IDs derive from `source`, not from the parent.
    ///
    /// `source` must be unique among elements built in a pass. Uses
    /// `MotionStyle::reorder`; see [`Ui::shared_with`].
    pub fn shared<R>(&mut self, source: impl Hash, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        let motion = self.style().motion.reorder.clone();
        self.shared_with(source, motion, build)
    }
    pub fn shared_with<R>(
        &mut self,
        source: impl Hash,
        motion: TweenOptions,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        if self.flow.is_some() {
            return self.layout_item(|ui| ui.shared_with(source, motion, build));
        }
        let motion = super::sanitize::forward("shared motion", motion);
        let key = Id::new(("shared", source));
        if self
            .context
            .effect_states
            .get(&key)
            .is_some_and(|s| s.built_frame == self.context.frame)
        {
            self.context
                .report(crate::DiagnosticKind::IdCollision, Some(key), None, || {
                    "duplicate id: a shared element was built twice in one pass".into()
                });
        }
        self.moving_item(key, motion, build)
    }
}
