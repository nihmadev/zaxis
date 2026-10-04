use super::{Response, Ui, Widget};
use crate::{Color, Id, Rect, Vec2};
use std::{hash::Hash, panic::Location};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProgressState {
    Indeterminate { active: bool },
    Determinate(f32),
    Complete,
}
/// No fabricated percentage for indeterminate work. Application state switches
/// immediately; the animated segment is purely decorative and owns no input.
pub struct Progress {
    state: ProgressState,
    id: Option<Id>,
    source: &'static Location<'static>,
    size: Option<Vec2>,
    color: Option<Color>,
    style: crate::ProgressStyle,
}
impl Progress {
    #[track_caller]
    pub fn new(state: ProgressState) -> Self {
        Self {
            state,
            id: None,
            source: Location::caller(),
            size: None,
            color: None,
            style: Default::default(),
        }
    }
    pub fn style(mut self, style: crate::ProgressStyle) -> Self {
        self.style = style;
        self
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    #[track_caller]
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = super::sanitize::size("Progress::size", size).or(self.size);
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}
impl Widget for Progress {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let id = match self.id {
            Some(id) => ui.scope.with(("progress", id)),
            None => ui.auto_id(("progress", self.source)),
        };
        let mut component = ui.style().progress;
        component.merge(self.style);
        let size = self
            .size
            .or(component.size)
            .unwrap_or(Vec2::new(160.0, 4.0));
        let rect = ui.allocate_space(size);
        let channel = id.with("phase");
        let color = self.color.unwrap_or(if ui.is_enabled() {
            ui.style().accent
        } else {
            ui.style().disabled_text
        });
        let busy = matches!(self.state, ProgressState::Indeterminate { active: true });
        let visible = !rect.intersect(ui.clip_rect()).is_empty();
        let phase = if busy && visible && !ui.style().motion.reduced_motion {
            let pass = ui.context.animation_pass(true);
            let period = ui.style().motion.cycle_period;
            ui.context
                .animations
                .animate(
                    channel,
                    crate::AnimationOptions::default(),
                    || crate::Rotation::new(period),
                    pass,
                )
                .value
                / std::f32::consts::TAU
        } else {
            ui.context.remove_animation(channel);
            0.0
        };
        let (offset, width) = match self.state {
            ProgressState::Determinate(value) => (
                0.0,
                super::sanitize::unit("ProgressState::Determinate", value),
            ),
            ProgressState::Complete => (0.0, 1.0),
            ProgressState::Indeterminate { .. } => (
                if ui.style().motion.reduced_motion || !busy {
                    0.35
                } else {
                    (0.5 - 0.5 * (phase * std::f32::consts::TAU).cos()) * 0.7
                },
                0.3,
            ),
        };
        let segment = Rect::from_min_size(
            rect.min + Vec2::new(rect.size().x * offset, 0.0),
            Vec2::new(rect.size().x * width, rect.size().y),
        );
        let style = ui.style().clone();
        let mut track = super::appearance::Appearance::new(
            style.button_fill,
            crate::Border::NONE,
            style.text_color,
        );
        track.rounding = crate::CornerRadius::all(size.y * 0.5);
        track.opacity = style.opacity;
        track.blur = 0.0;
        track.apply(component.track);
        let mut fill = track;
        fill.fill = crate::Gradient::new(color, color);
        fill.apply(component.fill);
        let (_, filter) = ui.control_blur(Some(track.blur));
        ui.context.paint_blur(
            id.with("blur"),
            ui.window,
            ui.clip,
            crate::Blur::new(rect)
                .radius(filter)
                .corner_radius(track.rounding),
        );
        let mut paint = Vec::new();
        track.paint_shadow(rect, track.rounding, &mut paint);
        track.paint_body(rect, track.rounding, &style, track.blur, &mut paint);
        fill.paint_shadow(segment, fill.rounding, &mut paint);
        fill.paint_body(segment, fill.rounding, &style, fill.blur, &mut paint);
        ui.context.paint(id, ui.window, ui.clip, paint);
        ui.response(id, rect, false)
    }
}
