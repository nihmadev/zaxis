//! Target selection: priority, nesting, clipping, layers, zones and payload checks.
use super::*;

fn dragging_scene() -> Scene {
    let mut s = scene(1.0);
    s.begin();
    s
}

#[test]
fn payload_is_checked_by_type_and_by_value_before_highlighting() {
    let mut s = scene(1.0);
    s.payload = 8; // right type, rejected by value
    s.begin();
    s.hover_at(s.seen.target_rect.center());
    assert!(s.seen.hover && s.seen.rejected && !s.seen.acceptable);
    assert_eq!(s.context.cursor_icon(), winit::window::CursorIcon::NoDrop);
    s.hover_at(s.seen.foreign_rect.center());
    // The `Other` target never even sees an `Item`.
    assert!(!s.seen.hover);
    s.release();
    s.frame();
    s.frame();
    assert!(s.log.drops.is_empty());
    assert_eq!(s.log.ends[0].reason, DragReason::Cancelled);
}

#[test]
fn acceptance_is_rechecked_when_the_drop_is_delivered() {
    let mut s = dragging_scene();
    s.hover_at(s.seen.target_rect.center());
    assert!(s.seen.acceptable);
    s.release();
    // The application changed its mind between the release and the next pass.
    s.context.probe_mut().drag.ended.as_mut().unwrap().payload = Some(Payload::new(Item(9)));
    s.frame();
    s.frame();
    assert!(s.log.drops.is_empty());
    assert_eq!(s.log.ends[0].reason, DragReason::Cancelled);
}

#[test]
fn innermost_target_wins_and_a_rejecting_one_blocks_its_parent_unless_passthrough() {
    for (inner, expect_outer, expect_inner) in [
        (Inner::Accepts, false, true),
        (Inner::Rejects, false, false),
        (Inner::RejectsPassthrough, true, false),
    ] {
        let mut s = scene(1.0);
        s.inner = inner;
        s.frame();
        s.begin();
        s.hover_at(s.seen.inner_rect.center());
        assert_eq!(
            s.seen.inner_hover,
            expect_inner || !expect_outer,
            "{inner:?}"
        );
        assert_eq!(s.seen.hover, expect_outer, "outer {}", inner as u8);
        assert_eq!(s.seen.inner_rejected, !expect_inner && !expect_outer);
        // Outside the child, the parent accepts again.
        let outer = s.seen.target_rect;
        s.hover_at(Vec2::new(outer.max.x - 4.0, outer.max.y - 4.0));
        assert!(s.seen.hover && s.seen.acceptable);
    }
}

#[test]
fn targets_hidden_by_a_scroll_clip_or_a_window_do_not_accept() {
    let mut s = scene(1.0);
    s.clipped = true;
    s.frame();
    s.begin();
    let rect = s.seen.target_rect;
    s.hover_at(rect.min + Vec2::new(10.0, 10.0));
    assert!(s.seen.acceptable, "visible part");
    // The target is 80px tall inside a 50px area: the rest is clipped.
    s.hover_at(rect.min + Vec2::new(10.0, 70.0));
    assert!(!s.seen.hover, "clipped part");
    s.release();
    s.frame();
    s.frame();

    let mut s = scene(1.0);
    s.cover = Some(s.seen.target_rect);
    s.frame();
    s.frame();
    s.begin();
    s.hover_at(s.seen.target_rect.center());
    assert!(!s.seen.hover, "under a window");
    s.hover_at(Vec2::new(700.0, 500.0));
    s.release();
    s.frame();
    s.frame();
    assert!(s.log.drops.is_empty());
}

#[test]
fn disabled_targets_are_skipped_and_pointer_outside_the_viewport_hits_nothing() {
    let mut s = scene(1.0);
    s.target_enabled = false;
    s.begin();
    s.hover_at(s.seen.target_rect.center());
    assert!(!s.seen.hover);
    s.target_enabled = true;
    s.frame();
    s.hover_at(s.seen.target_rect.center());
    assert!(s.seen.acceptable, "state preserved across disabling");
    s.hover_at(Vec2::new(-20.0, s.seen.target_rect.center().y));
    assert!(!s.seen.hover);
}

#[test]
fn zones_report_before_inside_after_with_a_configurable_edge() {
    let mut s = scene(1.0);
    s.zones = Some(DropZones::tree());
    s.frame();
    s.begin();
    let r = s.seen.target_rect;
    for (fraction, expected) in [
        (0.1, Insertion::Before),
        (0.5, Insertion::Inside),
        (0.9, Insertion::After),
    ] {
        s.hover_at(Vec2::new(r.center().x, r.min.y + r.size().y * fraction));
        assert_eq!(s.seen.insertion, Some(expected));
    }
    s.zones = Some(DropZones::tree().inside(false));
    s.frame();
    s.hover_at(Vec2::new(r.center().x, r.center().y - 1.0));
    assert_eq!(s.seen.insertion, Some(Insertion::Before));
    s.zones = Some(DropZones::tree().edge(0.4));
    s.frame();
    s.hover_at(Vec2::new(r.center().x, r.min.y + r.size().y * 0.35));
    assert_eq!(s.seen.insertion, Some(Insertion::Before));
    s.hover_at(Vec2::new(r.center().x, r.min.y + r.size().y * 0.5));
    assert_eq!(s.seen.insertion, Some(Insertion::Inside));
    s.release();
    s.frame();
    assert_eq!(s.log.drops[0].insertion, Some(Insertion::Inside));
    assert!(s.log.drops[0].local.y > 0.0);
}

#[test]
fn preview_never_shadows_targets_and_has_no_hits() {
    let mut s = dragging_scene();
    s.hover_at(s.seen.target_rect.center());
    assert!(s.seen.acceptable);
    let layer = drag_preview_layer();
    assert!(s
        .context
        .probe()
        .previous_hits
        .iter()
        .all(|h| h.window != layer));
    assert!(s.context.probe().popup_layers.contains(&layer));
    assert!(s
        .context
        .probe()
        .elements
        .iter()
        .any(|e| e.layer == layer && !e.mesh.vertices.is_empty()));
}
