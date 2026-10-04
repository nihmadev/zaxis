use super::{Animation, AnimationSample};
use crate::Vec2;
use std::time::Duration;

/// Linear vector operations, separate from interpolation. Position and velocity
/// share units; velocity is units/second. Norms must be finite and nonnegative.
pub trait SpringValue: Clone + PartialEq + 'static {
    fn zero() -> Self;
    fn add(&self, other: &Self) -> Self;
    fn sub(&self, other: &Self) -> Self;
    fn scale(&self, factor: f64) -> Self;
    fn norm(&self) -> f64;
}
macro_rules! scalar {
    ($t:ty) => {
        impl SpringValue for $t {
            fn zero() -> Self {
                0.0
            }
            fn add(&self, other: &Self) -> Self {
                *self + *other
            }
            fn sub(&self, other: &Self) -> Self {
                *self - *other
            }
            fn scale(&self, factor: f64) -> Self {
                (*self as f64 * factor) as Self
            }
            fn norm(&self) -> f64 {
                (*self as f64).abs()
            }
        }
    };
}
scalar!(f32);
scalar!(f64);
impl SpringValue for Vec2 {
    fn zero() -> Self {
        Self::ZERO
    }
    fn add(&self, other: &Self) -> Self {
        *self + *other
    }
    fn sub(&self, other: &Self) -> Self {
        *self - *other
    }
    fn scale(&self, factor: f64) -> Self {
        *self * factor as f32
    }
    fn norm(&self) -> f64 {
        (self.x as f64).hypot(self.y as f64)
    }
}

/// Implement [`SpringValue`] for a struct whose named fields are all
/// `SpringValue`s. The norm is the Euclidean norm of the field norms.
///
/// ```
/// use zaxis::{impl_spring_value, SpringValue, Vec2};
/// #[derive(Clone, PartialEq)]
/// struct Pose { offset: Vec2, angle: f32 }
/// impl_spring_value!(Pose { offset, angle });
/// assert_eq!(Pose::zero().angle, 0.0);
/// ```
#[macro_export]
macro_rules! impl_spring_value {
    ($ty:ty { $($field:ident),+ $(,)? }) => {
        impl $crate::SpringValue for $ty {
            fn zero() -> Self {
                Self { $($field: $crate::SpringValue::zero()),+ }
            }
            fn add(&self, other: &Self) -> Self {
                Self { $($field: $crate::SpringValue::add(&self.$field, &other.$field)),+ }
            }
            fn sub(&self, other: &Self) -> Self {
                Self { $($field: $crate::SpringValue::sub(&self.$field, &other.$field)),+ }
            }
            fn scale(&self, factor: f64) -> Self {
                Self { $($field: $crate::SpringValue::scale(&self.$field, factor)),+ }
            }
            fn norm(&self) -> f64 {
                (0.0_f64 $(+ $crate::SpringValue::norm(&self.$field).powi(2))+).sqrt()
            }
        }
    };
}

