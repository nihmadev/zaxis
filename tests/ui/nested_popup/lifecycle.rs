//! Lifecycle: what is built keeps the branch, what is not closes its part, and nothing stays.
use super::*;

#[test]
fn a_parent_that_is_not_built_closes_its_descendants_at_once() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    s.build_a = false;
    pass(&mut c, &mut s);
    assert!(ids(&c).is_empty());
    assert!(c
        .probe()
        .popup_layers
        .iter()
        .all(|l| !c.probe().popups.iter().any(|p| p.id == *l)));
}

#[test]
fn a_removed_leaf_keeps_its_ancestors() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 2);
    s.build_child = false;
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 1);
    assert!(s.a);
    tap(&mut c, &mut s, |s| s.plain);
    assert_eq!(s.plain_clicks, 1, "the ancestor still takes input");
}

#[test]
fn closing_a_level_hides_its_input_at_once() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    let leaf = ids(&c)[2];
    c.close_popup();
    assert!(!c.probe().previous_hits.iter().any(|h| h.window == leaf));
    assert!(c.probe().capture.is_none());
}

#[test]
fn focus_loss_closes_the_branch_without_restoring_focus() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    c.on_window_event(&winit::event::WindowEvent::Focused(false));
    assert!(ids(&c).is_empty());
    pass(&mut c, &mut s);
    assert_eq!(c.probe().focused_widget, None);
}

#[test]
fn a_modal_that_opens_cancels_the_whole_branch() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    s.modal = true;
    pass(&mut c, &mut s);
    assert!(ids(&c).is_empty());
    assert!(c.probe().modals.stack.len() == 1);
}

#[test]
fn focus_falls_back_when_the_trigger_is_gone() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 2);
    // The root's trigger vanishes (not built): the branch goes with it and focus is free.
    s.build_a = false;
    pass(&mut c, &mut s);
    pass(&mut c, &mut s);
    let focused = c.probe().focused_widget;
    assert!(focused.is_none_or(|f| c.probe().previous_hits.iter().any(|h| h.id == f)));
}

#[test]
fn opening_and_closing_many_times_leaves_nothing_behind() {
    let mut c = setup();
    let mut s = Scene::new();
    settle(&mut c, &mut s);
    for _ in 0..50 {
        s.a = true;
        pass(&mut c, &mut s);
        s.child = true;
        pass(&mut c, &mut s);
        s.grand = true;
        pass(&mut c, &mut s);
        assert_eq!(ids(&c).len(), 3);
        c.close_popup_branch();
        pass(&mut c, &mut s);
        pass(&mut c, &mut s);
        assert!(ids(&c).is_empty());
        assert!(!s.a);
    }
    assert!(c.probe().popup_layers.len() <= 1);
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .all(|h| h.window != Id::new("none")));
}

#[test]
fn a_settled_branch_asks_for_no_more_frames() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    for _ in 0..3 {
        pass(&mut c, &mut s);
    }
    assert!(!c.wants_animation_frame());
    assert!(c.next_repaint().is_none());
    c.close_popup_branch();
    for _ in 0..3 {
        pass(&mut c, &mut s);
    }
    assert!(!c.wants_animation_frame());
}

#[test]
fn a_disabled_ui_closes_its_popup_but_not_the_ones_it_is_not_part_of() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 2);
    s.child = false;
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 1);
    assert!(s.a);
}
