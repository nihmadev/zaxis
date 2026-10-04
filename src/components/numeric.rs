//! Arithmetic for numeric controls. Integer values never pass through floating point.
use std::{fmt::Display, str::FromStr};

mod sealed {
    pub trait Sealed {}
}

/// Primitive integers and floats supported by [`super::NumberInput`], [`super::DragValue`]
/// and [`super::Slider`]. The step has the same type as the value; fractional steps are
/// only valid for floats.
///
/// Controls derive their defaults from the type: integers step by one and show no
/// decimals, so `Slider::new(&mut count, 0..=10)` needs no `.step(1).precision(0)`.
pub trait Numeric: sealed::Sealed + Copy + PartialEq + PartialOrd + Display + FromStr {
    const MIN: Self;
    const MAX: Self;
    const ONE: Self;
    /// Integers have a natural step of one and no fractional digits.
    const INTEGER: bool;
    /// Nearest `f64`; integers beyond 2^53 lose their low bits.
    fn to_f64(self) -> f64;
    /// Nearest value of this type, saturating; NaN becomes zero.
    fn from_f64(value: f64) -> Self;
    fn finite(self) -> bool;
    fn positive(self) -> bool;
    fn normalized(self, min: Self, max: Self) -> Self;
    fn offset(self, step: Self, units: f64, min: Self, max: Self) -> Self;
    fn formatted(self, precision: Option<usize>) -> String;
    fn same(self, other: Self) -> bool {
        self == other
    }
    fn parse(text: &str) -> Option<Self> {
        text.trim().parse().ok().filter(|v: &Self| v.finite())
    }
}

// Multiply a typed integer step by a gesture count without casting the step.
// A 192-bit product (three u64 limbs) covers u128 * the f64 mantissa.
fn integer_delta(step: u128, units: f64) -> u128 {
    let mut units = units.abs();
    let nearest = units.round();
    if (units - nearest).abs() <= f64::EPSILON * units.max(1.0) * 4.0 {
        units = nearest;
    }
    let whole = step.saturating_mul(units as u128);
    let fraction = units.fract();
    if fraction == 0.0 || !fraction.is_finite() {
        return whole;
    }
    let bits = fraction.to_bits();
    let exponent = (bits >> 52) & 0x7ff;
    let mantissa = u128::from((bits & ((1 << 52) - 1)) | if exponent > 0 { 1 << 52 } else { 0 });
    let shift = if exponent == 0 { 1074 } else { 1075 - exponent };
    if shift >= 192 {
        return whole;
    }
    let low = u128::from(step as u64) * mantissa;
    let high = (step >> 64) * mantissa + (low >> 64);
    let fractional = if shift >= 64 {
        high >> (shift - 64)
    } else {
        (high << (64 - shift)) | (u128::from(low as u64) >> shift)
    };
    let rounding = if shift > 64 {
        (high >> (shift - 65)) & 1
    } else {
        (low >> (shift - 1)) & 1
    };
    whole.saturating_add(fractional).saturating_add(rounding)
}