/// k*x + c*v + m*a = 0. Default is critical damping, without bounce.
/// k=0 snaps to target. Invalid/nonfinite parameters panic. c=0 oscillates
/// indefinitely unless initially at rest or explicitly finished/cancelled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringOptions {
    pub stiffness: f64,
    pub damping: f64,
    pub mass: f64,
    pub distance_threshold: f64,
    pub velocity_threshold: f64,
}
impl Default for SpringOptions {
    fn default() -> Self {
        Self::frequency(3.0, 1.0)
    }
}
impl SpringOptions {
    /// omega=2*pi*Hz, stiffness=mass*omega^2, damping=2*mass*omega*ratio.
    /// ratio < 1 oscillates, = 1 is critical, > 1 is overdamped.
    pub fn frequency(hz: f64, ratio: f64) -> Self {
        assert!(hz.is_finite() && hz >= 0.0 && ratio.is_finite() && ratio >= 0.0);
        let omega = std::f64::consts::TAU * hz;
        Self {
            stiffness: omega * omega,
            damping: 2.0 * omega * ratio,
            mass: 1.0,
            distance_threshold: 0.001,
            velocity_threshold: 0.001,
        }
    }
    /// SwiftUI-style: `duration` is the period of one undamped oscillation (how
    /// long the spring takes to reach its target the first time, roughly a
    /// quarter of the way through the cycle later), `bounce` runs from -1
    /// (heavily overdamped) through 0 (critical, no overshoot) to 1 (never
    /// settles). Typical bouncy springs use 0.15 to 0.4.
    pub fn duration_bounce(duration: Duration, bounce: f64) -> Self {
        assert!(!duration.is_zero(), "spring duration must be positive");
        assert!(
            bounce.is_finite() && bounce > -1.0 && bounce < 1.0,
            "spring bounce must be within (-1, 1)"
        );
        let ratio = if bounce >= 0.0 {
            1.0 - bounce
        } else {
            1.0 / (1.0 + bounce)
        };
        Self::frequency(1.0 / duration.as_secs_f64(), ratio)
    }
    pub fn thresholds(mut self, distance: f64, velocity: f64) -> Self {
        self.distance_threshold = distance;
        self.velocity_threshold = velocity;
        self.validate();
        self
    }
    fn validate(self) {
        assert!(
            self.stiffness.is_finite()
                && self.stiffness >= 0.0
                && self.damping.is_finite()
                && self.damping >= 0.0
                && self.mass.is_finite()
                && self.mass > 0.0
                && self.distance_threshold.is_finite()
                && self.distance_threshold > 0.0
                && self.velocity_threshold.is_finite()
                && self.velocity_threshold > 0.0,
            "spring needs finite nonnegative k/c, positive mass and thresholds"
        );
        assert!(
            (self.stiffness / self.mass).is_finite()
                && (self.damping / self.mass).powi(2).is_finite(),
            "spring ratios overflow"
        );
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpringState<T> {
    pub value: T,
    pub velocity: T,
}

/// Analytic solution, O(1) per sample even after suspension. No integrator.
pub struct Spring<T> {
    from: T,
    target: T,
    velocity: T,
    options: SpringOptions,
}
impl<T: SpringValue> Spring<T> {
    pub fn new(from: T, target: T) -> Self {
        Self::with_options(from, target, SpringOptions::default())
    }
    pub fn with_options(from: T, target: T, options: SpringOptions) -> Self {
        options.validate();
        assert!(from.norm().is_finite() && target.norm().is_finite());
        Self {
            from,
            target,
            velocity: T::zero(),
            options,
        }
    }
    pub fn velocity(mut self, velocity: T) -> Self {
        assert!(velocity.norm().is_finite());
        self.velocity = velocity;
        self
    }
    pub fn state(&self, elapsed: Duration) -> AnimationSample<SpringState<T>> {
        let o = self.options;
        if o.stiffness == 0.0 {
            return AnimationSample::completed(self.rest());
        }
        let t = elapsed.as_secs_f64();
        let w2 = o.stiffness / o.mass;
        let a = o.damping / o.mass / 2.0;
        let x = self.from.sub(&self.target);
        let v = &self.velocity;
        if elapsed.is_zero() {
            return if x.norm() <= o.distance_threshold && v.norm() <= o.velocity_threshold {
                AnimationSample::completed(self.rest())
            } else {
                AnimationSample::running(SpringState {
                    value: self.from.clone(),
                    velocity: v.clone(),
                })
            };
        }
        let d = a * a - w2;
        // Fundamental matrix coefficients avoid component-specific arithmetic.
        let (xx, xv, vx, vv) = if d.abs() <= w2 * 1e-10 {
            let e = (-a * t).exp();
            if e == 0.0 {
                (0.0, 0.0, 0.0, 0.0)
            } else {
                (e * (1.0 + a * t), e * t, -e * w2 * t, e * (1.0 - a * t))
            }
        } else if d < 0.0 {
            let w = (-d).sqrt();
            let e = (-a * t).exp();
            let s = (w * t).sin() / w;
            let c = (w * t).cos();
            (e * (c + a * s), e * s, -e * w2 * s, e * (c - a * s))
        } else {
            let b = d.sqrt();
            // Stable slow root: -a+b suffers cancellation for large damping.
            let r1 = -w2 / (a + b);
            let r2 = -a - b;
            let e1 = (r1 * t).exp();
            let e2 = (r2 * t).exp();
            let s = (e1 - e2) / (2.0 * b);
            (
                (-r2 * e1 + r1 * e2) / (2.0 * b),
                s,
                -w2 * s,
                (r1 * e1 - r2 * e2) / (2.0 * b),
            )
        };
        let displacement = x.scale(xx).add(&v.scale(xv));
        let velocity = x.scale(vx).add(&v.scale(vv));
        if displacement.norm() <= o.distance_threshold && velocity.norm() <= o.velocity_threshold {
            AnimationSample::completed(self.rest())
        } else {
            AnimationSample::running(SpringState {
                value: self.target.add(&displacement),
                velocity,
            })
        }
    }
    fn rest(&self) -> SpringState<T> {
        SpringState {
            value: self.target.clone(),
            velocity: T::zero(),
        }
    }
}
impl<T: SpringValue> Animation<SpringState<T>> for Spring<T> {
    fn sample(&self, elapsed: Duration) -> AnimationSample<SpringState<T>> {
        self.state(elapsed)
    }
    fn finish(&self) -> SpringState<T> {
        self.rest()
    }
}
