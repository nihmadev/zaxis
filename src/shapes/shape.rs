use super::{Border, Color, CornerRadius, Gradient, Rect, Shadow};
use crate::Vec2;

/// Built-in vector shapes, tessellated only when their cached description changes.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// Rounded stroke with a two-color gradient projected at `angle` radians.
    /// Uses the ordinary antialiased contour; no material or shader is required.
    GradientBorder {
        rect: Rect,
        rounding: CornerRadius,
        width: f32,
        gradient: Gradient,
        angle: f32,
    },
    Gradient {
        rect: Rect,
        rounding: CornerRadius,
        gradient: Gradient,
    },
    Shadow {
        rect: Rect,
        rounding: CornerRadius,
        shadow: Shadow,
    },
    Rect {
        rect: Rect,
        fill: Color,
        rounding: CornerRadius,
        border: Border,
    },
    Circle {
        center: Vec2,
        radius: f32,
        fill: Color,
        border: Border,
    },
    Line {
        start: Vec2,
        end: Vec2,
        width: f32,
        color: Color,
    },
}

/// Rectangle builder. Accepted directly by [`crate::Ui::paint`].
#[derive(Clone, Debug, PartialEq)]
pub struct RectShape {
    rect: Rect,
    fill: Color,
    corner_radius: CornerRadius,
    border: Border,
}

impl Shape {
    pub fn rect(rect: Rect, fill: Color) -> RectShape {
        RectShape {
            rect,
            fill,
            corner_radius: CornerRadius::ZERO,
            border: Border::NONE,
        }
    }
}

impl RectShape {
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius = radius.into();
        self
    }
    pub fn border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }
}

impl From<RectShape> for Shape {
    fn from(shape: RectShape) -> Self {
        Self::Rect {
            rect: shape.rect,
            fill: shape.fill,
            rounding: shape.corner_radius,
            border: shape.border,
        }
    }
}
