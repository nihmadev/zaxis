use super::*;
fn panels(n: usize) -> Vec<SplitPanel> {
    (0..n).map(SplitPanel::new).collect()
}
#[test]
fn weighted_fixed_fraction_and_limits() {
    let p = panels(3);
    assert_eq!(
        resolve(
            &p,
            &[
                SplitSize::Pixels(100.0),
                SplitSize::Fraction(0.25),
                SplitSize::Weight(1.0)
            ],
            400.0
        ),
        [100.0, 100.0, 200.0]
    );
    let p = vec![
        SplitPanel::new(0).max_size(40.0),
        SplitPanel::new(1),
        SplitPanel::new(2),
    ];
    assert_eq!(
        resolve(&p, &[SplitSize::Weight(1.0); 3], 200.0),
        [40.0, 80.0, 80.0]
    );
}
#[test]
fn infeasible_limits_and_small_regions_fit() {
    let p = vec![
        SplitPanel::new(0).min_size(100.0).max_size(40.0),
        SplitPanel::new(1).min_size(300.0),
    ];
    assert_eq!(
        resolve(&p, &[SplitSize::Weight(1.0); 2], 80.0),
        [20.0, 60.0]
    );
    assert_eq!(resolve(&p, &[SplitSize::Weight(1.0); 2], 0.0), [0.0, 0.0]);
    let p = vec![
        SplitPanel::new(0).max_size(10.0),
        SplitPanel::new(1).max_size(20.0),
    ];
    assert_eq!(
        resolve(&p, &[SplitSize::Weight(1.0); 2], 100.0),
        [45.0, 55.0]
    );
    assert!(resolve(&[], &[], 100.0).is_empty());
    assert_eq!(
        resolve(&panels(1), &[SplitSize::Pixels(10.0)], 100.0),
        [100.0]
    );
}
#[test]
fn pair_resize_obeys_both_limits_and_keeps_other_panels() {
    let p = vec![
        SplitPanel::new(0).min_size(30.0).max_size(80.0),
        SplitPanel::new(1).min_size(40.0).max_size(90.0),
        SplitPanel::new(2),
    ];
    let mut s = vec![60.0, 60.0, 50.0];
    resize(&p, &mut s, 0, 1000.0);
    assert_eq!(s, [80.0, 40.0, 50.0]);
    resize(&p, &mut s, 0, -1000.0);
    assert_eq!(s, [30.0, 90.0, 50.0]);
}
#[test]
fn many_panels_keep_endpoints_without_rounding_debt() {
    for n in 1..130 {
        let p = panels(n);
        for total in [0.01, 1.0, 731.37, 100000.0] {
            let s = resolve(
                &p,
                &(0..n)
                    .map(|i| SplitSize::Weight((i + 1) as f32))
                    .collect::<Vec<_>>(),
                total,
            );
            assert!(s.iter().all(|s| s.is_finite() && *s >= 0.0));
            let sum: f64 = s.iter().map(|s| *s as f64).sum();
            assert!((sum - total as f64).abs() <= total as f64 * 1.0e-6);
        }
    }
}
#[test]
#[should_panic(expected = "positive and finite")]
fn invalid_weight_is_explicit() {
    resolve(&panels(1), &[SplitSize::Weight(f32::NAN)], 100.0);
}
#[test]
#[should_panic(expected = "finite and nonnegative")]
fn invalid_minimum_is_explicit() {
    resolve(
        &[SplitPanel {
            minimum: f32::INFINITY,
            ..SplitPanel::new(0)
        }],
        &[SplitSize::Weight(1.0)],
        100.0,
    );
}
