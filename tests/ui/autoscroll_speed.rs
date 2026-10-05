use zaxis::context::drag::autoscroll::axis_speed;

#[test]
fn speed_grows_toward_the_edge_and_stays_zero_in_the_middle() {
    let speeds: Vec<f32> = [98.0, 90.0, 80.0, 70.0, 50.0]
        .into_iter()
        .map(|p| axis_speed(p, 0.0, 100.0, 32.0, 80.0, 900.0))
        .collect();
    assert_eq!(speeds[4], 0.0);
    assert!(speeds[0] > speeds[1] && speeds[1] > speeds[2] && speeds[2] > speeds[3]);
    assert!(speeds[3] > 0.0 && speeds[0] <= 900.0);
    assert!(axis_speed(2.0, 0.0, 100.0, 32.0, 80.0, 900.0) < -700.0);
    assert!(axis_speed(-50.0, 0.0, 100.0, 32.0, 80.0, 900.0) <= -900.0 + 1e-3);
}
