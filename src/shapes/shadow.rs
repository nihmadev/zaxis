use super::Color;
use crate::Vec2;

/// A soft outer shadow, measured in logical pixels. No backdrop filtering is needed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub color: Color,
    pub offset: Vec2,
    pub blur_radius: f32,
    pub spread: f32,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            color: Color::rgba(0, 0, 0, 100),
            offset: Vec2::new(0.0, 3.0),
            blur_radius: 6.0,
            spread: 0.0,
        }
    }
}
