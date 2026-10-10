//! Explicit subtree effects. Closures are used only during the current UI pass.
use super::Ui;
use crate::{
    context::Paint, layout::LayoutCursor, Id, Layout, Rect, Transform, TweenOptions, Vec2,
};
use std::hash::Hash;

pub struct EffectState {
    pub last_frame: u64,
    pub revision: u64,
    pub value: f32,
    pub position: Vec2,
    pub built_frame: u64,
    /// Scroll content origin and scope `position` was measured against.
    pub origin: Vec2,
    pub scope: Option<Id>,
}
impl EffectState {
    pub(crate) fn new(frame: u64) -> Self {
        Self {
            last_frame: frame,
            revision: 0,
            value: 0.0,
            position: Vec2::ZERO,
            built_frame: frame,
            origin: Vec2::ZERO,
            scope: None,
        }
    }
}

/// Hidden pose. Fade, slide and scale can be combined using the builders.
#[derive(Clone, Debug, PartialEq)]
pub struct Presence {
    pub fade: bool,
    pub offset: Option<Vec2>,
    pub scale: f32,
    /// Relative pivot in measured bounds; center by default.
    pub pivot: Vec2,
    /// Keep the allocation during Exit. False releases it immediately.
    pub exit_layout: bool,
    pub interactive_enter: bool,
    pub motion: Option<TweenOptions>,
    /// Hidden pose angle in radians around `pivot`; turns to zero as it shows.
    pub rotation: f32,
    /// Animate the very first appearance. False snaps to the current state.
    pub appear: bool,
}
impl Default for Presence {
    fn default() -> Self {
        Self::fade()
    }
}
impl Presence {
    pub fn fade() -> Self {
        Self {
            fade: true,
            offset: None,
            scale: 1.0,
            pivot: Vec2::splat(0.5),
            exit_layout: true,
            interactive_enter: true,
            motion: None,
            rotation: 0.0,
            appear: true,
        }
    }
    pub fn slide(offset: Vec2) -> Self {
        Self::fade().fading(false).offset(offset)
    }
    pub fn scale(scale: f32) -> Self {
        Self::fade().fading(false).scaling(scale)
    }
    pub fn fading(mut self, fade: bool) -> Self {
        self.fade = fade;
        self
    }
    #[track_caller]
    pub fn offset(mut self, offset: Vec2) -> Self {
        self.offset = super::sanitize::finite_vec2("Presence::offset", offset).or(self.offset);
        self
    }
    #[track_caller]
    pub fn scaling(mut self, scale: f32) -> Self {
        self.scale = super::sanitize::positive("Presence::scaling", scale).unwrap_or(self.scale);
        self
    }
    #[track_caller]
    pub fn pivot(mut self, pivot: Vec2) -> Self {
        self.pivot = super::sanitize::finite_vec2("Presence::pivot", pivot).unwrap_or(self.pivot);
        self
    }
    pub fn exit_layout(mut self, keep: bool) -> Self {
        self.exit_layout = keep;
        self
    }
    pub fn interactive_enter(mut self, enabled: bool) -> Self {
        self.interactive_enter = enabled;
        self
    }
    pub fn motion(mut self, motion: TweenOptions) -> Self {
        self.motion = Some(motion);
        self
    }
    /// Spin from `angle` radians while entering and to it while leaving. Content
    /// is not interactive while it is turned (input follows axis-aligned
    /// geometry), so it becomes usable when the enter completes.
    #[track_caller]
    pub fn rotating(mut self, angle: f32) -> Self {
        self.rotation =
            super::sanitize::finite("Presence::rotating", angle).unwrap_or(self.rotation);
        self
    }
    /// With `false`, a presence whose first pass is already visible starts shown
    /// instead of animating in. Later changes animate as usual.
    pub fn appear(mut self, animate: bool) -> Self {
        self.appear = animate;
        self
    }
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        source: impl Hash,
        visible: bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> Option<R> {
        if ui.flow.is_some() {
            return ui.layout_item(|ui| self.show(ui, source, visible, build));
        }
        let id = ui.scope.with(("presence", Id::new(source)));
        let motion = self
            .motion
            .unwrap_or_else(|| ui.style().motion.presence.clone());
        let motion = super::sanitize::forward("Presence motion", motion);
        let pass = ui.context.animation_pass(!ui.clip_rect().is_empty());
        let progress = ui.context.animations.transition(
            id,
            Some(if visible && !self.appear {
                1.0_f32
            } else {
                0.0
            }),
            if visible { 1.0 } else { 0.0 },
            motion,
            pass,
        );
        if !visible && progress.value <= 0.0 {
            return None;
        }
        let turned = self.rotation != 0.0 && progress.value < 1.0;
        let interactive = visible && !turned && (self.interactive_enter || progress.completed());
        let (inner, size, placement) = ui.measure_effect(id, interactive, build);
        let rect = Rect::from_min_size(ui.layout.cursor, size);
        let t = progress.value.clamp(0.0, 1.0);
        let pivot = rect.min + size * self.pivot;
        let mut transform = Transform {
            scale: 1.0 + (self.scale - 1.0) * (1.0 - t),
            translation: Vec2::ZERO,
            angle: self.rotation * (1.0 - t),
        };
        transform.translation =
            pivot - transform.vector(pivot) + self.offset.unwrap_or(Vec2::ZERO) * (1.0 - t);
        // Parent clip stays fixed; descendant clips follow their geometry.
        let clip = ui.clip_rect();
        if transform.rect(rect).intersect(clip).is_empty() {
            ui.context.animations.hide(id);
            ui.context.hide_placement_animations(&placement);
        }
        ui.context.place_visual(
            placement,
            transform,
            if self.fade { t } else { 1.0 },
            clip,
            interactive,
        );
        if visible || self.exit_layout {
            ui.allocate_space(size);
        }
        Some(inner)
    }
}

