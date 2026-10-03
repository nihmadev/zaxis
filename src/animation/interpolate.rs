use crate::{Border, Color, CornerRadius, Gradient, Padding, Rect, Shadow, Vec2};

/// Implement this for application value types; no engine changes are needed.
/// Curves can overshoot, so `t` can be outside [0, 1]. Tracks clone endpoints
/// directly at their boundaries, independent of interpolation rounding.
pub trait Interpolate: Clone + PartialEq + 'static {
    fn interpolate(&self, to: &Self, t: f32) -> Self;
}

impl Interpolate for f32 {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        self + (to - self) * t
    }
}
impl Interpolate for f64 {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        self + (to - self) * f64::from(t)
    }
}
impl Interpolate for Vec2 {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        self.lerp(*to, t)
    }
}
/// RGB is interpolated in linear-light sRGB, alpha linearly in straight-alpha
/// space, then RGB is encoded back to sRGB and rounded to the nearest byte.
impl Interpolate for Color {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        if t == 0.0 {
            return *self;
        }
        if t == 1.0 {
            return *to;
        }
        let from = self.linear();
        let to = to.linear();
        let mut rgba = [0; 4];
        for i in 0..4 {
            let value = from[i].interpolate(&to[i], t).clamp(0.0, 1.0);
            let encoded = if i == 3 {
                value
            } else if value <= 0.0031308 {
                value * 12.92
            } else {
                1.055 * value.powf(1.0 / 2.4) - 0.055
            };
            rgba[i] = (encoded * 255.0).round() as u8;
        }
        Self(rgba)
    }
}

macro_rules! fields {
    ($ty:ty, $($field:ident),+ $(,)?) => {
        impl Interpolate for $ty {
            fn interpolate(&self, to: &Self, t: f32) -> Self {
                Self { $($field: self.$field.interpolate(&to.$field, t)),+ }
            }
        }
    };
}
fields!(Border, width, color);
fields!(CornerRadius, top_left, top_right, bottom_left, bottom_right);
fields!(Shadow, color, offset, blur_radius, spread);
fields!(Rect, min, max);
fields!(Padding, left, right, top, bottom);

impl Interpolate for Gradient {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        Self {
            start: self.start.interpolate(&to.start, t),
            end: self.end.interpolate(&to.end, t),
            // Direction is discrete; retain it until reaching the target.
            direction: if t >= 1.0 {
                to.direction
            } else {
                self.direction
            },
        }
    }
}
impl<T: Interpolate, const N: usize> Interpolate for [T; N] {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        std::array::from_fn(|i| self[i].interpolate(&to[i], t))
    }
}
impl<A: Interpolate, B: Interpolate> Interpolate for (A, B) {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        (self.0.interpolate(&to.0, t), self.1.interpolate(&to.1, t))
    }
}
