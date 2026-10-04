use super::{Response, Ui, Widget};
use crate::{context::Paint, AnimationOptions, Color, Id, Rect, Repeat, Shape, Tween, Vec2};
use std::{hash::Hash, panic::Location, time::Duration};

/// Placeholder block for content that is still loading: a muted rounded
/// rectangle with a soft highlight sweeping across it. Decorative, no hit
/// registration. Reduced motion, hidden or clipped placeholders stay still and
/// request no frames.
pub struct Skeleton {
    id: Option<Id>,
    source: &'static Location<'static>,
    width: Option<f32>,
    height: f32,
    rounding: Option<f32>,
    period: Option<Duration>,
}

impl Skeleton {
    /// A block as wide as the available space and `height` tall.
    #[track_caller]
    pub fn new(height: f32) -> Self {
        Self {
            id: None,
            source: Location::caller(),
            width: None,
            height: super::sanitize::length("Skeleton::new", height),
            rounding: None,
            period: None,
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = super::sanitize::non_negative("Skeleton::width", width).or(self.width);
        self
    }
    /// One radius for the whole block (the top-left value of a non-uniform radius).
    #[track_caller]
    pub fn corner_radius(mut self, radius: impl Into<crate::CornerRadius>) -> Self {
        let radius = radius.into().top_left;
        self.rounding =
            super::sanitize::non_negative("Skeleton::corner_radius", radius).or(self.rounding);
        self
    }
    #[deprecated(
        note = "use `.corner_radius(..)`; one name for the corner radius of every component"
    )]
    #[track_caller]
    pub fn rounding(self, radius: f32) -> Self {
        self.corner_radius(radius)
    }
    /// Time for the highlight to cross the block (defaults to
    /// `MotionStyle::cycle_period`).
    pub fn period(mut self, period: Duration) -> Self {
        self.period = Some(period);
        self
    }
}

impl Widget for Skeleton {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let id = match self.id {
            Some(id) => ui.scope.with(("skeleton", id)),
            None => ui.auto_id(("skeleton", self.source)),
        };
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let rect = ui.allocate_space(Vec2::new(width, self.height));
        let style = ui.style().clone();
        let radius = self.rounding.unwrap_or(style.rounding.top_left);
        ui.paint(Shape::rect(rect, style.button_fill).corner_radius(radius));

        let channel = id.with("shimmer");
        let moving = !style.motion.reduced_motion
            && rect.size().min_element() > 0.0
            && !rect.intersect(ui.clip_rect()).is_empty();
        if !moving {
            ui.context.remove_animation(channel);
            return ui.response(id, rect, false);
        }
        let period = self.period.unwrap_or(style.motion.cycle_period);
        let pass = ui.context.animation_pass(true);
        let t = ui
            .context
            .animations
            .animate(
                channel,
                AnimationOptions::default(),
                || Tween::new(0.0_f32, 1.0, period).repeat(Repeat::Forever),
                pass,
            )
            .value;
        // The highlight is cut into thin vertical slices whose height follows the
        // rounded outline, so it never pokes out of a circle or a pill. Each slice
        // fades with a raised-cosine profile around the band centre.
        let size = rect.size();
        let radius = radius.min(size.min_element() * 0.5);
        let half = size.x * 0.3;
        let centre = rect.min.x - half + t * (size.x + 2.0 * half);
        let highlight = style.button_hovered;
        let (from, to) = (
            (centre - half).max(rect.min.x),
            (centre + half).min(rect.max.x),
        );
        if to > from {
            let count = (((to - from) / 2.0).ceil() as usize).clamp(1, 48);
            let step = (to - from) / count as f32;
            let inset = |x: f32| {
                let edge = (x - rect.min.x).min(rect.max.x - x);
                if edge >= radius {
                    0.0
                } else {
                    radius - (radius * radius - (radius - edge).powi(2)).max(0.0).sqrt()
                }
            };
            let slices = (0..count)
                .map(|i| {
                    let (left, right) = (from + i as f32 * step, from + (i + 1) as f32 * step);
                    let middle = (left + right) * 0.5;
                    let weight =
                        0.5 + 0.5 * ((middle - centre) / half * std::f32::consts::PI).cos();
                    let alpha = (f32::from(highlight.0[3]) * weight).round() as u8;
                    let cut = inset(middle);
                    Paint::Shape(
                        Shape::rect(
                            Rect::from_min_max(
                                Vec2::new(left, rect.min.y + cut),
                                Vec2::new(right, rect.max.y - cut),
                            ),
                            Color([highlight.0[0], highlight.0[1], highlight.0[2], alpha]),
                        )
                        .into(),
                    )
                })
                .collect();
            ui.context
                .paint(id.with("band"), ui.window, ui.clip.intersect(rect), slices);
        }
        ui.response(id, rect, false)
    }
}

impl Ui<'_> {
    /// Full-width loading placeholder of the given height.
    #[track_caller]
    pub fn skeleton(&mut self, height: f32) -> Response {
        self.add(Skeleton::new(height))
    }
}