impl Ui<'_> {
    /// Apply an axis-aligned visual mapping without changing layout measurements.
    /// Opacity affects every emitted primitive. Parent clipping stays fixed.
    pub fn visual<R>(
        &mut self,
        source: impl Hash,
        transform: Transform,
        opacity: f32,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        self.visual_gated(source, transform, opacity, None, build)
    }

    /// Like [`Self::visual`], but whether the content takes input is decided by `input`.
    /// The content keeps the enabled look of its parent even when it takes no input, so a
    /// layer that is only passing through (a card on its way in) is not drawn greyed out.
    pub(crate) fn visual_gated<R>(
        &mut self,
        source: impl Hash,
        transform: Transform,
        opacity: f32,
        input: Option<bool>,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        if self.flow.is_some() {
            return self
                .layout_item(|ui| ui.visual_gated(source, transform, opacity, input, build));
        }
        let (transform, opacity) = if transform.angle.is_finite()
            && transform.scale.is_finite()
            && transform.scale > 0.0
            && transform.translation.is_finite()
            && opacity.is_finite()
        {
            (transform, opacity)
        } else {
            self.context
                .report(crate::DiagnosticKind::InvalidValue, None, None, || {
                    "Ui::visual: expected a finite transform with scale > 0 and a finite opacity;                  using the identity transform"
                        .into()
                });
            (crate::Transform::IDENTITY, 1.0)
        };
        // Input follows axis-aligned geometry: turned content is look-only.
        let interactive = opacity > 0.0 && transform.angle == 0.0;
        let id = self.scope.with(("visual", Id::new(source)));
        let (inner, size, placement) =
            self.measure_effect_as(id, self.enabled && (interactive || input.is_some()), build);
        let rect = transform.rect(Rect::from_min_size(self.layout.cursor, size));
        if rect.intersect(self.clip).is_empty() {
            self.context.hide_placement_animations(&placement);
        }
        self.context.place_visual(
            placement,
            transform,
            opacity.clamp(0.0, 1.0),
            self.clip,
            interactive && input.unwrap_or(true),
        );
        self.allocate_space(size);
        inner
    }
    pub(super) fn measure_effect<R>(
        &mut self,
        id: Id,
        interactive: bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> (R, Vec2, crate::context::placement::Placement) {
        self.measure_effect_as(id, self.enabled && interactive, build)
    }
    /// Measure `build` in a child UI whose enabled state is `enabled`.
    pub(super) fn measure_effect_as<R>(
        &mut self,
        id: Id,
        enabled: bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> (R, Vec2, crate::context::placement::Placement) {
        let bounds = Rect::from_min_max(self.layout.cursor, self.layout.bounds.max);
        let spacing = self.style().spacing.max(0.0);
        // This clip uses the current UI's units. An enclosing visual's parent clip
        // applies when that enclosing placement closes.
        let parent_clip = self.clip;
        self.context.begin_placement(self.window);
        self.context.visuals.depth += 1;
        self.context.visuals.clips.push((self.window, parent_clip));
        let mut child = Ui {
            flow: None,
            context: self.context,
            window: self.window,
            scope: id,
            sequence: 0,
            clip: Rect::from_min_size(Vec2::splat(-1.0e9), Vec2::splat(2.0e9)),
            layout: LayoutCursor::new(bounds, Layout::Vertical, spacing),
            enabled,
            backdrop_blur: self.backdrop_blur,
            hover_style: self.hover_style,
            local_style: self.local_style.clone(),
            local_style_revision: self.local_style_revision,
        };
        child.begin_layout(crate::Align::Start);
        let inner = build(&mut child);
        child.finish_layout();
        let size = child.layout.used;
        child.context.visuals.depth -= 1;
        child.context.visuals.clips.pop();
        let placement = child.context.end_placement();
        (inner, size, placement)
    }
    pub fn presence<R>(
        &mut self,
        source: impl Hash,
        visible: bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> Option<R> {
        Presence::fade().show(self, source, visible, build)
    }
    /// Height measured once each pass. The closing subtree loses input immediately.
    pub fn reveal<R>(
        &mut self,
        source: impl Hash,
        open: bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> Option<R> {
        self.reveal_with(source, open, self.style().motion.expand.clone(), build)
    }
    pub fn reveal_with<R>(
        &mut self,
        source: impl Hash,
        open: bool,
        motion: TweenOptions,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> Option<R> {
        if self.flow.is_some() {
            return self.layout_item(|ui| ui.reveal_with(source, open, motion, build));
        }
        let motion = super::sanitize::forward("reveal motion", motion);
        let id = self.scope.with(("reveal", Id::new(source)));
        let visible = !self.clip_rect().is_empty();
        let old_height = self
            .context
            .visuals
            .effects
            .get(&id)
            .map_or(0.0, |s| s.value);
        let mut state = EffectState::new(self.context.frame);
        if !open {
            let progress =
                self.context
                    .transition_visible(id, Some(0.0), 0.0_f32, motion.clone(), visible);
            state.value = progress.value;
            if state.value <= 0.0 {
                self.context.visuals.effects.insert(id, state);
                return None;
            }
        }
        let (inner, size, placement) = self.measure_effect(id, open, build);
        let height = self
            .context
            .transition_visible(
                id,
                Some(old_height),
                if open { size.y } else { 0.0 },
                motion,
                visible,
            )
            .value
            .max(0.0);
        state.value = height;
        self.context.visuals.effects.insert(id, state);
        let rect = Rect::from_min_size(self.layout.cursor, Vec2::new(size.x, height));
        let clip = self.clip_rect().intersect(rect);
        let target_bounds = Rect::from_min_size(self.layout.cursor, size);
        if target_bounds.intersect(self.clip_rect()).is_empty() || (clip.is_empty() && !open) {
            self.context.animations.hide(id);
            self.context.hide_placement_animations(&placement);
        }
        self.context
            .place_visual(placement, Transform::IDENTITY, 1.0, clip, open);
        if height > 0.0 {
            self.allocate_space(Vec2::new(size.x, height));
        }
        Some(inner)
    }
    pub fn spring_transition<T: crate::SpringValue>(
        &mut self,
        source: impl Hash,
        target: T,
        options: crate::SpringOptions,
    ) -> crate::Animated<crate::SpringState<T>> {
        let pass = self.context.animation_pass(!self.clip_rect().is_empty());
        self.context
            .animations
            .spring(self.animation_id(source), None, target, options, pass)
    }
    pub fn spring_transition_from<T: crate::SpringValue>(
        &mut self,
        source: impl Hash,
        initial: crate::SpringState<T>,
        target: T,
        options: crate::SpringOptions,
    ) -> crate::Animated<crate::SpringState<T>> {
        let pass = self.context.animation_pass(!self.clip_rect().is_empty());
        self.context.animations.spring(
            self.animation_id(source),
            Some(initial),
            target,
            options,
            pass,
        )
    }
    /// An external active flag controls the lifetime. Hiding resets phase.
    pub fn pulse<R>(
        &mut self,
        source: impl Hash,
        active: bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        self.pulse_with(
            source,
            active,
            crate::Pulse::new(self.style().motion.cycle_period),
            build,
        )
    }
    pub fn pulse_with<R>(
        &mut self,
        source: impl Hash,
        active: bool,
        effect: crate::Pulse,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        if self.flow.is_some() {
            return self.layout_item(|ui| ui.pulse_with(source, active, effect, build));
        }
        let id = self.scope.with(("pulse", Id::new(source)));
        let opacity = if active && !self.style().motion.reduced_motion {
            let pass = self.context.animation_pass(!self.clip_rect().is_empty());
            self.context
                .animations
                .animate(id, crate::AnimationOptions::default(), || effect, pass)
                .value
        } else {
            self.context.remove_animation(id);
            1.0
        };
        let (inner, size, placement) = self.measure_effect(id, true, build);
        let rect = Rect::from_min_size(self.layout.cursor, size);
        if rect.intersect(self.clip_rect()).is_empty() {
            self.context.animations.hide(id);
            self.context.hide_placement_animations(&placement);
        }
        self.context.place_visual(
            placement,
            Transform::IDENTITY,
            opacity,
            self.clip_rect(),
            true,
        );
        self.allocate_space(size);
        inner
    }
}
mod highlight;
