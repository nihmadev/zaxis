use super::*;

#[test]
fn collapsing_restores_focus_to_locked_header() {
    let mut c = setup();
    let mut open = true;
    let render = |c: &mut Context, open: &mut bool| {
        let mut out = None;
        c.run(|c| {
            Root::new().show(c, |ui| {
                out = Some(
                    CollapsingHeader::new("locked", "Locked")
                        .open(open)
                        .expandable(false)
                        .show(ui, |ui| ui.button("Child")),
                );
            })
        });
        out.unwrap()
    };
    let out = render(&mut c, &mut open);
    c.request_focus(out.body.unwrap().id);
    open = false;
    let out = render(&mut c, &mut open);
    assert_eq!(c.focused_widget, Some(out.header_response.id));
    assert!(!out.open && !out.changed);
    assert!(
        c.on_key_event(KeyCode::Space, ElementState::Pressed, false)
            .repaint
    );
    c.on_key_event(KeyCode::Space, ElementState::Released, false);
    assert!(!render(&mut c, &mut open).open);
}

#[test]
fn parentless_adapter_restores_nearest_visible_cached_ancestor() {
    struct Parentless<'a>(&'a Model);
    impl TreeModel for Parentless<'_> {
        fn revision(&self) -> u64 {
            self.0.revision()
        }
        fn roots(&self) -> impl Iterator<Item = Id> {
            self.0.roots()
        }
        fn children(&self, id: Id) -> impl Iterator<Item = Id> {
            self.0.children(id)
        }
        fn node(&self, id: Id) -> Option<TreeNode<'_>> {
            self.0.node(id)
        }
    }
    let mut c = setup();
    let mut m = Model::new(5);
    m.roots.push(id(5));
    m.nodes.get_mut(&id(0)).unwrap().1 = vec![id(1)];
    m.nodes.get_mut(&id(1)).unwrap().1 = vec![id(3)];
    m.nodes.get_mut(&id(1)).unwrap().3 = TreeChildren::Loaded;
    let mut open = HashSet::from([id(0), id(1)]);
    let render = |c: &mut Context, open: &mut HashSet<Id>| {
        let mut out = None;
        c.run(|c| {
            Root::new().show(c, |ui| {
                out = Some(TreeView::new("tree").open(open).show(ui, &Parentless(&m)));
            })
        });
        out.unwrap()
    };
    let out = render(&mut c, &mut open);
    c.request_focus(out.id);
    key(&mut c, KeyCode::ArrowRight);
    render(&mut c, &mut open);
    key(&mut c, KeyCode::ArrowRight);
    assert_eq!(render(&mut c, &mut open).focused, Some(id(3)));
    open.clear();
    assert_eq!(render(&mut c, &mut open).focused, Some(id(0)));
}

#[test]
fn split_tree_action_hits_follow_the_painted_right_edge() {
    let mut c = setup();
    let m = Model::new(100);
    c.set_viewport(PhysicalSize::new(1225, 875), 1.25);
    let mut rows = HashMap::new();
    let mut out = None;
    let mut edits = 0;
    let mut render = |c: &mut Context| {
        c.run(|c| {
            Root::new().show(c, |ui| {
                ui.button("Toolbar");
                crate::SplitPane::horizontal("split")
                    .panels([
                        crate::SplitPanel::new("tree"),
                        crate::SplitPanel::new("other"),
                    ])
                    .show(ui, |split| {
                        split.panel("tree", |ui| {
                            ui.horizontal(|ui| {
                                ui.button("Add");
                                ui.button("Reverse");
                                ui.button("Reveal last");
                            });
                            out = Some(
                                TreeView::new("tree")
                                    .default_open([id(0)])
                                    .max_height(300.0)
                                    .show_with_actions(ui, &m, |ui, node| {
                                        let r = ui.button("Edit");
                                        rows.insert(node, r.id);
                                        if r.clicked() {
                                            edits += 1;
                                        }
                                    }),
                            );
                        });
                        split.panel("other", |ui| {
                            ui.button("Other");
                        });
                    });
            });
        });
    };
    render(&mut c);
    let row = hit(&c, id(1), false);
    let action = c
        .previous_hits
        .iter()
        .find(|h| {
            h.action == HitAction::Activate && h.rect.min.y >= row.min.y && h.rect.min.y < row.max.y
        })
        .unwrap();
    assert!(
        action.rect.min.x >= row.max.x - 1.0,
        "action {:?}, row {:?}",
        action.rect,
        row
    );
    click(&mut c, row);
    render(&mut c);
    drop(render);
    assert_eq!(edits, 0);
    assert_eq!(out.unwrap().selected, Some(id(1)));
}

#[test]
fn disabled_branches_keep_accessible_children_and_deleted_focus_falls_back() {
    let mut c = setup();
    let mut m = Model::new(5);
    m.nodes.get_mut(&id(0)).unwrap().2 = false;
    let out = tree(&mut c, &m, |t| t.default_open([id(0)]));
    assert_eq!(out.focused, Some(id(1)));
    c.request_focus(out.id);
    key(&mut c, KeyCode::Home);
    assert_eq!(tree(&mut c, &m, |t| t).focused, Some(id(1)));
    key(&mut c, KeyCode::ArrowDown);
    tree(&mut c, &m, |t| t);
    m.nodes.remove(&id(3));
    m.nodes.get_mut(&id(0)).unwrap().1.retain(|n| *n != id(3));
    m.revision += 1;
    assert_eq!(tree(&mut c, &m, |t| t).focused, Some(id(4)));
    // Programmatic expansion/reveal is permitted for disabled ancestors.
    let out = tree(&mut c, &m, |t| t.reveal_node(id(5)));
    assert_eq!(out.focused, Some(id(5)));
}

