//! Keeps the overlay's share of the host's frame time bounded.

use std::time::Duration;

/// Most presents skipped after one expensive frame, so an overlay that hit a hitch (shader
/// compilation, a first upload) is back within a fraction of a second.
const MAX_SKIP: u32 = 8;

/// Tracks the cost of overlay frames. A frame over `limit` makes the following presents pass
/// through unchanged, as many as the overrun is worth, so the host's average frame time is
/// disturbed by no more than `limit`.
#[derive(Debug)]
pub(crate) struct Budget {
    limit: Option<Duration>,
    skip: u32,
}

impl Budget {
    pub fn new(limit: Option<Duration>) -> Self {
        Self { limit, skip: 0 }
    }

    /// Whether this present must pass through to pay back an earlier overrun.
    pub fn should_skip(&mut self) -> bool {
        if self.skip == 0 {
            return false;
        }
        self.skip -= 1;
        true
    }

    /// Record what a frame cost; returns whether it exceeded the limit.
    pub fn record(&mut self, cost: Duration) -> bool {
        let Some(limit) = self.limit.filter(|limit| !limit.is_zero()) else {
            return false;
        };
        if cost <= limit {
            return false;
        }
        let ratio = cost.as_secs_f64() / limit.as_secs_f64();
        self.skip = (ratio.ceil() as u32 - 1).clamp(1, MAX_SKIP);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_overrun_skips_in_proportion_and_is_capped() {
        let mut budget = Budget::new(Some(Duration::from_millis(4)));
        assert!(!budget.record(Duration::from_millis(4)));
        assert!(!budget.should_skip());
        assert!(budget.record(Duration::from_millis(12)));
        assert_eq!((budget.should_skip(), budget.should_skip()), (true, true));
        assert!(!budget.should_skip());
        assert!(budget.record(Duration::from_secs(5)));
        assert_eq!(
            std::iter::repeat_with(|| budget.should_skip())
                .take(20)
                .filter(|s| *s)
                .count(),
            8
        );
    }

    #[test]
    fn no_limit_never_skips() {
        let mut budget = Budget::new(None);
        assert!(!budget.record(Duration::from_secs(1)));
        assert!(!budget.should_skip());
    }
}
