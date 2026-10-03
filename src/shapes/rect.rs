use crate::Vec2;

/// An axis-aligned rectangle in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self {
            min,
            max: max.max(min),
        }
    }

    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self::from_min_max(min, min + size.max(Vec2::ZERO))
    }

    pub fn size(self) -> Vec2 {
        self.max - self.min
    }
    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
    pub fn is_empty(self) -> bool {
        self.max.x <= self.min.x || self.max.y <= self.min.y
    }

    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.x < self.max.x
            && point.y < self.max.y
    }

    pub fn intersect(self, other: Self) -> Self {
        Self::from_min_max(self.min.max(other.min), self.max.min(other.max))
    }

    pub fn shrink(self, amount: f32) -> Self {
        let amount = amount.max(0.0).min(self.size().min_element() * 0.5);
        Self::from_min_max(
            self.min + Vec2::splat(amount),
            self.max - Vec2::splat(amount),
        )
    }

    pub fn translate(self, delta: Vec2) -> Self {
        Self {
            min: self.min + delta,
            max: self.max + delta,
        }
    }
}
