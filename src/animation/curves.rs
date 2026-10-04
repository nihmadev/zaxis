//! Shapes behind the expressive easing families. Every family is defined by its
//! "out" curve on [0, 1]; "in" and "in-out" are its mirrored forms, so all three
//! agree on 0 -> 0, 1 -> 1 and meet at exactly 0.5 in the middle.
use std::f32::consts::PI;

#[derive(Clone, Copy)]
pub(super) enum Family {
    Expo,
    Circ,
    Back,
    Elastic,
    Bounce,
}

#[derive(Clone, Copy)]
pub(super) enum Mode {
    In,
    Out,
    InOut,
}

const BACK: f32 = 1.70158;

fn out(family: Family, t: f32) -> f32 {
    match family {
        // Normalised so the curve reaches exactly 1 at t = 1 without a jump.
        Family::Expo => (1.0 - 2.0_f32.powf(-10.0 * t)) / (1.0 - 2.0_f32.powi(-10)),
        Family::Circ => (1.0 - (t - 1.0) * (t - 1.0)).sqrt(),
        Family::Back => {
            let u = t - 1.0;
            1.0 + (BACK + 1.0) * u * u * u + BACK * u * u
        }
        Family::Elastic => {
            2.0_f32.powf(-10.0 * t) * ((10.0 * t - 0.75) * 2.0 * PI / 3.0).sin() + 1.0
        }
        Family::Bounce => {
            const N: f32 = 7.5625;
            const D: f32 = 2.75;
            if t < 1.0 / D {
                N * t * t
            } else if t < 2.0 / D {
                let u = t - 1.5 / D;
                N * u * u + 0.75
            } else if t < 2.5 / D {
                let u = t - 2.25 / D;
                N * u * u + 0.9375
            } else {
                let u = t - 2.625 / D;
                N * u * u + 0.984375
            }
        }
    }
}

pub(super) fn apply(family: Family, mode: Mode, t: f32) -> f32 {
    match mode {
        Mode::Out => out(family, t),
        Mode::In => 1.0 - out(family, 1.0 - t),
        Mode::InOut if t < 0.5 => (1.0 - out(family, 1.0 - 2.0 * t)) * 0.5,
        Mode::InOut => (1.0 + out(family, 2.0 * t - 1.0)) * 0.5,
    }
}

/// CSS `cubic-bezier(x1, y1, x2, y2)`: control points (0,0), (x1,y1), (x2,y2),
/// (1,1). `x` is solved by bisection, which is monotonic and needs no derivative.
pub(super) fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let at = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    };
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..32 {
        let mid = (low + high) * 0.5;
        if at(x1, x2, mid) < x {
            low = mid;
        } else {
            high = mid;
        }
    }
    at(y1, y2, (low + high) * 0.5)
}

/// `steps(n)` with the jump at the end of each interval.
pub(super) fn steps(count: u32, t: f32) -> f32 {
    let n = count as f32;
    ((t * n).floor() / n).min(1.0)
}
