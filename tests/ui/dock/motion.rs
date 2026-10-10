use super::*;

#[test]
fn closing_mid_transition_keeps_the_displayed_surface_size() {
    let mut s = Scene::new(false, 1.0);
    if let Some(DockNode::Split { children, .. }) = &mut s.state.root {
        children[0].fraction = 0.75;
        children[1].fraction = 0.25;
    }
    s.frame();
    s.advance(40);
    let shown = s.out.panels[0].bounds;
    let target = s.out.panels[0].target_bounds;
    assert!(target.size().x - shown.size().x > 40.0);
    s.state.close(p(1)).unwrap();
    s.state.close(p(0)).unwrap();
    s.advance(0);
    let dock = Id::new("zaxis-root")
        .with("content")
        .with(("dock", Id::new("test-dock")));
    let surface = dock.with(("exit", dock.with(("surface", p(0)))));
    let bounds = s.c.probe().cache[&surface].bounds.unwrap();
    assert!((bounds.size().x - shown.size().x).abs() < 3.0);
    assert!((bounds.size().y - shown.size().y).abs() < 3.0);
    assert!(!s.built.contains(&p(0)));
    s.settle();
    assert!(!s.c.probe().cache.contains_key(&surface));
    assert!(!s.c.wants_animation_frame());
    assert!(s.c.next_repaint().is_none());
}

#[test]
fn ring_glides_converges_and_snaps_to_pixels_at_each_scale() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let mut s = Scene::new(false, scale);
        let start = s.out.focus_ring.unwrap();
        s.state.activate(p(2)).unwrap();
        s.frame();
        assert_eq!(s.out.focus_ring.unwrap(), start);
        s.advance(100);
        let moving = s.out.focus_ring.unwrap();
        assert!(moving.min.x > start.min.x && moving.min.x < s.out.panels[1].bounds.min.x);
        for point in [moving.min, moving.max] {
            for n in point.to_array() {
                assert!((n * scale as f32 - (n * scale as f32).round()).abs() < 0.001);
            }
        }
        s.settle();
        let end = s.out.focus_ring.unwrap();
        assert!((end.min.x - s.out.panels[1].bounds.min.x + 1.0).abs() <= 1.0 / scale as f32);
        assert!(!s.c.wants_animation_frame());
        assert!(s.c.next_repaint().is_none());
        assert!(!s.c.needs_repaint_at(s.now));
    }
}
#[test]
fn layout_rects_animate_neighbors_and_ring_tracks_displayed_geometry() {
    let mut s = Scene::new(false, 1.0);
    let old = s.out.panels[0].bounds;
    s.state.close(p(2)).unwrap();
    s.state.close(p(3)).unwrap();
    s.frame();
    assert_eq!(s.out.panels[0].bounds, old);
    assert!(s.out.panels[0].target_bounds.size().x > old.size().x);
    s.advance(100);
    let panel = &s.out.panels[0];
    assert!(
        panel.bounds.size().x > old.size().x
            && panel.bounds.size().x < panel.target_bounds.size().x
    );
    let ring = s.out.focus_ring.unwrap();
    assert!((ring.max.x - panel.bounds.max.x - 1.0).abs() < 1.0);
    s.settle();
    assert!(!s.c.wants_animation_frame());
}
#[test]
fn interrupted_ring_keeps_position_and_velocity_and_reduced_motion_settles() {
    let mut s = Scene::new(false, 1.0);
    s.state.activate(p(2)).unwrap();
    s.frame();
    s.advance(70);
    let moving = s.out.focus_ring.unwrap();
    s.state.activate(p(0)).unwrap();
    s.advance(0);
    assert_eq!(s.out.focus_ring.unwrap(), moving);
    s.advance(2);
    assert!(
        s.out.focus_ring.unwrap().min.x >= moving.min.x,
        "velocity carries through reversal"
    );
    let mut style = s.c.style().clone();
    style.motion.reduced_motion = true;
    s.c.set_style(style);
    s.frame();
    assert!((s.out.focus_ring.unwrap().min.x - s.out.panels[0].bounds.min.x + 1.0).abs() < 0.01);
    s.frame();
    assert!(!s.c.wants_animation_frame());
    assert!(s.c.next_repaint().is_none());
}
#[test]
fn window_focus_loss_dims_ring_and_disabled_gradient_does_not_redraw() {
    let mut s = Scene::new(false, 1.0);
    let button = s.controls[&p(0)];
    s.click(button.rect.center());
    s.settle();
    let active = s.out.focus_ring_color.unwrap();
    s.c.on_input(InputEvent::Focus(false));
    s.settle();
    assert_ne!(s.out.focus_ring_color.unwrap(), active);
    assert!(!s.c.wants_animation_frame());
    s.style.ring_gradient = Some(true);
    s.frame();
    assert!(!s.c.wants_animation_frame());
}
#[test]
fn dock_removal_releases_animation_tracks_and_cached_content() {
    let mut s = Scene::new(false, 1.0);
    s.state.activate(p(2)).unwrap();
    s.frame();
    assert!(s.c.wants_animation_frame());
    s.c.run_at(s.now + Duration::from_millis(16), |_| {});
    assert!(!s.c.wants_animation_frame());
    assert!(s.c.next_repaint().is_none());
    assert!(s.c.probe().cache.is_empty());
    assert_eq!(s.c.probe().counts.docks, 0);
}

