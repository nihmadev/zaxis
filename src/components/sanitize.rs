//! Normalization of out-of-range builder and style values.
//!
//! One rule for every component: a bad argument never panics. It is replaced
//! by the nearest valid value, or ignored (the property keeps its previous
//! value or the theme default) when no nearest value exists, and the problem is
//! recorded once per call site as a `DiagnosticKind::InvalidValue` diagnostic
//! (see `Context::diagnostics`). Nothing is printed. `assert!` remains only for
//! violations of the library's own invariants.
//!
//! * lengths that must be positive (width, height, font size, step, scale):
//!   anything not finite and `> 0` is ignored;
//! * lengths that may be zero (gaps, padding, radius, thickness): negative
//!   becomes `0`, NaN and infinity are ignored;
//! * positions and angles: NaN and infinity are ignored;
//! * weights: anything not finite and `> 0` becomes `1`;
//! * fractions: clamped to `0..=1`, NaN becomes `0`.

use crate::{context::invalid_value, Rect, Vec2};

#[track_caller]
fn report(what: &'static str, need: &str, value: impl std::fmt::Display, using: &str) {
    invalid_value(what, format!("expected {need}, got {value}; {using}"));
}

/// Finite and strictly positive, or ignored.
#[track_caller]
pub(crate) fn positive(what: &'static str, value: f32) -> Option<f32> {
    if value.is_finite() && value > 0.0 {
        return Some(value);
    }
    report(what, "a finite value > 0", value, "ignored");
    None
}

/// Finite and non-negative; negative becomes zero, NaN and infinity are ignored.
#[track_caller]
pub(crate) fn non_negative(what: &'static str, value: f32) -> Option<f32> {
    if value.is_finite() && value >= 0.0 {
        return Some(value);
    }
    if value.is_finite() {
        report(what, "a value >= 0", value, "using 0");
        return Some(0.0);
    }
    report(what, "a finite value", value, "ignored");
    None
}

/// Like [`non_negative`] for values that are always needed: invalid becomes zero.
#[track_caller]
pub(crate) fn length(what: &'static str, value: f32) -> f32 {
    non_negative(what, value).unwrap_or(0.0)
}

#[track_caller]
pub(crate) fn finite(what: &'static str, value: f32) -> Option<f32> {
    if value.is_finite() {
        return Some(value);
    }
    report(what, "a finite value", value, "ignored");
    None
}

/// A positive weight; anything else counts as `1`.
#[track_caller]
pub(crate) fn weight(what: &'static str, value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        return value;
    }
    report(what, "a finite weight > 0", value, "using 1");
    1.0
}

/// A fraction in `0..=1`; NaN becomes zero, other values are clamped.
#[track_caller]
pub(crate) fn unit(what: &'static str, value: f32) -> f32 {
    if value.is_nan() {
        report(what, "a number in 0..=1", value, "using 0");
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

/// Both components finite and non-negative; negative components become zero.
#[track_caller]
pub(crate) fn size(what: &'static str, value: Vec2) -> Option<Vec2> {
    if !value.is_finite() {
        report(what, "a finite size", value, "ignored");
        return None;
    }
    if value.min_element() < 0.0 {
        report(what, "a size >= 0", value, "negative parts become 0");
    }
    Some(value.max(Vec2::ZERO))
}

#[track_caller]
pub(crate) fn finite_vec2(what: &'static str, value: Vec2) -> Option<Vec2> {
    if value.is_finite() {
        return Some(value);
    }
    report(what, "a finite vector", value, "ignored");
    None
}

#[track_caller]
pub(crate) fn finite_rect(what: &'static str, value: Rect) -> Option<Rect> {
    if value.is_finite() {
        return Some(value);
    }
    report(what, "a finite rectangle", format!("{value:?}"), "ignored");
    None
}

/// Repeating or reversing motion cannot drive presence, reveal or highlight
/// release; they are forced to a single forward transition.
#[track_caller]
pub(crate) fn forward(what: &'static str, mut motion: crate::TweenOptions) -> crate::TweenOptions {
    if motion.repeat != crate::Repeat::Once || motion.auto_reverse {
        report(
            what,
            "a single forward transition",
            "a repeating or reversing one",
            "playing it once",
        );
        motion.repeat = crate::Repeat::Once;
        motion.auto_reverse = false;
    }
    motion
}
