use super::*;
#[test]
fn outside_release_floats_once_and_tab_drop_returns_it() {
    let mut s = Scene::new(true, 1.0);
    s.begin(p(1));
    s.c.move_pointer(vec2(400.0, 3.0));
    s.frame();
    s.c.primary_button(ElementState::Released);
    s.frame();
    s.frame();
    assert_eq!(s.state.floats.len(), 1);
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e, DockEvent::Floated { .. }))
            .count(),
        1
    );
    s.state.dock_float(p(1), p(0), None, 0.5).unwrap();
    s.frame();
    assert!(s.state.floats.is_empty());
    assert_eq!(s.c.probe().windows.len(), 1);
}
#[test]
fn window_drag_resize_and_external_bounds_round_trip() {
    let mut s = Scene::new(true, 1.0);
    s.state
        .float(
            p(1),
            Rect::from_min_size(vec2(80.0, 80.0), vec2(300.0, 220.0)),
        )
        .unwrap();
    s.frame();
    let old = s.state.floats[0].bounds;
    let wid =
        s.c.probe()
            .windows
            .iter()
            .find(|(_, w)| !w.root)
            .map(|(id, _)| *id)
            .unwrap();
    let handle =
        s.c.probe()
            .previous_hits
            .iter()
            .find(|h| h.window == wid && h.action == HitAction::Resize)
            .unwrap()
            .rect
            .center();
    s.c.move_pointer(handle);
    s.c.primary_button(ElementState::Pressed);
    s.frame();
    s.c.move_pointer(handle + vec2(30.0, 40.0));
    s.frame();
    s.c.primary_button(ElementState::Released);
    s.frame();
    assert!(s.state.floats[0].bounds.size().x > old.size().x);
    s.state.floats[0].bounds = Rect::from_min_size(vec2(200.0, 160.0), vec2(320.0, 240.0));
    s.frame();
    assert_eq!(s.c.probe().windows[&wid].rect, s.state.floats[0].bounds);
    assert_eq!(
        DockState::load(&s.state.save(), s.state.panels()).unwrap(),
        s.state
    );
}

#[test]
fn floating_tab_returns_through_real_drop_and_raises_on_click() {
    let mut s = Scene::new(true, 1.0);
    s.state
        .float(
            p(1),
            Rect::from_min_size(vec2(120.0, 140.0), vec2(280.0, 220.0)),
        )
        .unwrap();
    s.state
        .float(
            p(3),
            Rect::from_min_size(vec2(300.0, 260.0), vec2(280.0, 220.0)),
        )
        .unwrap();
    s.frame();
    s.frame();
    let tab = s.tab(p(1));
    s.click(tab.min + vec2(20.0, 16.0));
    assert!(s.state.floats.last().unwrap().node.contains(p(1)));
    s.begin(p(1));
    let target = s
        .out
        .panels
        .iter()
        .find(|p| p.panel == super::p(2))
        .unwrap()
        .content_bounds
        .center();
    s.c.move_pointer(target);
    s.frame();
    s.frame();
    s.c.primary_button(ElementState::Released);
    s.frame();
    s.frame();
    assert_eq!(s.state.floats.len(), 1);
    assert!(s.state.root.as_ref().unwrap().contains(p(1)));
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e,DockEvent::Moved { panel,.. } if *panel==p(1)))
            .count(),
        1
    );
}

#[test]
fn floating_header_moves_and_docks_on_release_without_tab_drag() {
    let mut s = Scene::new(false, 1.5);
    s.state
        .float(
            p(1),
            Rect::from_min_size(vec2(100.0, 140.0), vec2(280.0, 200.0)),
        )
        .unwrap();
    s.settle();
    let hit =
        s.c.probe()
            .previous_hits
            .iter()
            .find(|h| h.action == HitAction::Move && h.rect.size().x > 24.0)
            .unwrap()
            .rect
            .center();
    s.c.move_pointer(hit);
    s.c.primary_button(ElementState::Pressed);
    s.frame();
    for step in 1..=8 {
        s.c.move_pointer(hit + vec2(10.0 * step as f32, 5.0 * step as f32));
        s.frame();
        assert_eq!(s.state.floats[0].bounds.min.x, 100.0 + 10.0 * step as f32);
    }
    let at = s
        .out
        .panels
        .iter()
        .find(|p| p.panel == super::p(2))
        .unwrap()
        .content_bounds
        .center();
    s.c.move_pointer(at);
    s.frame();
    assert!(s.out.preview.is_some());
    s.c.primary_button(ElementState::Released);
    s.frame();
    s.frame();
    assert!(s.state.floats.is_empty());
}

