//! Layout primitives. All UI distances are logical pixels (points).

use crate::{Rect, Vec2};

/// Cross-axis alignment in rows, columns, and Grid/Table cells.
/// Oversized content stays at the leading edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

impl Align {
    pub(crate) fn offset(self, spare: f32) -> f32 {
        spare.max(0.0)
            * match self {
                Self::Start => 0.0,
                Self::Center => 0.5,
                Self::End => 1.0,
            }
    }
}

impl Layout {
    pub(crate) fn main(self, size: Vec2) -> f32 {
        match self {
            Self::Horizontal => size.x,
            Self::Vertical => size.y,
        }
    }
    pub(crate) fn cross(self, size: Vec2) -> f32 {
        match self {
            Self::Horizontal => size.y,
            Self::Vertical => size.x,
        }
    }
    pub(crate) fn size(self, main: f32, cross: f32) -> Vec2 {
        match self {
            Self::Horizontal => Vec2::new(main, cross),
            Self::Vertical => Vec2::new(cross, main),
        }
    }
}

/// Insets for a rectangle, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Padding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Padding {
    pub const fn all(value: f32) -> Self {
        Self::symmetric(value, value)
    }

    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    pub fn size(self) -> Vec2 {
        Vec2::new(
            self.left.max(0.0) + self.right.max(0.0),
            self.top.max(0.0) + self.bottom.max(0.0),
        )
    }

    pub fn inset(self, rect: Rect) -> Rect {
        Rect::from_min_max(
            rect.min + Vec2::new(self.left.max(0.0), self.top.max(0.0)),
            rect.max - Vec2::new(self.right.max(0.0), self.bottom.max(0.0)),
        )
    }
}

/// The direction used when allocating widgets in a [`crate::Ui`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    Vertical,
    Horizontal,
}

pub(crate) struct LayoutCursor {
    pub bounds: Rect,
    pub cursor: Vec2,
    pub used: Vec2,
    pub direction: Layout,
    pub spacing: f32,
    pub preferred_width: Option<f32>,
}

impl LayoutCursor {
    pub fn new(bounds: Rect, direction: Layout, spacing: f32) -> Self {
        Self {
            bounds,
            cursor: bounds.min,
            used: Vec2::ZERO,
            direction,
            spacing,
            preferred_width: None,
        }
    }

    pub fn available_width(&self) -> f32 {
        (self.bounds.max.x - self.cursor.x).max(0.0)
    }

    pub fn allocate(&mut self, size: Vec2) -> Rect {
        let size = size.max(Vec2::ZERO);
        let rect = Rect::from_min_size(self.cursor, size);
        self.used = self.used.max(rect.max - self.bounds.min);
        match self.direction {
            Layout::Vertical => self.cursor.y += size.y + self.spacing,
            Layout::Horizontal => self.cursor.x += size.x + self.spacing,
        }
        rect
    }

    pub fn space(&mut self, amount: f32) {
        match self.direction {
            Layout::Vertical => self.cursor.y += amount.max(0.0),
            Layout::Horizontal => self.cursor.x += amount.max(0.0),
        }
        self.used = self.used.max(self.cursor - self.bounds.min);
    }
}
