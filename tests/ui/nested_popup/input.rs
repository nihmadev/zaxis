//! Pointer and keyboard: who takes a press, what each key closes, and the focus afterwards.
use super::*;

fn press(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
}

#[test]
fn a_press_inside_the_leaf_reaches_its_control() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    tap(&mut c, &mut s, |s| s.grand_inner);
    assert_eq!(s.grand_clicks, 1);
    assert_eq!(ids(&c).len(), 3, "the whole branch stays open");
}

#[test]
fn a_press_in_an_ancestor_closes_the_descendants_and_is_consumed_whole() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    // The plain button is in the root popup, outside child and grandchild.
    let plain = center(s.plain);
    let inside_child = s.child_rect.is_some_and(|r| r.contains(plain));
    assert!(!inside_child, "the probe point is outside the child");
    press(&mut c, plain);
    assert_eq!(ids(&c).len(), 1, "descendants closed, the ancestor stays");
    c.primary_button(ElementState::Released);
    pass(&mut c, &mut s);
    assert_eq!(s.plain_clicks, 0, "the dismissing press is not a click");
    assert!(s.a && !s.child);
    // What the closed child had open does not come back with it.
    s.child = true;
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 2);
    assert!(!s.grand);
    s.child = false;
    pass(&mut c, &mut s);
    click(&mut c, &mut s, plain);
    assert_eq!(s.plain_clicks, 1, "the next press works on the ancestor");
}

#[test]
fn a_press_outside_the_branch_closes_it_without_reaching_the_window_below() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    tap(&mut c, &mut s, |s| s.under);
    assert!(ids(&c).is_empty());
    assert_eq!(s.under_clicks, 0);
    assert!(!s.a, "the root builder hears of it");
    tap(&mut c, &mut s, |s| s.under);
    assert_eq!(s.under_clicks, 1);
}

#[test]
fn the_release_after_a_dismissing_press_is_not_a_click_below() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 2);
    press(&mut c, center(s.under));
    pass(&mut c, &mut s);
    c.primary_button(ElementState::Released);
    pass(&mut c, &mut s);
    assert_eq!(s.under_clicks, 0);
}

#[test]
fn a_secondary_press_closes_what_it_lands_outside_of_and_nothing_else() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    let secondary = |c: &mut Context, p: Vec2| {
        c.move_pointer(p);
        c.secondary_button(ElementState::Pressed);
        c.secondary_button(ElementState::Released);
    };
    secondary(&mut c, center(s.grand_inner));
    assert_eq!(ids(&c).len(), 3, "inside the leaf: nothing closes");
    secondary(&mut c, center(s.plain));
    assert_eq!(
        ids(&c).len(),
        1,
        "inside an ancestor: its descendants close"
    );
    secondary(&mut c, center(s.under));
    assert!(ids(&c).is_empty(), "outside the branch: all of it closes");
}

#[test]
fn escape_closes_one_level_per_press_and_its_repeat_does_not_fall_through() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    assert!(
        c.on_key_event(KeyCode::Escape, ElementState::Pressed, false)
            .consumed
    );
    assert_eq!(ids(&c).len(), 2);
    for _ in 0..5 {
        assert!(
            c.on_key_event(KeyCode::Escape, ElementState::Pressed, true)
                .consumed
        );
    }
    assert_eq!(
        ids(&c).len(),
        2,
        "autorepeat of the held key closes nothing more"
    );
    assert!(
        c.on_key_event(KeyCode::Escape, ElementState::Released, false)
            .consumed
    );
    assert!(key(&mut c, KeyCode::Escape));
    assert_eq!(ids(&c).len(), 1);
    assert!(key(&mut c, KeyCode::Escape));
    assert!(ids(&c).is_empty());
    assert!(
        !key(&mut c, KeyCode::Escape),
        "nothing is open: Escape is not taken"
    );
}

#[test]
fn escape_returns_focus_to_the_control_that_opened_the_closed_level() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain_focused(&mut c, &mut s, 3);
    let grand = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.rect == s.grand_btn)
        .unwrap()
        .id;
    let child = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.rect == s.child_btn)
        .unwrap()
        .id;
    key(&mut c, KeyCode::Escape);
    pass(&mut c, &mut s);
    assert_eq!(c.probe().focused_widget, Some(grand));
    key(&mut c, KeyCode::Escape);
    pass(&mut c, &mut s);
    assert_eq!(c.probe().focused_widget, Some(child));
}

#[test]
fn closing_the_whole_branch_restores_focus_once_on_the_outer_trigger() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain_focused(&mut c, &mut s, 3);
    let outer = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.rect == s.a_btn)
        .unwrap()
        .id;
    c.close_popup_branch();
    pass(&mut c, &mut s);
    assert_eq!(c.probe().focused_widget, Some(outer));
}

#[test]
fn tab_closes_the_leaf_and_moves_focus_in_what_remains() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    assert!(
        !c.on_key_event(KeyCode::Tab, ElementState::Pressed, false)
            .consumed
            || true
    );
    c.on_key_event(KeyCode::Tab, ElementState::Released, false);
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 2, "only the leaf closes");
    let focused = c.probe().focused_widget.unwrap();
    let hits = c.probe().previous_hits;
    let hit = hits.iter().find(|h| h.id == focused).unwrap();
    let popups = c.probe().popups;
    assert!(
        popups.iter().any(|p| p.id == hit.window),
        "focus stays inside the remaining branch"
    );
}

#[test]
fn keys_between_two_redraws_never_reach_a_closed_layer() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    assert!(key(&mut c, KeyCode::Escape));
    assert!(key(&mut c, KeyCode::Escape));
    assert_eq!(
        ids(&c).len(),
        1,
        "two presses before any redraw close two levels"
    );
    assert!(key(&mut c, KeyCode::Escape));
    assert!(ids(&c).is_empty());
    pass(&mut c, &mut s);
    assert!(!s.a);
}

#[test]
fn a_combo_box_inside_a_popup_closes_only_itself() {
    let mut c = setup();
    let mut s = Scene::new();
    s.combo_in_a = true;
    open_chain(&mut c, &mut s, 1);
    tap(&mut c, &mut s, |s| s.combo_rect);
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 2, "the list is a child of the editor popup");
    let row = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| {
            h.window == c.probe().popup.as_ref().unwrap().id && h.action == HitAction::Activate
        })
        .map(|h| center(h.rect.intersect(h.clip)))
        .expect("an option row");
    click(&mut c, &mut s, row);
    pass(&mut c, &mut s);
    assert!(s.combo.is_some(), "the choice changed the model");
    assert_eq!(ids(&c).len(), 1, "the editor popup is still open");
    tap(&mut c, &mut s, |s| s.plain);
    assert_eq!(s.plain_clicks, 1, "and usable");
}

#[test]
fn escape_closes_the_popup_in_a_modal_before_the_modal() {
    let mut c = setup();
    let mut s = Scene::new();
    s.modal = true;
    settle(&mut c, &mut s);
    tap(&mut c, &mut s, |s| s.modal_btn);
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 1);
    key(&mut c, KeyCode::Escape);
    pass(&mut c, &mut s);
    assert!(ids(&c).is_empty());
    assert!(s.modal, "the modal stays");
    key(&mut c, KeyCode::Escape);
    pass(&mut c, &mut s);
    assert!(!s.modal);
}