#[test]
fn empty_surface_accepts_a_floating_tab() {
    let mut s = Scene::new(true, 1.0);
    for panel in [p(1), p(2), p(3)] {
        s.state.close(panel).unwrap();
    }
    s.state
        .float(
            p(0),
            Rect::from_min_size(vec2(50.0, 80.0), vec2(260.0, 200.0)),
        )
        .unwrap();
    s.frame();
    s.frame();
    assert!(s.state.root.is_none());
    s.begin(p(0));
    s.c.move_pointer(vec2(600.0, 420.0));
    s.frame();
    s.frame();
    s.c.primary_button(ElementState::Released);
    s.frame();
    s.frame();
    assert!(s.state.floats.is_empty());
    assert!(s.state.root.as_ref().unwrap().contains(p(0)));
}

#[test]
fn application_float_order_controls_actual_hit_testing() {
    let mut s = Scene::new(true, 1.0);
    let rect = Rect::from_min_size(vec2(120.0, 140.0), vec2(280.0, 220.0));
    s.state.float(p(1), rect).unwrap();
    s.state.float(p(3), rect).unwrap();
    s.frame();
    s.frame();
    s.state.floats.reverse();
    s.frame();
    s.frame();
    s.click(s.controls[&p(1)].rect.center());
    assert_eq!(s.state.focused, Some(p(1)));
}

#[test]
fn flying_window_routes_hits_and_resize_grip_at_its_displayed_bounds() {
    let mut s = Scene::new(false, 1.0);
    s.state
        .float(
            p(0),
            Rect::from_min_size(vec2(420.0, 200.0), vec2(280.0, 200.0)),
        )
        .unwrap();
    s.frame();
    let shown = s.out.panels.iter().find(|item| item.panel == p(0)).unwrap();
    assert_ne!(shown.bounds, shown.target_bounds);
    let probe = s.c.probe();
    let hit = s.c.hit_test(shown.bounds.center()).unwrap();
    let window = probe.windows.iter().find(|(_, w)| !w.root).unwrap();
    assert_eq!(hit.window, *window.0);
    let grip = probe
        .previous_hits
        .iter()
        .find(|h| h.window == *window.0 && h.action == HitAction::Resize)
        .unwrap();
    assert_eq!(grip.rect.max, window.1.displayed_rect.max);
    assert_eq!(window.1.displayed_rect.min, shown.bounds.min);
    assert_eq!(
        window.1.displayed_rect.max,
        shown.bounds.max + Vec2::splat(12.0)
    );
}

#[test]
fn floating_minimum_includes_the_standard_window_grip_margin() {
    let mut s = Scene::new(true, 1.0);
    s.state
        .float(p(0), Rect::from_min_size(vec2(80.0, 80.0), Vec2::ONE))
        .unwrap();
    s.frame();
    let panel = s.out.panels.iter().find(|item| item.panel == p(0)).unwrap();
    assert!(panel.target_bounds.size().x >= 120.0);
    assert!(panel.target_bounds.size().y >= 80.0);
}

#[test]
fn detaching_a_hidden_tab_starts_at_its_current_group_pose() {
    let mut s = Scene::new(false, 1.0);
    s.state.split(p(3), p(0), DockSide::Left, 0.25).unwrap();
    s.settle();
    let source = s
        .out
        .panels
        .iter()
        .find(|item| item.panel == p(0))
        .unwrap()
        .bounds;
    assert!(!s.built.contains(&p(1)));
    s.state
        .float(
            p(1),
            Rect::from_min_size(vec2(420.0, 200.0), vec2(280.0, 200.0)),
        )
        .unwrap();
    s.frame();
    let flying = s.out.panels.iter().find(|item| item.panel == p(1)).unwrap();
    assert_eq!(flying.bounds, source);
    assert_ne!(flying.bounds, flying.target_bounds);
}
