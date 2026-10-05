use super::*;

#[test]
fn tree_keyboard_active_descendant_tab_and_one_shot_activation() {
    let mut c = setup();
    let m = Model::new(1000);
    let first = tree(&mut c, &m, |t| t.default_open([id(0)]));
    c.request_focus(first.id);
    key(&mut c, KeyCode::ArrowDown);
    assert_eq!(tree(&mut c, &m, |t| t).focused, Some(id(1)));
    key(&mut c, KeyCode::ArrowDown);
    assert_eq!(tree(&mut c, &m, |t| t).focused, Some(id(3))); // Disabled 2 skipped.
    key(&mut c, KeyCode::End);
    let out = tree(&mut c, &m, |t| t);
    assert_eq!(out.focused, Some(id(1000)));
    assert!(out.scroll_offset.y > 1000.0);
    assert_eq!(c.probe().focused_widget, Some(out.id));
    assert_eq!(out.selected, None);
    key(&mut c, KeyCode::Space);
    assert_eq!(tree(&mut c, &m, |t| t).selected, Some(id(1000)));
    c.on_key_event(KeyCode::Enter, ElementState::Pressed, false);
    c.on_key_event(KeyCode::Enter, ElementState::Pressed, true);
    c.on_key_event(KeyCode::Enter, ElementState::Released, false);
    assert_eq!(
        tree(&mut c, &m, |t| t).events,
        vec![TreeEvent::Activated { node: id(1000) }]
    );
    assert!(tree(&mut c, &m, |t| t).events.is_empty());
    key(&mut c, KeyCode::Home);
    assert_eq!(
        tree(&mut c, &m, |t| t.selection_follows_focus(true)).selected,
        Some(id(0))
    );
    let mut after = None;
    c.run(|c| {
        Root::new().show(c, |ui| {
            ui.button("Before");
            TreeView::new("tree")
                .default_open([id(0)])
                .max_height(120.0)
                .show_with_actions(ui, &m, |ui, _| {
                    ui.button("Action");
                });
            after = Some(ui.button("After").id);
        });
    });
    key(&mut c, KeyCode::Tab);
    assert_eq!(c.probe().focused_widget, after);
}

#[test]
fn tree_controlled_states_and_row_actions_do_not_leak() {
    let mut c = setup();
    let m = Model::new(4);
    let mut open = HashSet::new();
    let mut selected = None;
    let mut action = None;
    let mut clicks = 0;
    let mut render = |c: &mut Context, open: &mut HashSet<Id>, selected: &mut Option<Id>| {
        let mut out = None;
        c.run(|c| {
            Root::new().show(c, |ui| {
                out = Some(
                    TreeView::new("tree")
                        .open(open)
                        .selected(selected)
                        .show_with_actions(ui, &m, |ui, _| {
                            let response = ui.button("Edit");
                            action = Some(response.rect);
                            if response.clicked() {
                                clicks += 1;
                            }
                        }),
                );
            });
        });
        out.unwrap()
    };
    let out = render(&mut c, &mut open, &mut selected);
    c.request_focus(out.id);
    key(&mut c, KeyCode::ArrowRight);
    let out = render(&mut c, &mut open, &mut selected);
    assert!(open.contains(&id(0)));
    assert_eq!(out.events.len(), 1);
    open.clear();
    assert_eq!(render(&mut c, &mut open, &mut selected).logical_rows, 1);
    // Get action geometry from actual hit regions to avoid depending on local layout.
    let rect = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::Activate)
        .unwrap()
        .rect;
    click(&mut c, rect);
    let out = render(&mut c, &mut open, &mut selected);
    assert_eq!(out.selected, None);
    assert!(out.events.is_empty());
    assert!(open.is_empty());
    drop(render);
    assert_eq!(clicks, 1);
}

#[test]
fn tree_double_click_leaf_and_branch_policy() {
    let mut c = setup();
    let m = Model::new(3);
    tree(&mut c, &m, |t| t.default_open([id(0)]));
    // The first gesture after a long idle must use event time, not the stale pass time.
    c.probe().frame_time -= std::time::Duration::from_secs(5);
    click_node(&mut c, id(1), false);
    tree(&mut c, &m, |t| t);
    click_node(&mut c, id(1), false);
    assert!(tree(&mut c, &m, |t| t)
        .events
        .contains(&TreeEvent::Activated { node: id(1) }));
    click_node(&mut c, id(0), false);
    tree(&mut c, &m, |t| t.expand_on_double_click(true));
    click_node(&mut c, id(0), false);
    assert_eq!(
        tree(&mut c, &m, |t| t.expand_on_double_click(true)).logical_rows,
        1
    );
}
