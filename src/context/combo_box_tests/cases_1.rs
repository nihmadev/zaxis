use super::*;

#[test]
fn keyboard_long_list_disabled_rows_reveal_and_real_changes() {
    let mut c = setup();
    let mut selected = Some(80);
    let mut opts = options(100);
    opts[81].enabled = false;
    let r = draw(&mut c, &mut selected, &opts, false, false);
    key(&mut c, KeyCode::Tab);
    key(&mut c, KeyCode::ArrowDown);
    draw(&mut c, &mut selected, &opts, false, false);
    assert!(c.popup.is_some());
    let hit = row_hit(&c, r.id, opts[80].id);
    assert!(!hit.rect.intersect(hit.clip).is_empty());
    assert!(
        c.previous_hits
            .iter()
            .filter(|h| h.window == c.popup.as_ref().unwrap().id)
            .count()
            < 12
    );
    key(&mut c, KeyCode::ArrowDown);
    key(&mut c, KeyCode::Enter);
    let changed = draw(&mut c, &mut selected, &opts, false, false);
    assert_eq!(selected, Some(82));
    assert!(changed.changed());
    assert!(c.popup.is_none());
    assert_eq!(c.focused_widget, Some(r.id));
    key(&mut c, KeyCode::Enter);
    draw(&mut c, &mut selected, &opts, false, false);
    key(&mut c, KeyCode::Enter);
    assert!(!draw(&mut c, &mut selected, &opts, false, false).changed());
    key(&mut c, KeyCode::ArrowUp);
    draw(&mut c, &mut selected, &opts, false, false);
    key(&mut c, KeyCode::End);
    draw(&mut c, &mut selected, &opts, false, false);
    let hit = row_hit(&c, r.id, opts[99].id);
    assert!(!hit.rect.intersect(hit.clip).is_empty());
    key(&mut c, KeyCode::Escape);
    draw(&mut c, &mut selected, &opts, false, false);
    assert_eq!(selected, Some(82));
    assert!(c.popup.is_none());
}

#[test]
fn duplicates_labels_reordering_removal_empty_and_disable_while_open() {
    let mut c = setup();
    let mut selected = Some(1);
    let mut opts = options(4);
    for o in &mut opts {
        o.label = "Same ## literal".into();
    }
    let r = draw(&mut c, &mut selected, &opts, false, false);
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, false, false);
    key(&mut c, KeyCode::ArrowDown);
    draw(&mut c, &mut selected, &opts, false, false);
    assert_eq!(c.combo_boxes[&r.id].active, Some(opts[2].id));
    opts.reverse();
    draw(&mut c, &mut selected, &opts, false, false);
    assert_eq!(c.combo_boxes[&r.id].active, Some(Id::new(2_usize)));
    let hit = row_hit(&c, r.id, Id::new(3_usize));
    click(&mut c, hit.rect.intersect(hit.clip).center());
    assert!(draw(&mut c, &mut selected, &opts, false, false).changed());
    assert_eq!(selected, Some(3));
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, false, false);
    opts.retain(|o| o.value != 3);
    assert!(!draw(&mut c, &mut selected, &opts, false, false).changed());
    assert_eq!(selected, Some(3));
    assert_eq!(c.combo_boxes[&r.id].active, Some(opts[0].id));
    draw(&mut c, &mut selected, &opts, false, true);
    assert!(c.popup.is_none());
    assert!(!c
        .previous_hits
        .iter()
        .any(|h| h.id == r.id && h.action.focusable()));
    opts.clear();
    assert!(!draw(&mut c, &mut selected, &opts, false, false).changed());
    assert_eq!(selected, Some(3));
}

