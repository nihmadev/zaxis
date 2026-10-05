use super::{Response, Ui, Widget};
use crate::{context::Paint, Color, Id, Rotation, Shape, Vec2};
use std::{hash::Hash, panic::Location, time::Duration};

/// Lucide `loader`, eight round-capped strokes in a 24x24 viewbox.
/// Geometry: <https://lucide.dev/icons/loader> (ISC/Feather MIT; assets/LUCIDE-LICENSE).
/// No hit registration, constant size regardless of rotation.
///
/// While active it is a busy progress indicator to screen readers; give it a name with
/// [`Loader::label`]. An inactive loader is decoration and is not announced.
pub struct Loader {
    label: Option<String>,
    id: Option<Id>,
    source: &'static Location<'static>,
    active: bool,
    size: Option<f32>,
    color: Option<Color>,
    stroke: Option<f32>,
    period: Option<Duration>,
    style: crate::LoaderStyle,
}
impl Default for Loader {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}
impl Loader {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            label: None,
            id: None,
            source: Location::caller(),
            active: true,
            size: None,
            color: None,
            stroke: None,
            period: None,
            style: Default::default(),
        }
    }
    pub fn style(mut self, style: crate::LoaderStyle) -> Self {
        self.style = style;
        self
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
    /// What is being waited for, for screen readers ("Loading messages"). Nothing is drawn.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    #[track_caller]
    pub fn size(mut self, size: f32) -> Self {
        self.size = super::sanitize::non_negative("Loader::size", size).or(self.size);
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    #[track_caller]
    pub fn stroke(mut self, stroke: f32) -> Self {
        self.stroke = super::sanitize::non_negative("Loader::stroke", stroke).or(self.stroke);
        self
    }
    pub fn period(mut self, period: Duration) -> Self {
        self.period = Some(period);
        self
    }
}
impl Widget for Loader {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let id = match self.id {
            Some(id) => ui.scope.with(("loader", id)),
            None => ui.auto_id(("loader", self.source)),
        };
        let mut style = ui.style().loader;
        style.merge(self.style);
        let size = self
            .size
            .or(style.size)
            .unwrap_or(ui.style().motion.loader_size);
        let width = self
            .stroke
            .or(style.stroke)
            .unwrap_or(ui.style().motion.loader_stroke);
        let color = self.color.or(style.color).unwrap_or(if ui.is_enabled() {
            ui.style().muted_text
        } else {
            ui.style().disabled_text
        });
        let period = self.period.unwrap_or(ui.style().motion.cycle_period);
        let rect = ui.allocate_space(Vec2::splat(size));
        if self.active {
            ui.a11y(id, rect, crate::AccessRole::ProgressIndicator, |node| {
                node.busy(true);
                match &self.label {
                    Some(label) => node.label(label.as_str()),
                    // A spinner next to the text that explains it needs no name.
                    None => node.unnamed(),
                };
            });
        }
        let channel = id.with("rotation");
        let visible = self.active
            && !ui.style().motion.reduced_motion
            && !rect.intersect(ui.clip_rect()).is_empty();
        let angle = if visible {
            let pass = ui.context.animation_pass(true);
            ui.context
                .animations
                .animate(
                    channel,
                    crate::AnimationOptions::default(),
                    || Rotation::new(period),
                    pass,
                )
                .value
        } else {
            ui.context.remove_animation(channel);
            0.0
        };
        let point = |p: Vec2| rect.min + p * (size / 24.0);
        let strokes = [
            ((12.0, 2.0), (12.0, 6.0)),
            ((16.2, 7.8), (19.1, 4.9)),
            ((18.0, 12.0), (22.0, 12.0)),
            ((16.2, 16.2), (19.1, 19.1)),
            ((12.0, 18.0), (12.0, 22.0)),
            ((4.9, 19.1), (7.8, 16.2)),
            ((2.0, 12.0), (6.0, 12.0)),
            ((4.9, 4.9), (7.8, 7.8)),
        ];
        let mut paint = Vec::new();
        for (a, b) in strokes {
            let start = point(Vec2::new(a.0, a.1));
            let end = point(Vec2::new(b.0, b.1));
            paint.push(Paint::Shape(Shape::Line {
                start,
                end,
                width,
                color,
            }));
            // Existing line primitive has butt caps; Lucide requires round caps.
            for center in [start, end] {
                paint.push(Paint::Shape(Shape::Circle {
                    center,
                    radius: width * 0.5,
                    fill: color,
                    border: crate::Border::NONE,
                }));
            }
        }
        ui.context.paint(
            id,
            ui.window,
            ui.clip,
            vec![Paint::Visual {
                paint,
                transform: crate::Transform::rotation(rect.center(), angle),
                opacity: 1.0,
            }],
        );
        ui.response(id, rect, false)
    }
}
impl Ui<'_> {
    #[track_caller]
    pub fn loader(&mut self, active: bool) -> Response {
        self.add(Loader::new().active(active))
    }
}
