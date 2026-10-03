use super::Color;

/// An inset border. A zero width disables it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    pub width: f32,
    pub color: Color,
}

impl Border {
    pub const NONE: Self = Self {
        width: 0.0,
        color: Color::TRANSPARENT,
    };
    pub const fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }
}
