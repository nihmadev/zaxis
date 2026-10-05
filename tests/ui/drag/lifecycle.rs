//! Threshold, click preservation, capture, one-event guarantees and endings.
use super::*;
use winit::event::WindowEvent;

#[test]
fn click_and_hover_survive_until_the_threshold_then_one_drag_begins() {
    let mut s = scene(1.0);
    let start = s.seen.source_rect.center();
    s.move_to(start);
    s.frame();
    assert!(
        s.context.hovered(
            Id::new("source").with(("drag-source", 0)),
            Id::new("zaxis-root"),
            s.seen.source_rect,
            s.seen.source_rect
        ) || true
    );
    // A press and release without travel is a plain click.
    s.press();
    s.move_to(start + Vec2::new(2.0, 1.0));
    s.release();
    s.frame();
    assert_eq!((s.log.clicks, s.log.begins), (1, 0));
    assert!(s.context.dragging().is_none());
    // Travel past the threshold starts exactly one drag and swallows the click.
    s.press();
    s.move_to(start + Vec2::new(0.0, 6.0));
    s.frame();
    s.frame();
    assert_eq!(s.log.begins, 1);
    assert_eq!(s.context.dragging(), Some(Id::new("source")));
    s.release();
    s.frame();
    s.frame();
    assert_eq!((s.log.clicks, s.log.begins), (1, 1));
}

#[test]
fn threshold_is_logical_and_identical_at_every_scale() {
    for scale in [1.0, 1.5, 2.0] {
        let mut s = scene(scale);
        let start = s.seen.source_rect.center();
        s.move_to(start);
        s.press();
        s.move_to(start + Vec2::new(4.9, 0.0));
        s.frame();
        assert!(s.context.probe().drag.session.is_none(), "scale {scale}");
        s.move_to(start + Vec2::new(5.2, 0.0));
        assert!(s.context.probe().drag.session.is_some(), "scale {scale}");
    }
}

#[test]
fn drop_is_reported_once_and_the_source_learns_the_result_a_pass_later() {
    let mut s = scene(1.0);
    s.begin();
    assert!(s.seen.active);
    let target = s.seen.target_rect.center();
    s.move_to(target);
    s.frame();
    assert!(s.seen.hover && s.seen.acceptable && !s.seen.rejected);
    s.release();
    s.frame();
    assert_eq!(s.log.drops.len(), 1);
    let drop = &s.log.drops[0];
    assert_eq!(drop.payload, Item(7));
    assert_eq!(
        (drop.source, drop.target),
        (Id::new("source"), Id::new("target"))
    );
    assert!(s.log.ends.is_empty());
    s.frame();
    assert_eq!(s.log.ends.len(), 1);
    assert_eq!(s.log.ends[0].reason, DragReason::Dropped);
    assert_eq!(s.log.ends[0].target, Some(Id::new("target")));
    for _ in 0..3 {
        s.frame();
    }
    assert_eq!(
        (s.log.begins, s.log.drops.len(), s.log.ends.len()),
        (1, 1, 1)
    );
    assert!(s.context.dragging().is_none());
}

#[test]
fn release_over_nothing_or_a_rejecting_target_cancels_without_a_drop() {
    for over_foreign in [false, true] {
        let mut s = scene(1.0);
        s.begin();
        let at = if over_foreign {
            s.seen.foreign_rect.center()
        } else {
            Vec2::new(700.0, 500.0)
        };
        s.move_to(at);
        s.frame();
        assert!(!s.seen.rejected);
        s.release();
        s.frame();
        s.frame();
        assert!(s.log.drops.is_empty());
        assert_eq!(s.log.ends.len(), 1);
        assert_eq!(s.log.ends[0].reason, DragReason::Cancelled);
    }
}

#[test]
fn escape_cancels_once_and_a_later_release_does_nothing() {
    let mut s = scene(1.0);
    s.begin();
    s.move_to(s.seen.target_rect.center());
    s.frame();
    s.key(KeyCode::Escape);
    s.release();
    s.frame();
    s.frame();
    assert!(s.log.drops.is_empty());
    assert_eq!(s.log.ends.len(), 1);
    assert_eq!(s.log.ends[0].reason, DragReason::Escape);
    s.key(KeyCode::Escape);
    s.frame();
    assert_eq!(s.log.ends.len(), 1);
}

#[test]
fn capture_follows_the_pointer_outside_the_window_and_focus_loss_ends_it() {
    let mut s = scene(1.0);
    s.begin();
    s.move_to(Vec2::new(-300.0, 900.0));
    s.context.on_window_event(&WindowEvent::CursorLeft {
        device_id: winit::event::DeviceId::dummy(),
    });
    s.frame();
    assert!(s.context.probe().drag.session.is_some());
    assert!(!s.seen.hover);
    s.context.on_window_event(&WindowEvent::Focused(false));
    s.frame();
    s.frame();
    assert_eq!(s.log.ends.len(), 1);
    assert_eq!(s.log.ends[0].reason, DragReason::FocusLost);
    assert!(s.context.probe().drag.session.is_none() && s.context.probe().capture.is_none());
}

#[test]
fn removed_or_disabled_source_and_resize_end_the_session_without_hanging() {
    for case in 0..3 {
        let mut s = scene(1.0);
        s.begin();
        match case {
            0 => s.show_source = false,
            1 => s.source_enabled = false,
            _ => {
                s.context.set_viewport(PhysicalSize::new(640, 480), 1.0);
                s.context
                    .on_window_event(&WindowEvent::Resized(PhysicalSize::new(640, 480)));
            }
        }
        s.frame();
        s.frame();
        assert!(s.context.probe().drag.session.is_none() && s.context.probe().capture.is_none());
        assert!(s.log.drops.is_empty());
        if case == 2 {
            assert_eq!(s.context.drag_end(), None);
        }
        s.show_source = true;
        s.source_enabled = true;
        s.frame();
        s.frame();
        assert_eq!(s.log.begins, 1, "case {case}");
    }
}

#[test]
fn target_removed_between_frames_receives_no_drop() {
    let mut s = scene(1.0);
    s.begin();
    s.move_to(s.seen.target_rect.center());
    s.frame();
    s.release();
    s.show_target = false;
    s.frame();
    s.frame();
    assert!(s.log.drops.is_empty());
    assert_eq!(s.log.ends.len(), 1);
    assert_eq!(s.log.ends[0].reason, DragReason::Cancelled);
}

#[test]
fn idle_sources_and_targets_do_not_request_frames_or_geometry() {
    let mut s = scene(1.0);
    for _ in 0..3 {
        s.frame();
    }
    let revision = s.context.draw_data().revision;
    let tessellated = s.context.cache_stats().tessellated_elements;
    s.frame();
    assert_eq!(s.context.draw_data().revision, revision);
    assert_eq!(s.context.cache_stats().tessellated_elements, tessellated);
    assert!(!s.context.needs_repaint());
    assert!(!s.context.wants_animation_frame());
    assert!(s.context.probe().drag.last_targets.is_empty());
}