#[test]
fn gradient_redraw_is_opt_in_and_reduced_motion_releases_it() {
    let mut s = Scene::new(false, 1.0);
    let button = s.controls[&p(0)];
    s.click(button.rect.center());
    s.settle();
    s.style.ring_gradient = Some(true);
    s.frame();
    assert!(s.c.wants_animation_frame());
    s.style.ring_gradient = Some(false);
    s.frame();
    assert!(!s.c.wants_animation_frame());
    s.style.ring_gradient = Some(true);
    s.frame();
    assert!(s.c.wants_animation_frame());
    let mut style = s.c.style().clone();
    style.motion.reduced_motion = true;
    s.c.set_style(style);
    s.frame();
    assert!(!s.c.wants_animation_frame());
    assert!(s.c.next_repaint().is_none());
}

#[test]
fn dragging_changes_preview_without_retargeting_panels_or_ring() {
    let mut s = Scene::new(false, 1.0);
    let button = s.controls[&p(0)];
    s.click(button.rect.center());
    s.settle();
    s.begin(p(1));
    s.settle();
    let bounds: Vec<_> = s.out.panels.iter().map(|p| p.bounds).collect();
    let ring = s.out.focus_ring;
    let target = s
        .out
        .panels
        .iter()
        .find(|p| p.panel == super::p(2))
        .unwrap()
        .content_bounds;
    for i in 0..12 {
        s.c.move_pointer(target.min + vec2(2.0 + i as f32, 80.0));
        s.frame();
        assert_eq!(
            s.out.panels.iter().map(|p| p.bounds).collect::<Vec<_>>(),
            bounds
        );
        assert_eq!(s.out.focus_ring, ring);
    }
    assert!(s.out.preview.is_some());
}

#[test]
fn default_ring_spring_converges_without_overshoot_from_rest() {
    let mut s = Scene::new(false, 1.0);
    let from = s.out.focus_ring.unwrap().min.x;
    s.state.activate(p(2)).unwrap();
    s.frame();
    let target = s.out.panels[1].bounds.min.x - 1.0;
    let mut previous = from;
    for _ in 0..45 {
        s.frame();
        let x = s.out.focus_ring.unwrap().min.x;
        assert!(x >= previous && x <= target + 0.5);
        previous = x;
    }
    assert!((previous - target).abs() < 0.5);
    assert!(!s.c.wants_animation_frame());
}

#[test]
fn switching_tabs_in_a_moving_group_keeps_its_speed_and_attached_ring() {
    let mut s = Scene::new(false, 1.0);
    s.state.close(p(2)).unwrap();
    s.state.close(p(3)).unwrap();
    s.frame();
    s.advance(70);
    let before = s.out.focus_ring.unwrap();
    s.advance(2);
    let moving = s.out.focus_ring.unwrap();
    let speed = moving.max.x - before.max.x;
    assert!(speed >= 3.0);
    s.state.activate(p(1)).unwrap();
    s.advance(0);
    assert_eq!(s.out.focus_ring.unwrap(), moving);
    s.advance(2);
    assert!(s.out.focus_ring.unwrap().max.x - moving.max.x >= speed * 0.7);
    let panel = s.out.panels.iter().find(|item| item.panel == p(1)).unwrap();
    assert!((s.out.focus_ring.unwrap().max.x - panel.bounds.max.x - 1.0).abs() < 1.0);
}

#[test]
fn switching_focus_between_moving_groups_preserves_world_velocity() {
    let mut s = Scene::new(false, 1.0);
    if let Some(DockNode::Split { children, .. }) = &mut s.state.root {
        children[0].fraction = 0.8;
        children[1].fraction = 0.2;
    }
    s.frame();
    s.advance(70);
    let before = s.out.focus_ring.unwrap();
    s.state.activate(p(2)).unwrap();
    s.advance(0);
    assert_eq!(s.out.focus_ring.unwrap(), before);
    s.advance(1);
    assert!(s.out.focus_ring.unwrap().min.x - before.min.x <= 1.0);
    s.settle();
    assert!(!s.c.wants_animation_frame());
}
