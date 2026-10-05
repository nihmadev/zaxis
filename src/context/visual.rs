//! Where content is displayed, as opposed to where it was laid out: transforms composed
//! by placement while a pass is built, the ones the last pass published (input maps
//! pointer positions back through them), the visual effects being built, and the
//! retained state of subtree effects.

use super::{Context, Id};
use crate::{components::effects::EffectState, Rect, Transform};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Visuals {
    /// Nesting of visual effects being built. Inside one, layout positions are not where
    /// content is shown, so hover reads the published regions instead.
    pub(crate) depth: usize,
    /// Clips of the visual effects being built, with the window each one clips.
    pub(crate) clips: Vec<(Id, Rect)>,
    /// Transforms composed by this pass, by widget id.
    current: HashMap<Id, Transform>,
    /// Transforms published by the last pass.
    input: HashMap<Id, Transform>,
    /// Retained state of subtree effects (presence, reveal, motion), by effect id.
    pub(crate) effects: HashMap<Id, EffectState>,
}

impl Visuals {
    pub(super) fn begin_pass(&mut self) {
        self.current.clear();
    }

    /// The published transform of `id`; identity when it has none.
    pub(crate) fn input(&self, id: Id) -> Transform {
        self.input.get(&id).copied().unwrap_or_default()
    }

    /// Map from displayed to layout coordinates of `id`, for input of this pass.
    pub(crate) fn to_local(&self, id: Id) -> Transform {
        self.input(id).inverse()
    }

    /// Apply `transform` on top of what this pass composed for `id` so far.
    pub(super) fn compose(&mut self, id: Id, transform: Transform) {
        let previous = self.current.get(&id).copied().unwrap_or_default();
        self.current.insert(id, transform.compose(previous));
    }

    /// Move `id` by `delta`, giving it a transform when it has none yet.
    pub(super) fn translate(&mut self, id: Id, delta: crate::Vec2) {
        self.current.entry(id).or_default().translation += delta;
    }

    /// Move `id` by `delta` only if something already transformed it.
    pub(super) fn shift(&mut self, id: Id, delta: crate::Vec2) {
        if let Some(transform) = self.current.get_mut(&id) {
            transform.translation += delta;
        }
    }

    /// End of pass: the transforms of widgets that `live` accepts route the next input.
    pub(super) fn publish(&mut self, live: impl Fn(Id) -> bool) {
        self.current.retain(|id, _| live(*id));
        self.input = std::mem::take(&mut self.current);
    }

    /// Subtree effects not built in pass `frame` are dropped.
    pub(super) fn retire_effects(&mut self, frame: u64) {
        self.effects.retain(|_, state| state.last_frame == frame);
    }

    /// The current value of the subtree effect `id`, if it is retained.
    pub(crate) fn effect_value(&self, id: Id) -> Option<f32> {
        self.effects.get(&id).map(|state| state.value)
    }

    #[doc(hidden)]
    pub(super) fn published(&self) -> &HashMap<Id, Transform> {
        &self.input
    }
}

impl Context {
    /// Current visual bounds of a widget's logical allocation. During a pass,
    /// call after the enclosing visual/reorder helper has placed its content.
    pub fn visual_rect(&self, id: Id, logical: Rect) -> Rect {
        let transforms = if self.in_pass {
            &self.visuals.current
        } else {
            &self.visuals.input
        };
        transforms
            .get(&id)
            .copied()
            .unwrap_or_default()
            .rect(logical)
    }
}
