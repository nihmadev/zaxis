use super::{Response, Ui, Widget};
use crate::{context::Paint, Color, Id, Rect, Shape, Vec2};
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
    size: Vec2,
    color: Option<Color>,
}
impl Progress {
    #[track_caller]
    pub fn new(state: ProgressState) -> Self {
        Self {
            state,
            id: None,
            source: Location::caller(),
            size: Vec2::new(160.0, 4.0),
            color: None,
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    pub fn size(mut self, size: Vec2) -> Self {
        assert!(size.is_finite() && size.min_element() >= 0.0);
        self.size = size;
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
        let rect = ui.allocate_space(self.size);
        let channel = id.with("phase");
        let color = self.color.unwrap_or(ui.style().text_color);
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
            ProgressState::Determinate(value) => {
                assert!(value.is_finite());
                (0.0, value.clamp(0.0, 1.0))
            }
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
        ui.context.paint(
            id,
            ui.window,
            ui.clip,
            vec![
                Paint::Shape(
                    Shape::rect(rect, ui.style().button_fill)
                        .corner_radius(self.size.y * 0.5)
                        .into(),
                ),
                Paint::Shape(
                    Shape::rect(segment, color)
                        .corner_radius(self.size.y * 0.5)
                        .into(),
                ),
            ],
        );
        ui.response(id, rect, false)
    }
}
