use super::Color;

/// Direction of a two-color linear gradient.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GradientDirection {
    #[default]
    Horizontal,
    Vertical,
    Diagonal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gradient {
    pub start: Color,
    pub end: Color,
    pub direction: GradientDirection,
}

impl Gradient {
    pub const fn new(start: Color, end: Color) -> Self {
        Self {
            start,
            end,
            direction: GradientDirection::Horizontal,
        }
    }

    pub const fn direction(mut self, direction: GradientDirection) -> Self {
        self.direction = direction;
        self
    }
}
