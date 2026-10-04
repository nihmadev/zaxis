use super::curves::{self, Family, Mode};
use std::{fmt, sync::Arc};

/// Built-in curves or an application function. Inputs are clamped to [0, 1];
/// custom outputs must be finite and may overshoot. Endpoints are always exact.
///
/// `Back` and `Elastic` overshoot [0, 1] by design; `CubicBezier` with control
/// ordinates outside [0, 1] does too. Drawing code should allow for that.
#[derive(Clone)]
pub enum Easing {
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    QuintIn,
    QuintOut,
    QuintInOut,
    SineIn,
    SineOut,
    SineInOut,
    ExpoIn,
    ExpoOut,
    ExpoInOut,
    CircIn,
    CircOut,
    CircInOut,
    BackIn,
    BackOut,
    BackInOut,
    ElasticIn,
    ElasticOut,
    ElasticInOut,
    BounceIn,
    BounceOut,
    BounceInOut,
    /// CSS `cubic-bezier(x1, y1, x2, y2)`. Create with [`Easing::cubic_bezier`].
    CubicBezier {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    /// `n` equal jumps, the jump at the end of each interval. See [`Easing::steps`].
    Steps(u32),
    Custom(Arc<dyn Fn(f32) -> f32 + Send + Sync>),
}

impl Easing {
    pub fn custom(curve: impl Fn(f32) -> f32 + Send + Sync + 'static) -> Self {
        Self::Custom(Arc::new(curve))
    }
    /// The CSS timing function. `x1` and `x2` must be finite and within [0, 1],
    /// which keeps time monotonic; `y1` and `y2` may overshoot.
    pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        assert!(
            [x1, y1, x2, y2].iter().all(|v| v.is_finite())
                && (0.0..=1.0).contains(&x1)
                && (0.0..=1.0).contains(&x2),
            "cubic-bezier needs finite values and x1, x2 within [0, 1]"
        );
        Self::CubicBezier { x1, y1, x2, y2 }
    }
    /// CSS `ease`.
    pub fn ease() -> Self {
        Self::cubic_bezier(0.25, 0.1, 0.25, 1.0)
    }
    /// CSS `ease-in-out`.
    pub fn ease_in_out() -> Self {
        Self::cubic_bezier(0.42, 0.0, 0.58, 1.0)
    }
    pub fn steps(count: u32) -> Self {
        assert!(count > 0, "steps needs at least one step");
        Self::Steps(count)
    }
    pub fn sample(&self, progress: f32) -> f32 {
        assert!(progress.is_finite(), "easing progress must be finite");
        let t = progress.clamp(0.0, 1.0);
        if t == 0.0 || t == 1.0 {
            return t;
        }
        let power = |n: i32, mode: u8| match mode {
            0 => t.powi(n),
            1 => 1.0 - (1.0 - t).powi(n),
            _ if t < 0.5 => (2.0 * t).powi(n) * 0.5,
            _ => 1.0 - (2.0 * (1.0 - t)).powi(n) * 0.5,
        };
        let family = |family, mode| curves::apply(family, mode, t);
        let value = match self {
            Self::Linear => t,
            Self::QuadIn => power(2, 0),
            Self::QuadOut => power(2, 1),
            Self::QuadInOut => power(2, 2),
            Self::CubicIn => power(3, 0),
            Self::CubicOut => power(3, 1),
            Self::CubicInOut => power(3, 2),
            Self::QuintIn => power(5, 0),
            Self::QuintOut => power(5, 1),
            Self::QuintInOut => power(5, 2),
            Self::SineIn => 1.0 - (t * std::f32::consts::FRAC_PI_2).cos(),
            Self::SineOut => (t * std::f32::consts::FRAC_PI_2).sin(),
            Self::SineInOut => (1.0 - (t * std::f32::consts::PI).cos()) * 0.5,
            Self::ExpoIn => family(Family::Expo, Mode::In),
            Self::ExpoOut => family(Family::Expo, Mode::Out),
            Self::ExpoInOut => family(Family::Expo, Mode::InOut),
            Self::CircIn => family(Family::Circ, Mode::In),
            Self::CircOut => family(Family::Circ, Mode::Out),
            Self::CircInOut => family(Family::Circ, Mode::InOut),
            Self::BackIn => family(Family::Back, Mode::In),
            Self::BackOut => family(Family::Back, Mode::Out),
            Self::BackInOut => family(Family::Back, Mode::InOut),
            Self::ElasticIn => family(Family::Elastic, Mode::In),
            Self::ElasticOut => family(Family::Elastic, Mode::Out),
            Self::ElasticInOut => family(Family::Elastic, Mode::InOut),
            Self::BounceIn => family(Family::Bounce, Mode::In),
            Self::BounceOut => family(Family::Bounce, Mode::Out),
            Self::BounceInOut => family(Family::Bounce, Mode::InOut),
            Self::CubicBezier { x1, y1, x2, y2 } => curves::cubic_bezier(*x1, *y1, *x2, *y2, t),
            Self::Steps(count) => curves::steps(*count, t),
            Self::Custom(curve) => curve(t),
        };
        assert!(
            value.is_finite(),
            "custom easing must return a finite value"
        );
        value
    }
}

impl PartialEq for Easing {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(a), Self::Custom(b)) => Arc::ptr_eq(a, b),
            (
                Self::CubicBezier { x1, y1, x2, y2 },
                Self::CubicBezier {
                    x1: a,
                    y1: b,
                    x2: c,
                    y2: d,
                },
            ) => (x1, y1, x2, y2) == (a, b, c, d),
            (Self::Steps(a), Self::Steps(b)) => a == b,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}
impl fmt::Debug for Easing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Linear => "Linear",
            Self::QuadIn => "QuadIn",
            Self::QuadOut => "QuadOut",
            Self::QuadInOut => "QuadInOut",
            Self::CubicIn => "CubicIn",
            Self::CubicOut => "CubicOut",
            Self::CubicInOut => "CubicInOut",
            Self::QuintIn => "QuintIn",
            Self::QuintOut => "QuintOut",
            Self::QuintInOut => "QuintInOut",
            Self::SineIn => "SineIn",
            Self::SineOut => "SineOut",
            Self::SineInOut => "SineInOut",
            Self::ExpoIn => "ExpoIn",
            Self::ExpoOut => "ExpoOut",
            Self::ExpoInOut => "ExpoInOut",
            Self::CircIn => "CircIn",
            Self::CircOut => "CircOut",
            Self::CircInOut => "CircInOut",
            Self::BackIn => "BackIn",
            Self::BackOut => "BackOut",
            Self::BackInOut => "BackInOut",
            Self::ElasticIn => "ElasticIn",
            Self::ElasticOut => "ElasticOut",
            Self::ElasticInOut => "ElasticInOut",
            Self::BounceIn => "BounceIn",
            Self::BounceOut => "BounceOut",
            Self::BounceInOut => "BounceInOut",
            Self::CubicBezier { x1, y1, x2, y2 } => {
                return write!(f, "CubicBezier({x1}, {y1}, {x2}, {y2})");
            }
            Self::Steps(count) => return write!(f, "Steps({count})"),
            Self::Custom(_) => "Custom",
        };
        f.write_str(name)
    }
}