#[test]
fn filtering_uses_text_edit_no_matches_does_not_reset_query() {
    let mut c = setup();
    let mut selected = Some(0);
    let opts = options(100);
    let r = draw(&mut c, &mut selected, &opts, true, false);
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, true, false);
    assert!(c
        .previous_hits
        .iter()
        .any(|h| Some(h.id) == c.focused_widget && h.action == HitAction::TextEdit));
    assert!(c.on_text_event("99").consumed);
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.combo_boxes[&r.id].active, Some(opts[99].id));
    key(&mut c, KeyCode::Enter);
    assert!(draw(&mut c, &mut selected, &opts, true, false).changed());
    assert_eq!(selected, Some(99));
    assert_eq!(c.focused_widget, Some(r.id));
    key(&mut c, KeyCode::Enter);
    draw(&mut c, &mut selected, &opts, true, false);
    c.on_text_event("missing");
    draw(&mut c, &mut selected, &opts, true, false);
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.combo_boxes[&r.id].query, "missing");
    assert_eq!(c.combo_boxes[&r.id].active, None);
    key(&mut c, KeyCode::Enter);
    assert!(!draw(&mut c, &mut selected, &opts, true, false).changed());
    assert!(c.popup.is_some());
    key(&mut c, KeyCode::Escape);
    key(&mut c, KeyCode::Enter);
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.combo_boxes[&r.id].query, "");
    assert!(c.popup.is_some());
    key(&mut c, KeyCode::Tab);
    assert!(c.popup.is_none());
    assert_ne!(c.focused_widget, Some(r.id));
}

#[test]
fn overlay_above_later_window_and_outside_press_never_clicks_through() {
    let mut c = setup();
    let mut selected = Some(0);
    let opts = options(10);
    let mut lower_clicked = false;
    let mut show = |c: &mut Context| {
        Window::new("test").show(c, |ui| {
            ui.add(ComboBox::new(&mut selected, &opts).id_source("combo"));
        });
        Window::new("overlap")
            .default_position(vec2(40.0, 150.0))
            .show(c, |ui| {
                lower_clicked = ui.button("Lower").clicked();
            });
    };
    c.run(&mut show);
    let trigger = c
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::ComboBox)
        .unwrap()
        .rect;
    click(&mut c, trigger.center());
    c.run(&mut show);
    let popup = c.popup.as_ref().unwrap();
    let popup_id = popup.id;
    let point = popup.rect.center();
    assert_eq!(c.top_window(point), Some(popup_id));
    assert_eq!(c.elements.last().unwrap().layer, popup_id);
    assert!(c.hit_test(point).is_some_and(|h| h.window == popup_id));
    c.move_pointer(point);
    assert!(c.scroll_wheel(vec2(0.0, 44.0)));
    c.move_pointer(vec2(80.0, 300.0));
    assert!(c.scroll_wheel(vec2(0.0, 44.0)));
    c.move_pointer(vec2(80.0, 300.0));
    assert!(c.primary_button(ElementState::Pressed));
    assert!(c.popup.is_none());
    c.primary_button(ElementState::Released);
    assert!(c.clicked.is_empty());
    c.run(&mut show);
    assert!(!lower_clicked);
    assert!(!c.previous_hits.iter().any(|h| h.window == popup_id));
}

#[test]
fn popup_escapes_scroll_clip_and_fits_bottom_right_edge() {
    let mut c = setup();
    let mut selected = Some(70);
    let opts = options(100);
    let mut response = None;
    c.run(|c| {
        Window::new("edge")
            .default_position(vec2(580.0, 350.0))
            .default_size(vec2(300.0, 140.0))
            .show(c, |ui| {
                ScrollArea::vertical().max_height(30.0).show(ui, |ui| {
                    response = Some(
                        ui.add(
                            ComboBox::new(&mut selected, &opts)
                                .id_source("edge")
                                .default_open(true),
                        ),
                    );
                });
            });
    });
    let popup = c.popup.as_ref().unwrap();
    assert!(popup.rect.max.x <= c.viewport().max.x);
    assert!(popup.rect.max.y <= response.unwrap().rect.min.y);
    assert!(popup.rect.min.x >= c.viewport().min.x);
    let hit = row_hit(&c, response.unwrap().id, opts[70].id);
    assert!(!hit.rect.intersect(hit.clip).is_empty());
    assert!(hit.clip.min.y < response.unwrap().rect.min.y);
    assert_eq!(
        c.hit_test(hit.rect.intersect(hit.clip).center())
            .unwrap()
            .id,
        hit.id
    );
}