#[test]
fn node_move_reorder_and_hidden_expansion_preserve_state() {
    let mut c = setup();
    let mut m = Model::new(5);
    m.nodes.get_mut(&id(1)).unwrap().3 = TreeChildren::Loaded;
    m.nodes.get_mut(&id(0)).unwrap().1.retain(|n| *n != id(4));
    m.nodes.get_mut(&id(1)).unwrap().1.push(id(4));
    m.nodes.get_mut(&id(4)).unwrap().0 = Some(id(1));
    let out = tree(&mut c, &m, |t| {
        t.default_open([id(0), id(1)])
            .default_selected(Some(id(4)))
            .reveal_node(id(4))
    });
    assert_eq!(out.selected, Some(id(4)));
    m.nodes.get_mut(&id(1)).unwrap().1.clear();
    m.nodes.get_mut(&id(0)).unwrap().1.push(id(4));
    m.nodes.get_mut(&id(4)).unwrap().0 = Some(id(0));
    m.revision += 1;
    let out = tree(&mut c, &m, |t| t);
    assert_eq!(out.selected, Some(id(4)));
    assert_eq!(out.focused, Some(id(4)));
    let mut open = HashSet::new();
    let out = tree(&mut c, &m, |t| t.open(&mut open));
    assert_eq!(out.selected, Some(id(4)));
    // Removing controlled binding leaves the last authoritative state as retained value.
    assert_eq!(tree(&mut c, &m, |t| t).logical_rows, 1);
}

#[test]
fn text_action_owns_keys_and_tab_leaves_tree() {
    let mut c = setup();
    let m = Model::new(3);
    let mut text = "test".to_string();
    let mut owner = None;
    let mut input = None;
    let mut after = None;
    c.run(|c| {
        Root::new().show(c, |ui| {
            let out = TreeView::new("tree")
                .default_open([id(0)])
                .max_height(120.0)
                .show_with_actions(ui, &m, |ui, n| {
                    if n == id(1) {
                        input = Some(ui.text_edit(&mut text).id);
                    }
                });
            owner = Some(out.id);
            after = Some(ui.button("After").id);
        });
    });
    c.request_focus(input.unwrap());
    key(&mut c, KeyCode::ArrowLeft);
    key(&mut c, KeyCode::Space);
    assert!(c.tree_input.get(&owner.unwrap()).is_none());
    assert_eq!(c.focused_widget, input);
    key(&mut c, KeyCode::Tab);
    assert_eq!(c.focused_widget, after);
}

#[test]
fn theme_overrides_zero_transparency_and_clipping_are_preserved() {
    let mut theme = crate::Theme::light().density(crate::Density::Compact);
    theme.overrides.tree.indent = Some(0.0);
    theme.overrides.tree.guides = Some(crate::Border::NONE);
    theme.overrides.tree.row.chevron_stroke = Some(0.0);
    theme.overrides.tree.row.surface.idle = crate::SurfaceStyle::fill(crate::Color::TRANSPARENT);
    theme.overrides.collapsing.header.surface.idle.border = Some(crate::Border::NONE);
    let style = theme.resolve();
    assert_eq!(style.tree.indent, Some(0.0));
    assert_eq!(style.tree.guides, Some(crate::Border::NONE));
    assert_eq!(
        style.tree.row.surface.idle.fill.unwrap().start,
        crate::Color::TRANSPARENT
    );
    assert_eq!(
        style.collapsing.header.surface.idle.border,
        Some(crate::Border::NONE)
    );
    let mut c = setup();
    let m = Model::new(1000);
    let mut built = 0;
    c.run(|c| {
        Root::new().show(c, |ui| {
            ui.with_width(40.0, |ui| {
                ui.with_theme(&theme, |ui| {
                    TreeView::new("tree")
                        .default_open([id(0)])
                        .max_height(80.0)
                        .show_with_actions(ui, &m, |ui, _| {
                            built += 1;
                            ui.button("Long action caption");
                        });
                })
            });
        });
    });
    assert!(built <= 6);
    assert!(c
        .previous_hits
        .iter()
        .all(|h| h.rect.size().min_element() >= 0.0));
    assert!(c
        .draw_data()
        .commands
        .iter()
        .all(|command| command.clip_rect.size().min_element() >= 0.0));
}

#[test]
fn lazy_model_arrival_completes_pending_reveal_once() {
    let mut c = setup();
    let mut m = Model::new(10);
    m.nodes.get_mut(&id(0)).unwrap().3 = TreeChildren::Unloaded;
    m.nodes.get_mut(&id(0)).unwrap().1.clear();
    let out = tree(&mut c, &m, |t| t.reveal_node(id(8)));
    assert_eq!(out.logical_rows, 1);
    assert_eq!(
        out.events
            .iter()
            .filter(|e| matches!(e, TreeEvent::RequestChildren { .. }))
            .count(),
        1
    );
    assert!(tree(&mut c, &m, |t| t).events.is_empty());
    m.nodes.get_mut(&id(0)).unwrap().3 = TreeChildren::Loaded;
    m.nodes.get_mut(&id(0)).unwrap().1 = (1..=10).map(id).collect();
    m.revision += 1;
    let out = tree(&mut c, &m, |t| t);
    assert_eq!(out.focused, Some(id(8)));
    assert!(out.scroll_offset.y > 0.0);
    assert!(out.events.is_empty());
}
