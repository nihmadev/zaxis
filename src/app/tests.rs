use super::*;

#[test]
fn idle_input_and_animation_schedule_without_polling() {
    let now = Instant::now();
    let later = now + Duration::from_millis(50);
    assert_eq!(
        repaint_schedule(now, true, false, None, None),
        (false, ControlFlow::Wait)
    );
    assert_eq!(
        repaint_schedule(now, true, true, None, None),
        (true, ControlFlow::Wait)
    );
    assert_eq!(
        repaint_schedule(now, true, false, Some(later), None),
        (false, ControlFlow::WaitUntil(later))
    );
    assert_eq!(
        repaint_schedule(later, true, true, Some(later), None),
        (true, ControlFlow::Wait)
    );
}

#[test]
fn retry_backs_off_even_with_dirty_ui_and_expired_animation() {
    let now = Instant::now();
    let retry = now + Duration::from_millis(16);
    assert_eq!(
        repaint_schedule(now, true, true, Some(now), Some(retry)),
        (false, ControlFlow::WaitUntil(retry))
    );
    assert_eq!(
        repaint_schedule(retry, true, false, None, Some(retry)),
        (true, ControlFlow::Wait)
    );
}

#[test]
fn hidden_windows_sleep_even_with_active_motion_and_expired_retry() {
    let now = Instant::now();
    assert_eq!(
        repaint_schedule(now, false, true, Some(now), Some(now)),
        (false, ControlFlow::Wait)
    );
}

#[test]
fn vsync_motion_requests_the_next_present_without_a_second_timer() {
    let now = Instant::now();
    assert_eq!(
        repaint_schedule(now, true, true, Some(now + Duration::from_millis(16)), None),
        (true, ControlFlow::Wait)
    );
}