macro_rules! integers {
    ($($t:ty),*) => {$(
        impl sealed::Sealed for $t {}
        impl Numeric for $t {
            const MIN: Self = Self::MIN;
            const MAX: Self = Self::MAX;
            const ONE: Self = 1;
            const INTEGER: bool = true;
            fn to_f64(self) -> f64 { self as f64 }
            fn from_f64(value: f64) -> Self { value.round() as Self }
            fn finite(self) -> bool { true }
            fn positive(self) -> bool { self > 0 }
            fn normalized(self, min: Self, max: Self) -> Self { self.clamp(min, max) }
            fn offset(self, step: Self, units: f64, min: Self, max: Self) -> Self {
                let delta = integer_delta(step as u128, units);
                // Ordered unsigned representation covers the full signed range too.
                let origin = self.clamp(min, max).wrapping_sub(Self::MIN) as u128;
                let limit = Self::MAX.wrapping_sub(Self::MIN) as u128;
                let next = if units < 0.0 { origin.saturating_sub(delta) }
                    else { origin.saturating_add(delta).min(limit) };
                (next as Self).wrapping_add(Self::MIN).clamp(min, max)
            }
            fn formatted(self, _: Option<usize>) -> String { self.to_string() }
        }
    )*};
}
// For signed types the cast must preserve the unsigned bit pattern at that width.
macro_rules! signed {
    ($(($t:ty, $u:ty)),*) => {$(
        impl sealed::Sealed for $t {}
        impl Numeric for $t {
            const MIN: Self = Self::MIN;
            const MAX: Self = Self::MAX;
            const ONE: Self = 1;
            const INTEGER: bool = true;
            fn to_f64(self) -> f64 { self as f64 }
            fn from_f64(value: f64) -> Self { value.round() as Self }
            fn finite(self) -> bool { true }
            fn positive(self) -> bool { self > 0 }
            fn normalized(self, min: Self, max: Self) -> Self { self.clamp(min, max) }
            fn offset(self, step: Self, units: f64, min: Self, max: Self) -> Self {
                let delta = integer_delta(step as u128, units);
                let origin = self.clamp(min, max).wrapping_sub(Self::MIN) as $u as u128;
                let next = if units < 0.0 { origin.saturating_sub(delta) }
                    else { origin.saturating_add(delta).min(<$u>::MAX as u128) };
                (next as Self).wrapping_add(Self::MIN).clamp(min, max)
            }
            fn formatted(self, _: Option<usize>) -> String { self.to_string() }
        }
    )*};
}
integers!(u8, u16, u32, u64, u128, usize);
signed!(
    (i8, u8),
    (i16, u16),
    (i32, u32),
    (i64, u64),
    (i128, u128),
    (isize, usize)
);

fn decimal_places(step: &str) -> i32 {
    let (mantissa, exponent) = step.split_once(['e', 'E']).unwrap_or((step, "0"));
    let digits = mantissa
        .split_once('.')
        .map_or(0, |(_, tail)| tail.len() as i32);
    digits - exponent.parse::<i32>().unwrap_or(0)
}

macro_rules! floats {
    ($($t:ty),*) => {$(
        impl sealed::Sealed for $t {}
        impl Numeric for $t {
            const MIN: Self = -Self::MAX;
            const MAX: Self = Self::MAX;
            const ONE: Self = 1.0;
            const INTEGER: bool = false;
            fn to_f64(self) -> f64 { self as f64 }
            fn from_f64(value: f64) -> Self { value as Self }
            fn finite(self) -> bool { self.is_finite() }
            fn positive(self) -> bool { self.is_finite() && self > 0.0 }
            fn normalized(self, min: Self, max: Self) -> Self {
                if self.is_nan() { min } else { self.clamp(min, max) }
            }
            fn offset(self, step: Self, units: f64, min: Self, max: Self) -> Self {
                if units == 0.0 { return self.normalized(min, max); }
                let origin = self.normalized(min, max) as f64;
                let next = (step as f64).mul_add(units, origin);
                // Correct decimal step noise without rounding an off-grid origin.
                let places = decimal_places(&step.to_string());
                let scale = 10_f64.powi(places.clamp(-308, 308));
                let origin_scaled = origin * scale;
                let epsilon = <$t>::EPSILON as f64 * 4.0;
                let aligned = (origin_scaled - origin_scaled.round()).abs()
                    <= epsilon * origin_scaled.abs().max(1.0);
                let scaled = next * scale;
                let rounded = scaled.round();
                let next = if places.abs() <= 308 && aligned && scaled.is_finite()
                    && scaled.abs() < (1_u64 << 52) as f64
                    && (scaled - rounded).abs() <= epsilon * scaled.abs().max(1.0) {
                    rounded / scale
                } else { next };
                (next as Self).normalized(min, max)
            }
            fn formatted(self, precision: Option<usize>) -> String {
                precision.map_or_else(|| self.to_string(), |p| format!("{self:.p$}"))
            }
            fn same(self, other: Self) -> bool {
                self == other || (self.is_nan() && other.is_nan())
            }
        }
    )*};
}
floats!(f32, f64);
