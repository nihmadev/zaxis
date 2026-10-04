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

fn timing(visible: bool, needs: bool, next: Option<Instant>) -> schedule::WindowTiming {
    schedule::WindowTiming {
        visible,
        needs_repaint: needs,
        next_repaint: next,
        retry_at: None,
    }
}

#[test]
fn only_the_window_with_work_is_redrawn() {
    let now = Instant::now();
    let (redraw, flow) = schedule::schedule_all(
        now,
        [
            ("animating", timing(true, true, None)),
            ("idle", timing(true, false, None)),
            ("also idle", timing(true, false, None)),
        ],
    );
    assert_eq!(redraw, ["animating"]);
    assert_eq!(flow, ControlFlow::Wait);
}

#[test]
fn idle_windows_contribute_no_wakeup() {
    let now = Instant::now();
    let (redraw, flow) = schedule::schedule_all(
        now,
        [
            ("a", timing(true, false, None)),
            ("b", timing(true, false, None)),
        ],
    );
    assert!(redraw.is_empty());
    assert_eq!(flow, ControlFlow::Wait, "no busy loop, no timer");
}

#[test]
fn deadlines_are_independent_and_the_loop_wakes_for_the_nearest() {
    let now = Instant::now();
    let soon = now + Duration::from_millis(20);
    let late = now + Duration::from_millis(500);
    let (redraw, flow) = schedule::schedule_all(
        now,
        [
            ("late", timing(true, false, Some(late))),
            ("soon", timing(true, false, Some(soon))),
        ],
    );
    assert!(redraw.is_empty(), "no window is due yet");
    assert_eq!(flow, ControlFlow::WaitUntil(soon));
    let (redraw, flow) = schedule::schedule_all(
        soon,
        [
            ("late", timing(true, false, Some(late))),
            ("soon", timing(true, true, Some(soon))),
        ],
    );
    assert_eq!(redraw, ["soon"], "the other window keeps sleeping");
    assert_eq!(flow, ControlFlow::WaitUntil(late));
}

#[test]
fn hidden_minimized_and_closed_windows_neither_draw_nor_wake_the_loop() {
    let now = Instant::now();
    let (redraw, flow) = schedule::schedule_all(
        now,
        [
            ("hidden", timing(false, true, Some(now))),
            ("shown", timing(true, false, None)),
        ],
    );
    assert!(redraw.is_empty());
    assert_eq!(flow, ControlFlow::Wait);
    // A closed window is simply absent from the set.
    let (redraw, flow) = schedule::schedule_all::<&str>(now, []);
    assert!(redraw.is_empty() && flow == ControlFlow::Wait);
}

#[test]
fn a_retrying_window_backs_off_without_delaying_the_others() {
    let now = Instant::now();
    let retry = now + Duration::from_millis(16);
    let mut failing = timing(true, true, None);
    failing.retry_at = Some(retry);
    let (redraw, flow) = schedule::schedule_all(
        now,
        [("failing", failing), ("healthy", timing(true, true, None))],
    );
    assert_eq!(redraw, ["healthy"]);
    assert_eq!(flow, ControlFlow::WaitUntil(retry));
}

#[test]
fn earliest_prefers_a_deadline_over_waiting() {
    let at = Instant::now();
    assert_eq!(
        schedule::earliest(ControlFlow::Wait, ControlFlow::Wait),
        ControlFlow::Wait
    );
    assert_eq!(
        schedule::earliest(ControlFlow::Wait, ControlFlow::WaitUntil(at)),
        ControlFlow::WaitUntil(at)
    );
}
