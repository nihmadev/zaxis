//! Opening and hierarchy: chains, replacement, ownership, depth.
use super::*;

#[test]
fn a_chain_opens_without_closing_its_parents() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    let popups = c.probe().popups;
    assert_eq!(popups.len(), 3);
    assert_eq!(popups[0].parent, None);
    assert_eq!(popups[1].parent, Some(popups[0].id));
    assert_eq!(popups[2].parent, Some(popups[1].id));
    assert!(popups.iter().all(|p| p.owner == popups[0].owner));
    assert!(s.a && s.child && s.grand);
    assert!(s.child_rect.is_some() && s.grand_rect.is_some());
}

#[test]
fn a_sibling_replaces_its_sibling_and_keeps_the_parent() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 2);
    let first = ids(&c);
    s.child2 = true;
    pass(&mut c, &mut s);
    let now = ids(&c);
    assert_eq!(now.len(), 2);
    assert_eq!(now[0], first[0]);
    assert_ne!(now[1], first[1]);
    pass(&mut c, &mut s);
    assert!(
        s.child2 && !s.child,
        "the replaced sibling hears of it once"
    );
    pass(&mut c, &mut s);
    assert!(!s.child, "and does not reopen");
}

#[test]
fn another_root_replaces_the_whole_branch_once() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    // The press on the other root's trigger is outside the branch: it closes it.
    tap(&mut c, &mut s, |s| s.b_btn);
    assert!(ids(&c).is_empty());
    tap(&mut c, &mut s, |s| s.b_btn);
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 1);
    assert!(s.b && !s.a, "open flags follow the dismissal");
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 1);
    s.a = true;
    s.child = false;
    s.grand = false;
    pass(&mut c, &mut s);
    let now = ids(&c);
    assert_eq!(
        now.len(),
        1,
        "an application opening a root replaces the other"
    );
    pass(&mut c, &mut s);
    assert!(!s.b);
}

#[test]
fn closing_the_leaf_keeps_ancestors_and_closing_a_parent_closes_children() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    c.close_popup();
    assert_eq!(ids(&c).len(), 2);
    pass(&mut c, &mut s);
    assert!(!s.grand && s.child && s.a);
    s.child = false;
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 1);
    open_chain(&mut c, &mut s, 3);
    c.close_popup_branch();
    assert!(ids(&c).is_empty());
    pass(&mut c, &mut s);
    assert!(!s.a && !s.child && !s.grand || !s.a);
}

#[test]
fn the_same_source_in_two_scopes_does_not_collide() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    let grand = ids(&c)[2];
    s.child = false;
    s.child2 = true;
    s.grand = false;
    pass(&mut c, &mut s);
    s.grand = true;
    pass(&mut c, &mut s);
    assert_eq!(ids(&c).len(), 2);
    assert_ne!(ids(&c)[1], grand);
    assert!(c
        .diagnostics()
        .iter()
        .all(|d| d.kind != DiagnosticKind::IdCollision));
}

fn nested(ui: &mut zaxis::Ui<'_>, level: usize, limit: usize, opened: &mut Vec<bool>) {
    if level >= limit {
        return;
    }
    let anchor = rect(
        10.0 + level as f32 * 6.0,
        10.0 + level as f32 * 30.0,
        60.0,
        20.0,
    );
    let mut open = true;
    Popup::new("level", anchor)
        .size(vec2(220.0, 200.0))
        .show(ui, &mut open, |ui| nested(ui, level + 1, limit, opened));
    opened.push(open);
}

#[test]
fn depth_is_limited_with_a_diagnostic_and_nothing_below_is_lost() {
    let mut c = setup();
    let limit = zaxis::MAX_POPUP_DEPTH;
    let mut opened = Vec::new();
    for _ in 0..3 {
        opened.clear();
        c.run(|c| {
            Root::new().show(c, |ui| nested(ui, 0, limit + 2, &mut opened));
        });
    }
    assert_eq!(c.probe().popups.len(), limit);
    assert!(c
        .probe()
        .popups
        .windows(2)
        .all(|w| w[1].parent == Some(w[0].id)));
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidUsage));
}

#[test]
fn a_popup_built_in_a_modal_belongs_to_the_modal() {
    let mut c = setup();
    let mut s = Scene::new();
    s.modal = true;
    settle(&mut c, &mut s);
    tap(&mut c, &mut s, |s| s.modal_btn);
    pass(&mut c, &mut s);
    let popups = c.probe().popups;
    assert_eq!(popups.len(), 1);
    assert_eq!(Some(popups[0].owner), c.top_modal_id());
    // A popup built outside the top modal is refused.
    s.a = true;
    pass(&mut c, &mut s);
    assert!(!s.a);
    assert_eq!(c.probe().popups.len(), 1);
}

#[test]
fn a_whole_chain_can_open_in_one_pass() {
    let mut c = setup();
    let mut opened = Vec::new();
    c.run(|c| {
        Root::new().show(c, |ui| nested(ui, 0, 3, &mut opened));
    });
    assert_eq!(c.probe().popups.len(), 3);
}
