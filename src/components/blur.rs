use super::Ui;
use crate::{context::Paint, Border, Color, CornerRadius, Rect, Shape};

/// Blur everything drawn behind a rectangle, then optionally apply a tint.
/// Radius is Gaussian sigma in logical pixels; zero disables the effect.
#[derive(Clone, Copy, Debug)]
pub struct Blur {
    pub(crate) rect: Rect,
    pub(crate) radius: f32,
    pub(crate) rounding: CornerRadius,
    tint: Color,
}

impl Blur {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            radius: 12.0,
            rounding: CornerRadius::ZERO,
            tint: Color::TRANSPARENT,
        }
    }
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = normalize_radius(radius);
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = radius.into();
        self
    }
    pub fn tint(mut self, color: Color) -> Self {
        self.tint = color;
        self
    }
    pub fn show(self, ui: &mut Ui<'_>) {
        let id = ui.next_id("blur");
        ui.context.paint_blur(id, ui.window, ui.clip, self);
        if self.tint.0[3] > 0 {
            ui.context.paint(
                id.with("tint"),
                ui.window,
                ui.clip,
                vec![Paint::Shape(Shape::Rect {
                    rect: self.rect,
                    fill: self.tint,
                    rounding: self.rounding,
                    border: Border::NONE,
                })],
            );
        }
    }
}

pub(crate) fn normalize_radius(radius: f32) -> f32 {
    if radius.is_finite() {
        radius.clamp(0.0, 64.0)
    } else {
        0.0
    }
}

impl Ui<'_> {
    pub fn blur(&mut self, rect: Rect, radius: f32) {
        Blur::new(rect).radius(radius).show(self);
    }
}
