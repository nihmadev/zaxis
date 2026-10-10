use super::*;

#[test]
fn quarter_edge_classification_and_preview_geometry() {
    let r = Rect::from_min_size(vec2(30.0, 20.0), vec2(200.0, 100.0));
    assert_eq!(DockDropZone::at(r, r.center()), Some(DockDropZone::Center));
    for (side, point) in [
        (DockSide::Left, vec2(35.0, 70.0)),
        (DockSide::Right, vec2(225.0, 70.0)),
        (DockSide::Top, vec2(130.0, 25.0)),
        (DockSide::Bottom, vec2(130.0, 115.0)),
    ] {
        assert_eq!(DockDropZone::at(r, point), Some(DockDropZone::Edge(side)));
        let preview = DockDropZone::Edge(side).preview(r);
        assert_eq!(preview.size().x * preview.size().y, 10000.0);
    }
    assert_eq!(DockDropZone::at(r, Vec2::ZERO), None);
}
#[test]
fn center_drop_moves_between_groups_once() {
    let mut s = Scene::new(true, 1.0);
    s.begin(p(1));
    let at = s.out.panels[1].content_bounds.center();
    s.c.move_pointer(at);
    s.frame();
    s.frame();
    assert!(s.out.preview.is_some());
    s.c.primary_button(ElementState::Released);
    s.frame();
    s.frame();
    s.frame();
    let moves: Vec<_> = s
        .events
        .iter()
        .filter(|e| matches!(e,DockEvent::Moved { panel,.. } if *panel == p(1)))
        .collect();
    assert_eq!(moves.len(), 1, "{:?}", s.events);
    assert!(s.state.floats.is_empty());
    assert!(s.built.contains(&p(1)));
}
#[test]
fn edge_drop_splits_and_escape_leaves_model_unchanged() {
    let mut s = Scene::new(true, 1.0);
    let before = s.state.save();
    s.begin(p(1));
    s.c.move_pointer(s.out.panels[1].content_bounds.min + vec2(4.0, 80.0));
    s.frame();
    s.frame();
    s.c.key(KeyCode::Escape, ElementState::Pressed, false);
    s.c.key(KeyCode::Escape, ElementState::Released, false);
    s.frame();
    assert_eq!(s.state.save(), before);
    assert!(s.events.is_empty());
    s.c.primary_button(ElementState::Released);
    s.frame();
    s.begin(p(1));
    s.c.move_pointer(s.out.panels[1].content_bounds.min + vec2(4.0, 80.0));
    s.frame();
    s.frame();
    s.c.primary_button(ElementState::Released);
    s.frame();
    s.frame();
    s.frame();
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e,DockEvent::Split { panel,.. } if *panel==p(1)))
            .count(),
        1
    );
    assert_eq!(s.out.panels.len(), 3);
}

#[test]
fn active_tab_can_split_its_own_multi_tab_group() {
    let mut s = Scene::new(true, 1.0);
    s.begin(p(0));
    let bounds = s.out.panels[0].content_bounds;
    s.c.move_pointer(bounds.min + vec2(2.0, 80.0));
    s.frame();
    s.frame();
    s.c.primary_button(ElementState::Released);
    s.frame();
    s.frame();
    assert_eq!(s.out.panels.len(), 3);
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e,DockEvent::Split { panel,.. } if *panel==p(0)))
            .count(),
        1
    );
}

#[test]
fn overflow_strip_autoscrolls_during_tab_drag_and_escape_releases_deadline() {
    let mut s = Scene::new(true, 1.0);
    s.state = DockState::new(DockNode::tabs("many", (0..40).map(p)));
    s.frame();
    s.frame();
    s.begin(p(0));
    let (id, clip) =
        s.c.probe()
            .scrolling
            .states
            .iter()
            .find(|(_, area)| area.axes[0] && area.max_offset().x > 0.0)
            .map(|(id, area)| (*id, area.clip))
            .unwrap();
    s.c.move_pointer(vec2(clip.max.x - 2.0, clip.center().y));
    s.frame();
    s.advance(100);
    s.advance(100);
    assert!(s.c.probe().scrolling.states[&id].offset.x > 0.0);
    s.c.key(KeyCode::Escape, ElementState::Pressed, false);
    s.c.key(KeyCode::Escape, ElementState::Released, false);
    s.c.primary_button(ElementState::Released);
    s.frame();
    s.settle();
    assert_eq!(s.state.panels().len(), 40);
    assert!(!s.c.wants_animation_frame());
}
