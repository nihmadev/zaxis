use crate::{Rect, Vec2};

/// Uniform positive scale and translation in logical pixels. Layout coordinates
/// remain unchanged; paint, clip and input use the same mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub scale: f32,
    pub translation: Vec2,
    pub angle: f32,
}
impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}
impl Transform {
    pub const IDENTITY: Self = Self {
        scale: 1.0,
        translation: Vec2::ZERO,
        angle: 0.0,
    };
    pub fn translation(delta: Vec2) -> Self {
        Self {
            translation: delta,
            ..Self::IDENTITY
        }
    }
    pub fn around(pivot: Vec2, scale: f32, offset: Vec2) -> Self {
        assert!(scale.is_finite() && scale > 0.0 && pivot.is_finite() && offset.is_finite());
        Self {
            scale,
            translation: pivot * (1.0 - scale) + offset,
            angle: 0.0,
        }
    }
    pub fn rotation(pivot: Vec2, angle: f32) -> Self {
        assert!(pivot.is_finite() && angle.is_finite());
        let mut t = Self {
            angle,
            ..Self::IDENTITY
        };
        t.translation = pivot - t.vector(pivot);
        t
    }
    pub fn vector(self, point: Vec2) -> Vec2 {
        if self.angle == 0.0 {
            return point * self.scale;
        }
        let (s, c) = self.angle.sin_cos();
        Vec2::new(c * point.x - s * point.y, s * point.x + c * point.y) * self.scale
    }
    pub fn point(self, point: Vec2) -> Vec2 {
        self.vector(point) + self.translation
    }
    pub fn rect(self, rect: Rect) -> Rect {
        let points = [
            rect.min,
            rect.max,
            Vec2::new(rect.min.x, rect.max.y),
            Vec2::new(rect.max.x, rect.min.y),
        ]
        .map(|p| self.point(p));
        Rect::from_min_max(
            points
                .into_iter()
                .fold(Vec2::splat(f32::INFINITY), Vec2::min),
            points
                .into_iter()
                .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max),
        )
    }
    pub fn inverse(self) -> Self {
        let mut inverse = Self {
            scale: 1.0 / self.scale,
            translation: Vec2::ZERO,
            angle: -self.angle,
        };
        inverse.translation = -inverse.vector(self.translation);
        inverse
    }
    /// `self` after `inner`.
    pub fn compose(self, inner: Self) -> Self {
        Self {
            scale: self.scale * inner.scale,
            translation: self.point(inner.translation),
            angle: self.angle + inner.angle,
        }
    }
}
