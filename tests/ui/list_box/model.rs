use super::*;
use zaxis::{DiagnosticKind, Insertion};

#[test]
fn selection_and_active_row_follow_keys_through_insert_remove_reorder_and_filter() {
    let mut l = List::new(numbered(50), ListMode::Multiple);
    l.click(4);
    l.click_with(6, CTRL);
    assert_eq!((l.selected(), l.active()), (vec![4, 6], Some(6)));
    for key in 100..103 {
        l.items.insert(0, item(key, "new"));
    }
    l.frame();
    assert_eq!((l.selected(), l.active()), (vec![4, 6], Some(6)));
    assert_eq!(l.out().active, Some(k(6)));
    l.items.reverse();
    l.frame();
    assert_eq!((l.selected(), l.active()), (vec![4, 6], Some(6)));
    l.items.reverse();
    l.revision += 1;
    l.frame();
    assert_eq!(l.active(), Some(6));
    l.items.retain(|it| it.key % 2 == 0 || it.key >= 100);
    l.frame();
    assert_eq!(
        l.selected(),
        vec![4, 6],
        "filtering keeps what is still listed"
    );
    l.items.retain(|it| it.key != 6);
    l.frame();
    assert_eq!(l.active(), Some(8), "the active row moves to its neighbour");
    assert!(
        l.sel.contains(&k(6)),
        "the application's set is never pruned"
    );
}

#[test]
fn scroll_to_key_reveals_the_row_and_insertions_keep_the_view_anchor() {
    let mut l = List::new(numbered(1000), ListMode::Single);
    l.scroll_to = Some(500);
    l.frame();
    assert!(l.out().visible.contains(&500));
    let before = l.out().visible.start;
    for _ in 0..3 {
        l.frame();
    }
    assert_eq!(l.out().visible.start, before, "no drift once settled");
    for key in 2000..2010 {
        l.items.insert(0, item(key, "new"));
    }
    l.frame();
    assert_eq!(
        l.out().visible.start,
        before + 10,
        "the row on top stays on top"
    );
}

#[test]
fn shrinking_the_set_pulls_the_scroll_position_back() {
    let mut l = List::new(numbered(1000), ListMode::Single);
    l.scroll_to = Some(990);
    l.frame();
    assert!(l.out().scroll_offset.y > 20000.0);
    l.items.truncate(15);
    l.frame();
    l.frame();
    let o = l.out();
    assert!(
        o.scroll_offset.y <= 15.0 * ROW - o.viewport.size().y + 0.5,
        "{:?}",
        o.scroll_offset
    );
    assert!(o.visible.contains(&14));
}

#[test]
fn empty_list_is_safe_for_every_input() {
    let mut l = List::new(Vec::new(), ListMode::Multiple);
    l.key(KeyCode::ArrowDown);
    l.key(KeyCode::End);
    l.key_with(KeyCode::KeyA, CTRL);
    l.type_text("a");
    l.click_at(Vec2::new(40.0, 40.0), ModifiersState::empty());
    let o = l.out();
    assert_eq!((o.len, o.visible.clone(), o.active), (0, 0..0, None));
    l.frame();
    l.frame();
    assert!(l.selected().is_empty() && l.settled());
}

#[test]
fn huge_lists_build_only_the_viewport_and_settle() {
    let mut l = List::new(numbered(300_000), ListMode::Single);
    let built = l.out().rows_built;
    assert!(built <= 14, "built {built} rows");
    l.frame();
    l.frame();
    assert!(l.settled(), "a settled list asks for no redraw");
    assert!(!l.out().rebuilt);
    l.click(0);
    l.key(KeyCode::End);
    assert!(l.out().visible.contains(&299_999));
}

#[test]
fn load_more_is_requested_once_per_length() {
    let count = |l: &List| {
        l.events
            .iter()
            .filter(|e| **e == ListEvent::LoadMore)
            .count()
    };
    let mut l = List::new(numbered(30), ListMode::Single);
    l.more = true;
    l.frame();
    assert!(!l.out().wants_more());
    l.click(2);
    l.key(KeyCode::End);
    assert_eq!(count(&l), 1);
    l.frame();
    l.frame();
    assert_eq!(count(&l), 1);
    l.items.extend((30..60).map(|i| item(i, "more")));
    l.key(KeyCode::End);
    assert_eq!(count(&l), 2);
}

#[test]
fn dragging_a_row_reports_one_move_and_leaves_the_model() {
    let moved = |l: &List| {
        l.events
            .iter()
            .filter(|e| matches!(e, ListEvent::Moved { .. }))
            .copied()
            .collect::<Vec<_>>()
    };
    let mut l = List::new(numbered(10), ListMode::Single);
    l.drag = true;
    l.frame();
    let from = l.at(1);
    l.c.move_pointer(from);
    l.c.primary_button(ElementState::Pressed);
    l.c.move_pointer(from + Vec2::new(0.0, 10.0));
    l.frame();
    l.frame();
    l.c.move_pointer(l.at(4) + Vec2::new(0.0, 8.0));
    l.frame();
    l.c.primary_button(ElementState::Released);
    l.frame();
    assert_eq!(
        moved(&l),
        vec![ListEvent::Moved {
            key: k(1),
            target: k(4),
            position: Insertion::After
        }]
    );
    assert_eq!(l.items.len(), 10);
    assert!(l.selected().is_empty(), "a drag is not a click");
    l.frame();
    assert_eq!(moved(&l).len(), 1);
}

