use super::{SplitPanel, SplitSize};

pub(super) fn key(panels: &[SplitPanel], prefs: &[SplitSize], available: f32) -> crate::Id {
    let sizes: Vec<_> = panels
        .iter()
        .zip(prefs)
        .map(|(p, pref)| {
            let pref = match pref {
                SplitSize::Pixels(n) => (0, n.to_bits()),
                SplitSize::Fraction(n) => (1, n.to_bits()),
                SplitSize::Weight(n) => (2, n.to_bits()),
            };
            (p.id, p.minimum.to_bits(), p.maximum.map(f32::to_bits), pref)
        })
        .collect();
    crate::Id::new((available.to_bits(), sizes))
}

/// A non-negative length; invalid values become zero.
#[track_caller]
pub(crate) fn dimension(n: f32) -> f32 {
    crate::components::sanitize::length("SplitPane size", n)
}
pub(super) fn limits(p: &SplitPanel) -> (f64, f64) {
    let min = dimension(p.minimum) as f64;
    (
        min,
        p.maximum
            .map_or(f64::INFINITY, |max| (dimension(max) as f64).max(min)),
    )
}

/// Bounded water filling in f64. Container fit outranks infeasible limits.
pub(super) fn resolve(panels: &[SplitPanel], prefs: &[SplitSize], available: f32) -> Vec<f32> {
    let total = dimension(available) as f64;
    let prefs: Vec<SplitSize> = prefs.iter().map(|pref| pref.normalized()).collect();
    let prefs = prefs.as_slice();
    let mins: Vec<_> = panels.iter().map(|p| limits(p).0).collect();
    let maxs: Vec<_> = panels.iter().map(|p| limits(p).1).collect();
    let min_sum: f64 = mins.iter().sum();
    if min_sum > total {
        return finish(
            mins.iter().map(|n| n * total / min_sum).collect(),
            available,
        );
    }
    let mut sizes: Vec<_> = prefs
        .iter()
        .enumerate()
        .map(|(i, pref)| {
            let desired = match pref.normalized() {
                SplitSize::Pixels(n) => n as f64,
                SplitSize::Fraction(n) => n as f64 * total,
                SplitSize::Weight(_) => 0.0,
            };
            desired.clamp(mins[i], maxs[i])
        })
        .collect();
    let sum: f64 = sizes.iter().sum();
    if sum > total {
        let capacity: f64 = sizes.iter().zip(&mins).map(|(s, m)| s - m).sum();
        for (s, m) in sizes.iter_mut().zip(&mins) {
            *s -= (sum - total) * (*s - m) / capacity;
        }
    } else {
        // Allocate total flexible shares, rather than adding shares on top of
        // minima. Thus remembering actual sizes as weights is idempotent.
        let indices: Vec<_> = prefs
            .iter()
            .enumerate()
            .filter_map(|(i, p)| matches!(p, SplitSize::Weight(_)).then_some(i))
            .collect();
        let fixed: f64 = sizes
            .iter()
            .enumerate()
            .filter(|(i, _)| !indices.contains(i))
            .map(|(_, s)| s)
            .sum();
        weighted(&mut sizes, &mins, &maxs, prefs, indices, total - fixed);
        let sum: f64 = sizes.iter().sum();
        let mut spare = total - sum;
        // Remainder weights first; fractions second; fixed panels last.
        for group in 0..3 {
            let weights: Vec<_> = prefs
                .iter()
                .map(|p| match (group, p) {
                    (0, SplitSize::Weight(w)) => *w as f64,
                    (1, SplitSize::Fraction(w)) => (*w as f64).max(1.0e-12),
                    (2, _) => 1.0,
                    _ => 0.0,
                })
                .collect();
            spare = distribute(&mut sizes, &maxs, &weights, spare);
        }
        if spare > 0.0 && !sizes.is_empty() {
            // All maxima exhausted: fill equally beyond max rather than overflow/leave holes.
            let share = spare / sizes.len() as f64;
            for size in &mut sizes {
                *size += share;
            }
        }
    }
    finish(sizes, available)
}
fn weighted(
    sizes: &mut [f64],
    mins: &[f64],
    maxs: &[f64],
    prefs: &[SplitSize],
    mut active: Vec<usize>,
    mut budget: f64,
) {
    for _ in 0..=sizes.len() {
        if active.is_empty() {
            break;
        }
        let weight = |i: usize| match prefs[i] {
            SplitSize::Weight(w) => w as f64,
            _ => 0.0,
        };
        let sum: f64 = active.iter().map(|&i| weight(i)).sum();
        let desired = |i| budget.max(0.0) * weight(i) / sum;
        let clamped_sum: f64 = active
            .iter()
            .map(|&i| desired(i).clamp(mins[i], maxs[i]))
            .sum();
        let fix_low = clamped_sum > budget;
        let frozen: Vec<_> = active
            .iter()
            .copied()
            .filter(|&i| {
                if fix_low {
                    desired(i) < mins[i]
                } else {
                    desired(i) > maxs[i]
                }
            })
            .collect();
        if frozen.is_empty() {
            for &i in &active {
                sizes[i] = desired(i).clamp(mins[i], maxs[i]);
            }
            break;
        }
        for &i in &frozen {
            sizes[i] = if fix_low { mins[i] } else { maxs[i] };
            budget -= sizes[i];
        }
        active.retain(|i| !frozen.contains(i));
    }
}
fn distribute(sizes: &mut [f64], maxs: &[f64], weights: &[f64], mut spare: f64) -> f64 {
    for _ in 0..=sizes.len() {
        if spare <= 1.0e-9 {
            break;
        }
        let weight: f64 = sizes
            .iter()
            .enumerate()
            .filter(|(i, s)| **s < maxs[*i])
            .map(|(i, _)| weights[i])
            .sum();
        if weight <= 0.0 {
            break;
        }
        let before = spare;
        for (i, size) in sizes.iter_mut().enumerate() {
            let add = (before * weights[i] / weight).min((maxs[i] - *size).max(0.0));
            *size += add;
            spare -= add;
        }
    }
    spare.max(0.0)
}
fn finish(sizes: Vec<f64>, total: f32) -> Vec<f32> {
    // Cumulative endpoints avoid a per-panel rounding debt; final endpoint is exact.
    let mut endpoint = 0.0_f64;
    let mut previous = 0.0_f32;
    let last = sizes.len().saturating_sub(1);
    sizes
        .into_iter()
        .enumerate()
        .map(|(i, size)| {
            endpoint += size.max(0.0);
            let next = if i == last {
                total
            } else {
                (endpoint as f32).min(total)
            };
            let result = (next - previous).max(0.0);
            previous = next;
            result
        })
        .collect()
}

/// Clamp both neighbours simultaneously. Infeasible layouts use effective
/// bounds including the current compressed/expanded sizes; no jump on press.
pub(super) fn resize(panels: &[SplitPanel], sizes: &mut [f32], i: usize, desired: f32) {
    let sum = sizes[i] as f64 + sizes[i + 1] as f64;
    let (a_min, a_max) = limits(&panels[i]);
    let (b_min, b_max) = limits(&panels[i + 1]);
    let low = a_min
        .min(sizes[i] as f64)
        .max(sum - b_max.max(sizes[i + 1] as f64));
    let high = a_max
        .max(sizes[i] as f64)
        .min(sum - b_min.min(sizes[i + 1] as f64));
    let a = (desired as f64).clamp(low.max(0.0), high.max(low)) as f32;
    sizes[i] = a;
    sizes[i + 1] = (sum - a as f64).max(0.0) as f32;
}

#[cfg(test)]
#[path = "../../../tests/split/allocation.rs"]
mod tests;
