use super::Ui;
use crate::{layout::LayoutCursor, Id, Layout, Rect, Vec2};
use std::hash::Hash;

pub(crate) struct TabPagesState {
    pub last_frame: u64,
    current: usize,
    outgoing: Option<usize>,
    pending: Option<usize>,
    direction: f32,
}

impl Ui<'_> {
    /// A stable, scoped channel ID for an application animation/property.
    pub fn animation_id(&self, source: impl Hash) -> Id {
        self.scope.with(("animation", Id::new(source)))
    }
    pub fn transition<T: crate::Interpolate>(
        &mut self,
        source: impl Hash,
        target: T,
        options: crate::TweenOptions,
    ) -> crate::Animated<T> {
        self.context.transition_visible(
            self.animation_id(source),
            None,
            target,
            options,
            !self.clip_rect().is_empty(),
        )
    }
    pub fn transition_from<T: crate::Interpolate>(
        &mut self,
        source: impl Hash,
        initial: T,
        target: T,
        options: crate::TweenOptions,
    ) -> crate::Animated<T> {
        self.context.transition_visible(
            self.animation_id(source),
            Some(initial),
            target,
            options,
            !self.clip_rect().is_empty(),
        )
    }
    pub fn animate<T: Clone + 'static, A: crate::Animation<T>>(
        &mut self,
        source: impl Hash,
        create: impl FnOnce() -> A,
    ) -> crate::Animated<T> {
        self.animate_with(source, crate::AnimationOptions::default(), create)
    }
    pub fn animate_with<T: Clone + 'static, A: crate::Animation<T>>(
        &mut self,
        source: impl Hash,
        options: crate::AnimationOptions,
        create: impl FnOnce() -> A,
    ) -> crate::Animated<T> {
        let pass = self.context.animation_pass(!self.clip_rect().is_empty());
        self.context
            .animations
            .animate(self.animation_id(source), options, create, pass)
    }

    pub(super) fn transition_property<T: crate::Interpolate>(
        &mut self,
        id: Id,
        rect: Rect,
        target: T,
        options: crate::TweenOptions,
    ) -> T {
        self.context
            .transition_visible(
                id,
                None,
                target,
                options,
                !rect.intersect(self.clip_rect()).is_empty(),
            )
            .value
    }

    /// Directional page slides in a fixed allocation. While switching, only the
    /// latest requested index is queued. `build` runs for both visible pages during
    /// a slide; controls are disabled until it ends. Hidden pages are not built.
    /// The translated layout drives paint and hit geometry under a fixed clip.
    pub fn tab_pages(
        &mut self,
        source: impl Hash,
        selected: usize,
        size: Vec2,
        mut build: impl FnMut(&mut Ui<'_>, usize),
    ) {
        let id = self.scope.with(("tab-pages", Id::new(source)));
        let rect = self.allocate_space(size.min(Vec2::new(self.available_width(), size.y)));
        let mut state = self.context.tab_pages.remove(&id).unwrap_or(TabPagesState {
            last_frame: 0,
            current: selected,
            outgoing: None,
            pending: None,
            direction: 1.0,
        });
        state.last_frame = self.context.frame;
        let progress_id = id.with("slide");
        let mut progress = 1.0;
        if state.outgoing.is_some() {
            state.pending = (selected != state.current).then_some(selected);
            let pass = self
                .context
                .animation_pass(!rect.intersect(self.clip_rect()).is_empty());
            let motion = self
                .context
                .animations
                .read::<f32>(progress_id, pass)
                .unwrap();
            progress = motion.value;
            if motion.completed() {
                state.outgoing = None;
            }
        }
        if state.outgoing.is_none() {
            let next = state.pending.take().unwrap_or(selected);
            if next != state.current {
                state.direction = if next > state.current { 1.0 } else { -1.0 };
                state.outgoing = Some(state.current);
                state.current = next;
                let tween =
                    crate::Tween::with_options(0.0_f32, 1.0, self.style().motion.page.clone());
                self.context.restart_animation(progress_id, tween);
                let pass = self
                    .context
                    .animation_pass(!rect.intersect(self.clip_rect()).is_empty());
                let motion = self
                    .context
                    .animations
                    .read::<f32>(progress_id, pass)
                    .unwrap();
                progress = motion.value;
                if motion.completed() {
                    state.outgoing = None;
                }
            }
        }
        let switching = state.outgoing.is_some();
        let distance = (rect.size().x + 20.0) * state.direction;
        if let Some(outgoing) = state.outgoing {
            self.build_page(id, outgoing, rect, -distance * progress, false, &mut build);
        }
        self.build_page(
            id,
            state.current,
            rect,
            if switching {
                distance * (1.0 - progress)
            } else {
                0.0
            },
            !switching,
            &mut build,
        );
        self.context.tab_pages.insert(id, state);
    }

    fn build_page(
        &mut self,
        id: Id,
        index: usize,
        rect: Rect,
        offset: f32,
        interactive: bool,
        build: &mut impl FnMut(&mut Ui<'_>, usize),
    ) {
        let bounds = rect.translate(Vec2::new(offset, 0.0));
        let clip = self.clip.intersect(rect).intersect(bounds);
        if clip.is_empty() {
            return;
        }
        let spacing = self.style().spacing.max(0.0);
        let mut page = Ui {
            flow: None,
            context: self.context,
            window: self.window,
            scope: id.with(("page", index)),
            sequence: 0,
            clip,
            layout: LayoutCursor::new(bounds, Layout::Vertical, spacing),
            enabled: self.enabled && interactive,
            backdrop_blur: self.backdrop_blur,
            hover_style: self.hover_style,
        };
        page.begin_layout(crate::Align::Start);
        build(&mut page, index);
        page.finish_layout();
    }
}
