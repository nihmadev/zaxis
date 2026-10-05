use super::*;
impl Ui<'_> {
    /// Revision triggers once; repeat smoothly attacks from current alpha to one,
    /// then decays. Base background is supplied fresh each pass (theme/hover safe).
    pub fn highlight(
        &mut self,
        source: impl Hash,
        revision: u64,
        rect: Rect,
        base: crate::Color,
        accent: crate::Color,
    ) {
        self.highlight_with(
            source,
            revision,
            rect,
            base,
            accent,
            self.style().motion.highlight.clone(),
        );
    }
    pub fn highlight_with(
        &mut self,
        source: impl Hash,
        revision: u64,
        rect: Rect,
        base: crate::Color,
        accent: crate::Color,
        release: TweenOptions,
    ) {
        let release = crate::components::sanitize::forward("highlight release", release);
        let id = self.scope.with(("highlight", Id::new(source)));
        let mut state = self.context.visuals.effects.remove(&id).unwrap_or_else(|| {
            let mut state = EffectState::new(self.context.frame);
            state.revision = revision;
            state
        });
        state.last_frame = self.context.frame;
        let pass = self
            .context
            .animation_pass(!rect.intersect(self.clip_rect()).is_empty());
        let channel = id.with("amount");
        let current = self
            .context
            .animations
            .read::<f32>(channel, pass)
            .map_or(0.0, |s| s.value);
        if revision != state.revision {
            state.revision = revision;
            state.value = 1.0;
            let effect = crate::Sequence::new(current)
                .then(crate::Tween::with_options(
                    current,
                    1.0,
                    self.style().motion.hover.clone(),
                ))
                .then(crate::Tween::with_options(1.0, 0.0, release));
            self.context.restart_animation(channel, effect);
        }
        let alpha = self
            .context
            .animations
            .read::<f32>(channel, pass)
            .map_or(0.0, |s| s.value);
        let color = if self.style().motion.reduced_motion && state.value > 0.0 {
            crate::Interpolate::interpolate(&base, &accent, 0.25)
        } else {
            crate::Interpolate::interpolate(&base, &accent, alpha)
        };
        self.context.paint(
            id.with("paint"),
            self.window,
            self.clip,
            vec![Paint::Shape(
                crate::Shape::rect(rect, color)
                    .corner_radius(self.style().rounding.top_left)
                    .into(),
            )],
        );
        self.context.visuals.effects.insert(id, state);
    }
}