#[test]
fn rapid_retarget_closing_removes_hits_immediately_and_animation_sleeps() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.motion.reduced_motion = false;
    c.set_style(style);
    let mut selected = Some(0);
    let opts = options(10);
    let start = Instant::now();
    let render = |c: &mut Context, selected: &mut Option<usize>, time: Instant| {
        c.run_at(time, |c| {
            Window::new("test").show(c, |ui| {
                ui.add(ComboBox::new(selected, &opts).id_source("combo"));
            });
        });
    };
    render(&mut c, &mut selected, start);
    let hit = *c
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::ComboBox)
        .unwrap();
    click(&mut c, hit.rect.center());
    render(&mut c, &mut selected, start + Duration::from_millis(10));
    render(&mut c, &mut selected, start + Duration::from_millis(70));
    assert!(c.next_repaint().is_some());
    let popup_id = c.popup.as_ref().unwrap().id;
    key(&mut c, KeyCode::Escape);
    assert!(!c.previous_hits.iter().any(|h| h.window == popup_id));
    render(&mut c, &mut selected, start + Duration::from_millis(80));
    assert!(!c.previous_hits.iter().any(|h| h.window == popup_id));
    key(&mut c, KeyCode::Enter);
    render(&mut c, &mut selected, start + Duration::from_millis(90));
    assert!(c.popup.is_some());
    key(&mut c, KeyCode::Escape);
    render(&mut c, &mut selected, start + Duration::from_millis(100));
    render(&mut c, &mut selected, start + Duration::from_millis(400));
    assert!(c.popup.is_none());
    assert!(!c.previous_hits.iter().any(|h| h.window == popup_id));
    assert!(c.next_repaint().is_none());
    assert!(!c.needs_repaint_at(start + Duration::from_millis(400)));
    let geometry = c.draw_data().revision;
    render(&mut c, &mut selected, start + Duration::from_millis(500));
    assert_eq!(c.draw_data().revision, geometry);
}

#[test]
fn generic_popup_is_not_window_restores_focus_and_disappears_with_owner() {
    let mut c = setup();
    let mut open = false;
    let mut trigger_id = None;
    let mut show = |c: &mut Context| {
        Window::new("generic").show(c, |ui| {
            let trigger = ui.button("Open");
            if trigger.clicked() {
                open = true;
            }
            trigger_id = Some(trigger.id);
            Popup::new("generic-menu", trigger.rect)
                .return_focus(trigger.id)
                .show(ui, &mut open, |ui| {
                    ui.button("Item");
                });
        });
    };
    c.run(&mut show);
    key(&mut c, KeyCode::Tab);
    key(&mut c, KeyCode::Enter);
    c.run(&mut show);
    assert!(c.popup.is_some());
    assert_eq!(c.windows.len(), 1);
    key(&mut c, KeyCode::Escape);
    c.run(&mut show);
    assert_eq!(c.focused_widget, trigger_id);
    assert!(!open);
    let point = c
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::Activate)
        .unwrap()
        .rect
        .center();
    click(&mut c, point);
    c.run(|c| {
        Window::new("generic").show(c, |ui| {
            let trigger = ui.button("Open");
            open = trigger.clicked();
            Popup::new("generic-menu", trigger.rect)
                .return_focus(trigger.id)
                .show(ui, &mut open, |_| {});
        });
    });
    assert!(c.popup.is_some());
    c.run(|_| {});
    assert!(c.popup.is_none());
    assert!(c.previous_hits.is_empty());
}

#[test]
fn clicking_the_trigger_again_closes_the_popup() {
    let mut c = setup();
    let mut selected = Some(0);
    let opts = options(10);
    let r = draw(&mut c, &mut selected, &opts, false, false);
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, false, false);
    assert!(c.popup.is_some());
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, false, false);
    draw(&mut c, &mut selected, &opts, false, false);
    assert!(c.popup.is_none());
    assert!(!c.combo_boxes[&r.id].open);
}
