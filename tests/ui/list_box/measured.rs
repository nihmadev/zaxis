use super::*;
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent},
};
use zaxis::testing::RowMetrics;

fn varied(count: u32) -> Vec<Item> {
    (0..count)
        .map(|i| Item {
            lines: 1 + (i % 3) as usize,
            ..item(i, &format!("Row {i}"))
        })
        .collect()
}
fn measured(count: u32) -> List {
    let mut l = List::new(Vec::new(), ListMode::Single);
    l.items = varied(count);
    l.measured = Some(ROW);
    l.height = 200.0;
    for _ in 0..4 {
        l.frame();
    }
    l
}
fn state(l: &List) -> &ListState {
    l.c.probe().list_boxes.values().next().unwrap()
}
fn screen_y(l: &List, index: usize) -> f32 {
    state(l).heights.top(index) - l.out().scroll_offset.y
}
fn wheel(l: &mut List, dy: f32) {
    let p = l.out().viewport.center();
    l.c.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(p.x as f64, p.y as f64),
    });
    l.c.on_window_event(&WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -dy as f64)),
        phase: TouchPhase::Moved,
    });
    l.frame();
}

#[test]
fn rows_take_their_content_height_and_settle() {
    let l = measured(200);
    let h = &state(&l).heights;
    // Row 0 has one line, row 2 has three: measured heights differ.
    assert!(h.height(2) > h.height(0) + 10.0);
    assert!(l.settled());
    assert!(!l.out().rebuilt);
}

#[test]
fn scrolling_up_through_unmeasured_rows_never_moves_the_visible_rows() {
    let mut l = measured(2000);
    l.scroll_to = Some(1500);
    for _ in 0..4 {
        l.frame();
    }
    let probe = l.out().visible.start + 1;
    let mut last = screen_y(&l, probe);
    for _ in 0..25 {
        wheel(&mut l, -20.0);
        let now = screen_y(&l, probe);
        assert!(
            (now - last - 20.0).abs() < 0.6,
            "probe moved {} instead of 20",
            now - last
        );
        last = now;
    }
}

#[test]
fn content_height_changes_only_by_measured_deltas() {
    let mut l = measured(5000);
    l.scroll_to = Some(2500);
    for _ in 0..8 {
        l.frame();
    }
    let settled = state(&l).heights.total_height();
    for _ in 0..3 {
        l.frame();
    }
    assert_eq!(
        state(&l).heights.total_height(),
        settled,
        "no input, no change"
    );
    // Unmeasured rows keep the estimate: far rows are still `ROW` tall.
    assert_eq!(state(&l).heights.height(4999), ROW);
}

#[test]
fn measured_cache_is_bounded_by_the_model() {
    let mut l = measured(300);
    for target in [10, 100, 290] {
        l.scroll_to = Some(target);
        l.frame();
        l.frame();
    }
    assert!(state(&l).heights.cached() <= 300);
    l.items.truncate(20);
    l.frame();
    l.frame();
    assert!(
        state(&l).heights.cached() <= 20,
        "entries of removed rows are dropped"
    );
}

#[test]
fn keyboard_and_pointer_agree_with_measured_rows() {
    let mut l = measured(100);
    let p = Vec2::new(
        l.out().viewport.min.x + 40.0,
        l.out().viewport.min.y + screen_y(&l, 2) + 6.0,
    );
    l.click_at(p, ModifiersState::empty());
    assert_eq!(l.selected(), vec![2]);
    l.key(KeyCode::End);
    for _ in 0..6 {
        l.frame();
    }
    assert_eq!(l.active(), Some(99));
    assert!(l.out().visible.contains(&99));
}
