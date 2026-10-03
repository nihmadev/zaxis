use super::{effects::EffectState, Ui};
use crate::{Id, Rect, Transform, TweenOptions, Vec2};
use std::{collections::HashSet, hash::Hash};

/// Opt-in visual placement. Supply full MODEL membership, even with virtual rows.
/// Only built, clipped-in rows wake the host. Missing model IDs are deleted at
/// end of pass; an ID returning after deletion starts at its final layout.
pub struct Reorder {
    id: Id,
    motion: Option<TweenOptions>,
}
pub struct ReorderUi<'a, 'b> {
    ui: &'a mut Ui<'b>,
    id: Id,
    members: HashSet<Id>,
    built: HashSet<Id>,
    motion: TweenOptions,
}
/// Decorative selection marker; activation and focus stay under application control.
pub struct SelectionIndicator {
    id: Id,
    orientation: crate::Layout,
    motion: Option<TweenOptions>,
    color: Option<crate::Color>,
    thickness: Option<f32>,
}
impl SelectionIndicator {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            orientation: crate::Layout::Horizontal,
            motion: None,
            color: None,
            thickness: None,
        }
    }
    pub fn orientation(mut self, orientation: crate::Layout) -> Self {
        self.orientation = orientation;
        self
    }
    pub fn motion(mut self, motion: TweenOptions) -> Self {
        self.motion = Some(motion);
        self
    }
    pub fn color(mut self, color: crate::Color) -> Self {
        self.color = Some(color);
        self
    }
    pub fn thickness(mut self, thickness: f32) -> Self {
        assert!(thickness.is_finite() && thickness >= 0.0);
        self.thickness = Some(thickness);
        self
    }
    pub fn show(
        self,
        ui: &mut Ui<'_>,
        selected: Option<Id>,
        bounds: &[(Id, Rect)],
    ) -> Option<Rect> {
        let id = ui.scope.with(("selection-indicator", self.id));
        let Some(rect) =
            selected.and_then(|id| bounds.iter().find(|(key, _)| *key == id).map(|(_, r)| *r))
        else {
            ui.context.remove_animation(id);
            return None;
        };
        let origin = ui.content_origin();
        let thickness = self
            .thickness
            .unwrap_or(ui.style().motion.indicator_thickness);
        let target = match self.orientation {
            crate::Layout::Horizontal => Rect::from_min_max(
                Vec2::new(rect.min.x, (rect.max.y - thickness).max(rect.min.y)),
                rect.max,
            ),
            crate::Layout::Vertical => Rect::from_min_max(
                rect.min,
                Vec2::new((rect.min.x + thickness).min(rect.max.x), rect.max.y),
            ),
        }
        .translate(-origin);
        let options = self
            .motion
            .unwrap_or_else(|| ui.style().motion.reorder.clone());
        let value = ui
            .context
            .transition_visible(
                id,
                None,
                target,
                options,
                !rect.intersect(ui.clip_rect()).is_empty(),
            )
            .value
            .translate(origin);
        ui.context.paint(
            id.with("paint"),
            ui.window,
            ui.clip,
            vec![crate::context::Paint::Shape(
                crate::Shape::rect(value, self.color.unwrap_or(ui.style().accent)).into(),
            )],
        );
        Some(value)
    }
}
impl Reorder {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            motion: None,
        }
    }
    pub fn motion(mut self, motion: TweenOptions) -> Self {
        self.motion = Some(motion);
        self
    }
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        model: impl IntoIterator<Item = Id>,
        build: impl FnOnce(&mut ReorderUi<'_, '_>) -> R,
    ) -> R {
        if ui.flow.is_some() {
            return ui.layout_item(|ui| self.show(ui, model, build));
        }
        let id = ui.scope.with(("reorder", self.id));
        let motion = self
            .motion
            .unwrap_or_else(|| ui.style().motion.reorder.clone());
        let mut members = HashSet::new();
        let hidden_pass = ui.context.animation_pass(false);
        for member in model {
            assert!(members.insert(member), "duplicate reorder model ID");
            let key = id.with(member);
            if let Some(state) = ui.context.effect_states.get_mut(&key) {
                state.last_frame = ui.context.frame;
            }
            ui.context.animations.read::<Vec2>(key, hidden_pass);
        }
        build(&mut ReorderUi {
            ui,
            id,
            members,
            built: HashSet::new(),
            motion,
        })
    }
}
impl<'a, 'b> ReorderUi<'a, 'b> {
    pub fn ui(&mut self) -> &mut Ui<'b> {
        self.ui
    }
    pub fn item<R>(&mut self, member: Id, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        assert!(
            self.members.contains(&member) && self.built.insert(member),
            "row must have a unique model ID"
        );
        let key = self.id.with(member);
        let (inner, size, placement) = self.ui.measure_effect(key, true, build);
        let rect = Rect::from_min_size(self.ui.layout.cursor, size);
        // Normalize by scroll CONTENT origin: viewport scrolling never retargets.
        let origin = self.ui.content_origin();
        let target = rect.min - origin;
        let virtual_return = self
            .ui
            .context
            .effect_states
            .get(&key)
            .is_some_and(|s| s.built_frame + 1 < self.ui.context.frame);
        if virtual_return {
            self.ui.context.remove_animation(key);
        }
        let old = if virtual_return {
            None
        } else {
            self.ui.context.effect_states.get(&key).map(|s| s.position)
        };
        let motion =
            self.ui
                .context
                .transition_visible(key, old, target, self.motion.clone(), false);
        let delta = motion.value - target;
        let visible = !rect
            .translate(delta)
            .intersect(self.ui.clip_rect())
            .is_empty()
            || !rect.intersect(self.ui.clip_rect()).is_empty();
        if visible {
            let pass = self.ui.context.animation_pass(true);
            self.ui.context.animations.read::<Vec2>(key, pass);
        } else {
            self.ui.context.hide_placement_animations(&placement);
        }
        let mut state = EffectState::new(self.ui.context.frame);
        state.position = motion.value;
        self.ui.context.effect_states.insert(key, state);
        self.ui.context.place_visual(
            placement,
            Transform::translation(delta),
            1.0,
            self.ui.clip_rect(),
            true,
        );
        self.ui.allocate_space(size);
        inner
    }
}
impl Ui<'_> {
    pub(super) fn content_origin(&self) -> Vec2 {
        self.context
            .scrolling
            .stack
            .last()
            .copied()
            .filter(|&n| self.context.scrolling.scopes[n].window == self.window)
            .map_or(Vec2::ZERO, |n| self.context.scrolling.scopes[n].origin)
    }
    /// Bounds belong to stable selected IDs. Pass logical response bounds from
    /// this layout; scroll is normalized out. Indicator is decorative and clipped.
    pub fn selection_indicator(
        &mut self,
        source: impl Hash,
        selected: Option<Id>,
        bounds: &[(Id, Rect)],
        orientation: crate::Layout,
    ) -> Option<Rect> {
        SelectionIndicator::new(source)
            .orientation(orientation)
            .show(self, selected, bounds)
    }
}
