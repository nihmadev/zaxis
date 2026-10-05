use crate::{Border, Color, CornerRadius, Gradient, Padding, Rect, Shadow, Transform, Vec2};

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
            let encoded = if i == 3 { value } else { encode_srgb(value) };
            rgba[i] = (encoded * 255.0).round() as u8;
        }
        Self(rgba)
    }
}

fn encode_srgb(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// A color interpolated in the perceptual Oklab space. Plain `Color` blends in
/// linear light, which keeps brightness physically right but makes the middle of
/// saturated pairs (blue to yellow) grey and black-to-white too bright; Oklab
/// keeps hue and perceived lightness steady. Endpoints are exact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklab(pub Color);

// The conversion matrices are quoted as published (Björn Ottosson, "A perceptual color
// space for image processing"); f32 rounds the extra digits.
#[allow(clippy::excessive_precision)]
impl Oklab {
    fn lab(color: Color) -> [f32; 3] {
        let [r, g, b, _] = color.linear();
        let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
        let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
        let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
        [
            0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
            1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
            0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
        ]
    }
    fn color(lab: [f32; 3], alpha: f32) -> Color {
        let [lightness, a, b] = lab;
        let l = (lightness + 0.3963377774 * a + 0.2158037573 * b).powi(3);
        let m = (lightness - 0.1055613458 * a - 0.0638541728 * b).powi(3);
        let s = (lightness - 0.0894841775 * a - 1.2914855480 * b).powi(3);
        let linear = [
            4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
            -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
            -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
        ];
        let byte = |v: f32| (encode_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8;
        Color([
            byte(linear[0]),
            byte(linear[1]),
            byte(linear[2]),
            (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
        ])
    }
}
impl Interpolate for Oklab {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        if t == 0.0 {
            return *self;
        }
        if t == 1.0 {
            return *to;
        }
        let (from_lab, to_lab) = (Self::lab(self.0), Self::lab(to.0));
        let lab = from_lab.interpolate(&to_lab, t);
        let alpha = f32::from(self.0 .0[3]) / 255.0;
        let alpha = alpha.interpolate(&(f32::from(to.0 .0[3]) / 255.0), t);
        Self(Self::color(lab, alpha))
    }
}
impl From<Color> for Oklab {
    fn from(color: Color) -> Self {
        Self(color)
    }
}
impl From<Oklab> for Color {
    fn from(color: Oklab) -> Self {
        color.0
    }
}

/// Implement [`Interpolate`] for a struct by interpolating the named fields,
/// each of which must itself implement `Interpolate`.
///
/// ```
/// use zaxis::{impl_interpolate, Interpolate, Vec2};
/// #[derive(Clone, PartialEq)]
/// struct Pose { offset: Vec2, angle: f32 }
/// impl_interpolate!(Pose { offset, angle });
/// let half = Pose { offset: Vec2::ZERO, angle: 0.0 }
///     .interpolate(&Pose { offset: Vec2::new(10.0, 0.0), angle: 2.0 }, 0.5);
/// assert_eq!(half.angle, 1.0);
/// ```
#[macro_export]
macro_rules! impl_interpolate {
    ($ty:ty { $($field:ident),+ $(,)? }) => {
        impl $crate::Interpolate for $ty {
            fn interpolate(&self, to: &Self, t: f32) -> Self {
                Self {
                    $($field: $crate::Interpolate::interpolate(&self.$field, &to.$field, t)),+
                }
            }
        }
    };
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
fields!(Transform, scale, translation, angle);

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
impl<A: Interpolate, B: Interpolate, C: Interpolate> Interpolate for (A, B, C) {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        (
            self.0.interpolate(&to.0, t),
            self.1.interpolate(&to.1, t),
            self.2.interpolate(&to.2, t),
        )
    }
}
/// `Some` to `Some` interpolates the values. Appearing or disappearing is
/// discrete: the `from` state is kept until the end of the transition.
impl<T: Interpolate> Interpolate for Option<T> {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        match (self, to) {
            (Some(a), Some(b)) => Some(a.interpolate(b, t)),
            _ if t >= 1.0 => to.clone(),
            _ => self.clone(),
        }
    }
}
/// Element-wise for equal lengths; a changed length is discrete like `Option`.
impl<T: Interpolate> Interpolate for Vec<T> {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        if self.len() != to.len() {
            return if t >= 1.0 { to.clone() } else { self.clone() };
        }
        self.iter()
            .zip(to)
            .map(|(a, b)| a.interpolate(b, t))
            .collect()
    }
}
