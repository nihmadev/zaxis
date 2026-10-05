use super::*;

#[test]
fn tree_defaults_pointer_controls_and_lazy_events() {
    let mut c = setup();
    let mut model = Model::new(10);
    let out = tree(&mut c, &model, |t| t);
    assert_eq!(out.logical_rows, 1);
    assert_eq!(model.visits.get(), 0);
    click_node(&mut c, id(0), true);
    let out = tree(&mut c, &model, |t| t);
    assert_eq!(out.logical_rows, 11);
    assert_eq!(
        out.events,
        vec![TreeEvent::OpenChanged {
            node: id(0),
            open: true
        }]
    );
    assert_eq!(out.selected, None);
    click_node(&mut c, id(1), false);
    let out = tree(&mut c, &model, |t| t);
    assert_eq!(out.events, vec![TreeEvent::Selected { node: Some(id(1)) }]);
    let steady = tree(&mut c, &model, |t| t.default_open([]));
    assert!(!steady.rebuilt);
    assert_eq!(steady.logical_rows, 11);
    assert!(steady.events.is_empty());
    c.move_pointer(hit(&c, id(1), false).center());
    assert!(c.secondary_button(ElementState::Pressed));
    c.secondary_button(ElementState::Released);
    assert_eq!(
        tree(&mut c, &model, |t| t).events,
        vec![TreeEvent::ContextAction { node: id(1) }]
    );
    model.nodes.get_mut(&id(0)).unwrap().3 = TreeChildren::Unloaded;
    model.revision += 1;
    tree(&mut c, &model, |t| t);
    click_node(&mut c, id(0), true);
    tree(&mut c, &model, |t| t);
    click_node(&mut c, id(0), true);
    let out = tree(&mut c, &model, |t| t);
    assert_eq!(
        out.events
            .iter()
            .filter(|e| matches!(e, TreeEvent::RequestChildren { .. }))
            .count(),
        1
    );
    assert!(tree(&mut c, &model, |t| t).events.is_empty());
}

#[test]
fn tree_virtualization_revision_stable_actions_and_theme() {
    let mut c = setup();
    let mut m = Model::new(10_000);
    let mut first = None;
    let build = |c: &mut Context, m: &Model| {
        let mut out = None;
        let mut controls = HashMap::new();
        c.run(|c| {
            Root::new().show(c, |ui| {
                out = Some(
                    TreeView::new("tree")
                        .default_open([id(0)])
                        .max_height(120.0)
                        .show_with_actions(ui, m, |ui, n| {
                            controls.insert(n, ui.button("Edit").id);
                        }),
                );
            });
        });
        (out.unwrap(), controls)
    };
    let (out, controls) = build(&mut c, &m);
    assert!(out.rows_built <= 7);
    assert_eq!(out.logical_rows, 10_001);
    first.replace(controls[&id(1)]);
    m.nodes.get_mut(&id(0)).unwrap().1.swap(0, 2);
    m.revision += 1;
    let (out, controls) = build(&mut c, &m);
    assert!(out.rebuilt);
    assert_eq!(first.unwrap(), controls[&id(1)]);
    let visits = m.visits.get();
    c.set_theme(zaxis::Theme::light());
    let (out, _) = build(&mut c, &m);
    assert!(!out.rebuilt);
    assert_eq!(visits, m.visits.get());
    assert!(out.rows_built <= 7);
    c.run_at(c.frame_time() + std::time::Duration::from_secs(2), |c| {
        Root::new().show(c, |ui| {
            TreeView::new("tree").max_height(120.0).show(ui, &m);
        });
    });
    assert!(!c.needs_repaint());
}

#[test]
fn tree_deletion_hidden_selection_reveal_cycles() {
    let mut c = setup();
    let mut m = Model::new(30);
    let out = tree(&mut c, &m, |t| {
        t.default_selected(Some(id(25))).reveal_node(id(25))
    });
    assert_eq!(out.focused, Some(id(25)));
    assert!(out.scroll_offset.y > 0.0);
    let visits = m.visits.get();
    tree(&mut c, &m, |t| t);
    assert_eq!(visits, m.visits.get());
    let owner = out.id;
    c.request_focus(owner);
    key(&mut c, KeyCode::ArrowLeft);
    tree(&mut c, &m, |t| t);
    key(&mut c, KeyCode::ArrowLeft);
    let out = tree(&mut c, &m, |t| t);
    assert_eq!(out.logical_rows, 1);
    assert_eq!(out.selected, Some(id(25)));
    assert_eq!(out.focused, Some(id(0)));
    m.nodes.remove(&id(25));
    m.revision += 1;
    let out = tree(&mut c, &m, |t| t);
    assert_eq!(out.selected, None);
    assert!(out.events.contains(&TreeEvent::Selected { node: None }));
    m.nodes.get_mut(&id(0)).unwrap().1.extend([id(0), id(1)]);
    m.revision += 1;
    let out = tree(&mut c, &m, |t| t.reveal_node(id(1)));
    assert!(out
        .issues
        .contains(&zaxis::TreeIssue::DuplicateOrCycle(id(0))));
    assert!(out
        .issues
        .contains(&zaxis::TreeIssue::DuplicateOrCycle(id(1))));
}
