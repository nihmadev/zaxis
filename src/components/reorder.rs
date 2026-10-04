use super::Ui;
use crate::{Id, Rect, TweenOptions, Vec2};
use std::{collections::HashSet, hash::Hash};

/// Opt-in visual placement. Supply full MODEL membership, even with virtual rows.
/// Only built, clipped-in rows wake the host. Missing model IDs are deleted at
/// end of pass; an ID returning after deletion starts at its final layout.
pub struct Reorder {
    id: Id,
    motion: Option<TweenOptions>,
}
/// Result of [`ReorderUi::item_presence`].
pub struct RowPresence<R> {
    /// The closure's result while the row is still on screen.
    pub inner: Option<R>,
    /// The exit has finished: remove the item from the model.
    pub exited: bool,
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
    #[track_caller]
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness =
            super::sanitize::non_negative("Reorder::thickness", thickness).or(self.thickness);
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
            if !members.insert(member) {
                ui.context.report(
                    crate::DiagnosticKind::IdCollision,
                    Some(member),
                    None,
                    || "duplicate id: the reorder model lists an id twice".into(),
                );
            }
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
    /// A row that also fades (or slides, scales, rotates) in and out through
    /// `presence`. While `present` is false the row plays its exit and keeps its
    /// space; when it has finished, `exited` is true and the application should
    /// drop the item from its model, after which the rows below glide up into
    /// the gap. New items enter with the same motion at their final place while
    /// the rows after them glide down.
    ///
    /// Items present when the list first appears fade in too; pass
    /// `Presence::appear(false)` for those so only later arrivals animate.
    pub fn item_presence<R>(
        &mut self,
        member: Id,
        present: bool,
        presence: super::Presence,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> RowPresence<R> {
        let mut shown = None;
        self.item(member, |ui| {
            shown = presence.show(ui, "row", present, build);
        });
        RowPresence {
            exited: !present && shown.is_none(),
            inner: shown,
        }
    }
    pub fn item<R>(&mut self, member: Id, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        if !self.members.contains(&member) || !self.built.insert(member) {
            self.ui.context.report(
                crate::DiagnosticKind::IdCollision,
                Some(member),
                None,
                || "duplicate or unknown id: each reorder row needs one unique model id".into(),
            );
        }
        let key = self.id.with(member);
        self.ui.moving_item(key, self.motion.clone(), build)
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