#[test]
fn sticky_header_blocks_rows_under_it() {
    let mut items = numbered(60);
    for at in [0, 20, 40] {
        items[at].header = true;
    }
    let mut l = List::new(items, ListMode::Single);
    l.scroll_to = Some(30);
    l.frame();
    l.frame();
    let o = l.out().viewport;
    l.click_at(
        Vec2::new(o.min.x + 40.0, o.min.y + 4.0),
        ModifiersState::empty(),
    );
    assert!(
        l.selected().is_empty(),
        "the row under the pinned header ignores the click"
    );
    l.click(25);
    assert_eq!(l.selected().len(), 1);
}

#[test]
fn clicks_outside_the_list_are_ignored() {
    let mut l = List::new(numbered(50), ListMode::Single);
    let o = l.out().viewport;
    l.click_at(
        Vec2::new(o.min.x + 40.0, o.max.y + 6.0),
        ModifiersState::empty(),
    );
    assert!(l.selected().is_empty());
    l.click_at(
        Vec2::new(o.max.x + 30.0, o.min.y + 30.0),
        ModifiersState::empty(),
    );
    assert!(l.selected().is_empty());
}

#[test]
fn fractional_scale_keeps_hits_aligned_and_building_bounded() {
    for scale in [1.25_f64, 1.5, 2.0] {
        let mut l = List::with_scale(numbered(500), ListMode::Single, scale);
        let o = l.out().viewport;
        l.click_at(
            Vec2::new(o.min.x + 40.0, o.min.y + ROW * 2.5),
            ModifiersState::empty(),
        );
        assert_eq!(l.selected(), vec![2], "scale {scale}");
        assert!(l.out().rows_built <= 16);
    }
}

#[test]
fn invalid_sizes_are_reported_and_ignored() {
    let mut l = List::new(numbered(5), ListMode::Single);
    let before = l.c.diagnostics().len();
    l.c.run_at(l.c.frame_time() + Duration::from_millis(50), |c| {
        Root::new().show(c, |ui| {
            let items = numbered(3);
            ListBox::new("bad")
                .row_height(f32::NAN)
                .row_height(0.0)
                .max_height(f32::NAN)
                .measured_rows(-1.0)
                .show_slice(
                    ui,
                    &items,
                    |it| ListEntry::item(it.key, &it.text),
                    |ui, _, it| {
                        ui.label(&it.text);
                    },
                );
        });
    });
    let diagnostics = l.c.diagnostics();
    assert!(diagnostics.len() > before);
    assert!(diagnostics
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidValue));
}

#[test]
fn state_is_dropped_with_the_component() {
    let mut l = List::new(numbered(10), ListMode::Single);
    assert_eq!(l.c.probe().list_boxes.len(), 1);
    for _ in 0..2 {
        l.c.run_at(l.c.frame_time() + Duration::from_millis(16), |c| {
            Root::new().show(c, |ui| {
                ui.label("gone");
            });
        });
    }
    assert!(l.c.probe().list_boxes.is_empty());
}

#[test]
fn works_inside_a_window_and_a_clipping_scroll_area() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let items = numbered(100);
    let mut sel = HashSet::new();
    let frame = |c: &mut Context, sel: &mut HashSet<Id>, step: u64| {
        let mut shown = None;
        c.run_at(Instant::now() + Duration::from_millis(step * 16), |c| {
            zaxis::Window::new("Host").show(c, |ui| {
                let outer = zaxis::ScrollArea::vertical()
                    .id_source("outer")
                    .max_height(150.0)
                    .show(ui, |ui| {
                        ui.label("Title");
                        let out = ListBox::new("nested")
                            .selection(sel)
                            .row_height(ROW)
                            .max_height(240.0)
                            .show_slice(
                                ui,
                                &items,
                                |it| ListEntry::item(it.key, &it.text),
                                |ui, _, it| {
                                    ui.label(&it.text);
                                },
                            );
                        ui.add_space(400.0);
                        out
                    });
                shown = Some((outer.inner, outer.viewport));
            });
        });
        shown.unwrap()
    };
    for step in 0..3 {
        frame(&mut c, &mut sel, step);
    }
    let (out, outer) = frame(&mut c, &mut sel, 3);
    let row = |i: f32| {
        Vec2::new(
            out.viewport.min.x + 40.0,
            out.viewport.min.y + (i + 0.5) * ROW,
        )
    };
    c.move_pointer(row(1.0));
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    frame(&mut c, &mut sel, 4);
    assert_eq!(sel.len(), 1, "a click in the visible part selects");
    assert!(
        out.viewport.max.y > outer.max.y,
        "the list is taller than the area that clips it"
    );
    let hidden = Vec2::new(out.viewport.min.x + 40.0, outer.max.y + 30.0);
    sel.clear();
    c.move_pointer(hidden);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    frame(&mut c, &mut sel, 5);
    assert!(
        sel.is_empty(),
        "rows clipped by the outer area ignore the pointer"
    );
}
