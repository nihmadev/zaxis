use std::{fmt, sync::Arc};

/// Built-in curves or an application function. Inputs are clamped to [0, 1];
/// custom outputs must be finite and may overshoot. Endpoints are always exact.
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
    Custom(Arc<dyn Fn(f32) -> f32 + Send + Sync>),
}

impl Easing {
    pub fn custom(curve: impl Fn(f32) -> f32 + Send + Sync + 'static) -> Self {
        Self::Custom(Arc::new(curve))
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
            Self::Custom(_) => "Custom",
        };
        f.write_str(name)
    }
}
