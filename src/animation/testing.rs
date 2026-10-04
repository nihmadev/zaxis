//! Deterministic checks for application-defined [`Animation`] tracks.
//!
//! Tracks are pure functions of elapsed time, so these helpers need no window or
//! clock. For retained channels, drive [`crate::Context::run_at`] with explicit
//! instants instead.
use super::{Animation, AnimationSample};
use std::{fmt::Debug, time::Duration};

/// Frame rates every track should behave identically at.
pub const FRAME_RATES: [u32; 4] = [24, 30, 60, 144];

/// Longest span a track may take to complete before it is considered unbounded.
const LIMIT: Duration = Duration::from_secs(120);

/// Samples at `hz` from zero until completion or `limit`, endpoints included.
pub fn frames<T>(
    track: &impl Animation<T>,
    hz: u32,
    limit: Duration,
) -> Vec<(Duration, AnimationSample<T>)> {
    assert!(hz > 0, "frame rate must be positive");
    let mut out = Vec::new();
    let mut frame = 0_u64;
    loop {
        let at = Duration::from_secs_f64(frame as f64 / f64::from(hz));
        let sample = track.sample(at);
        let done = sample.completed || at >= limit;
        out.push((at, sample));
        if done {
            return out;
        }
        frame += 1;
    }
}

/// Asserts the contract of a finite track at every frame rate in `rates`:
/// time zero is not completed, the track completes exactly on `finish()`, a
/// sample after a long suspension is the same exact end, and the value shown
/// at a given instant does not depend on the sampling history.
pub fn assert_exact_finish<T: PartialEq + Debug>(track: &impl Animation<T>, rates: &[u32]) {
    let start = track.sample(Duration::ZERO);
    assert!(!start.completed, "track must not complete at time zero");
    for &hz in rates {
        let samples = frames(track, hz, LIMIT);
        let (at, last) = samples.last().unwrap();
        assert!(
            last.completed,
            "track did not complete within {LIMIT:?} at {hz} Hz"
        );
        assert_eq!(last.value, track.finish(), "final sample at {hz} Hz");
        if let Some(length) = track.duration() {
            assert!(
                *at >= length,
                "completed at {at:?}, before the declared duration {length:?}"
            );
        }
        for (at, sample) in &samples {
            assert_eq!(
                track.sample(*at).value,
                sample.value,
                "sampling at {at:?} is not repeatable"
            );
        }
    }
    let late = track.sample(LIMIT * 1000);
    assert!(late.completed, "long suspension must complete the track");
    assert_eq!(late.value, track.finish(), "suspended sample");
}

/// Asserts that a scalar track never decreases (or never increases, when `rising`
/// is false) while running, at every frame rate in `rates`.
pub fn assert_monotonic(track: &impl Animation<f32>, rates: &[u32], rising: bool) {
    for &hz in rates {
        let samples = frames(track, hz, LIMIT);
        for pair in samples.windows(2) {
            let (a, b) = (pair[0].1.value, pair[1].1.value);
            assert!(
                if rising { b >= a } else { b <= a },
                "{a} -> {b} at {hz} Hz is not monotonic"
            );
        }
    }
}
