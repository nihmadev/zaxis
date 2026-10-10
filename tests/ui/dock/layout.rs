use super::*;

#[test]
fn builds_only_active_panels_once_and_fills_remaining_bounds() {
    let s = Scene::new(true, 1.0);
    assert_eq!(s.built, [p(0), p(2)]);
    assert_eq!(s.out.panels.len(), 2);
    assert!(s
        .out
        .panels
        .iter()
        .all(|p| p.bounds.is_finite() && !p.bounds.is_empty()));
    assert!(s.out.panels[0].bounds.max.x < s.out.panels[1].bounds.min.x);
}
#[test]
fn minima_and_infeasible_resize_use_splitpane_allocation() {
    let mut s = Scene::new(true, 1.0);
    if let Some(DockNode::Split { children, .. }) = &mut s.state.root {
        children[0].fraction = 0.001;
        children[1].fraction = 0.999;
    }
    s.frame();
    assert!(s.out.panels[0].bounds.size().x >= 120.0);
    s.c.set_viewport(PhysicalSize::new(100, 100), 1.0);
    s.frame();
    assert!(s
        .out
        .panels
        .iter()
        .all(|p| p.bounds.is_finite() && p.bounds.max.x <= 100.0));
}
#[test]
fn split_handles_change_application_shares_and_double_press_resets() {
    let mut s = Scene::new(true, 1.0);
    let hit =
        s.c.probe()
            .previous_hits
            .iter()
            .find(|h| matches!(h.action, HitAction::SplitResize { .. }))
            .unwrap()
            .rect
            .center();
    s.c.move_pointer(hit);
    s.c.primary_button(ElementState::Pressed);
    s.frame();
    s.c.move_pointer(hit + vec2(90.0, 0.0));
    s.frame();
    s.c.primary_button(ElementState::Released);
    s.frame();
    let first = s.out.panels[0].bounds.size().x;
    assert!(first > 400.0);
    assert!(s.events.contains(&DockEvent::LayoutChanged));
    let hit =
        s.c.probe()
            .previous_hits
            .iter()
            .find(|h| matches!(h.action, HitAction::SplitResize { .. }))
            .unwrap()
            .rect
            .center();
    s.click(hit);
    s.click(hit);
    s.frame();
    assert!((s.out.panels[0].bounds.size().x - s.out.panels[1].bounds.size().x).abs() < 1.0);
}
#[test]
fn theme_and_preset_resolve_from_tokens() {
    let mut s = Scene::new(true, 1.0);
    let tiled = s.out.panels[0].bounds;
    s.style.preset = Some(DockPreset::Flat);
    s.frame();
    assert!(s.out.panels[0].bounds.size().x > tiled.size().x);
    let dark = s.out.focus_ring_color;
    let mut theme = Theme::light();
    theme.overrides.motion = Some(MotionStyle {
        reduced_motion: true,
        ..Default::default()
    });
    s.c.set_theme(theme);
    s.frame();
    assert_ne!(s.out.focus_ring_color, dark);
}

#[test]
fn content_width_uses_final_layout_while_panel_clip_moves() {
    let mut s = Scene::new(false, 1.0);
    s.text = Some("text at final width".into());
    s.frame();
    s.settle();
    let old = s.out.panels[0].bounds.size().x;
    s.state.close(p(2)).unwrap();
    s.state.close(p(3)).unwrap();
    s.frame();
    let width = s.controls[&p(0)].rect.size().x;
    assert!(width > old);
    for _ in 0..12 {
        s.frame();
        assert_eq!(s.controls[&p(0)].rect.size().x, width);
    }
    assert!(s.out.panels[0].bounds.size().x > s.out.panels[0].content_bounds.size().x);
}

#[test]
fn ui_dock_closure_builds_only_visible_values() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut state = state();
    let mut built = Vec::new();
    c.run(|c| {
        Root::new().show(c, |ui| {
            let out = ui.dock(&mut state, |ui, panel| {
                built.push(*panel);
                ui.button("Content");
            });
            assert_eq!(out.panels.len(), 2);
            assert!(out.focus_ring.is_some());
        })
    });
    assert_eq!(built, [p(0), p(2)]);
}
