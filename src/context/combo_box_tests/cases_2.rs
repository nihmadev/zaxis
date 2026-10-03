use super::*;

#[test]
fn animated_filter_gains_focus_after_reveal_and_empty_options_close() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.motion.reduced_motion = false;
    c.set_style(style);
    let mut opts = options(100);
    let mut selected = Some(30);
    let start = Instant::now();
    let render =
        |c: &mut Context, selected: &mut Option<usize>, opts: &[ComboBoxOption<usize>], time| {
            c.run_at(time, |c| {
                Window::new("animated filter").show(c, |ui| {
                    ui.add(
                        ComboBox::new(selected, opts)
                            .id_source("filter")
                            .filterable(true),
                    );
                });
            });
        };
    render(&mut c, &mut selected, &opts, start);
    let trigger = *c
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::ComboBox)
        .unwrap();
    click(&mut c, trigger.rect.center());
    render(
        &mut c,
        &mut selected,
        &opts,
        start + Duration::from_millis(10),
    );
    render(
        &mut c,
        &mut selected,
        &opts,
        start + Duration::from_millis(40),
    );
    assert!(c.focused_widget.is_some_and(|id| c
        .previous_hits
        .iter()
        .any(|h| h.id == id && h.action == HitAction::TextEdit)));
    assert!(c.on_text_event("99").consumed);
    render(
        &mut c,
        &mut selected,
        &opts,
        start + Duration::from_millis(50),
    );
    assert_eq!(c.combo_boxes[&trigger.id].active, Some(opts[99].id));
    opts.clear();
    render(
        &mut c,
        &mut selected,
        &opts,
        start + Duration::from_millis(70),
    );
    assert!(c.popup.is_none());
    assert_eq!(selected, Some(30));
    assert_eq!(c.focused_widget, Some(trigger.id));
}

#[test]
fn clipped_anchor_has_no_invisible_trigger_proxy_and_resize_repositions_popup() {
    let mut c = setup();
    let mut selected = Some(0);
    let opts = options(10);
    let mut trigger = None;
    let mut show = |c: &mut Context| {
        Window::new("clipped")
            .default_position(vec2(580.0, 350.0))
            .show(c, |ui| {
                ScrollArea::vertical().max_height(15.0).show(ui, |ui| {
                    trigger = Some(
                        ui.add(
                            ComboBox::new(&mut selected, &opts)
                                .id_source("proxy")
                                .default_open(true),
                        ),
                    );
                });
            });
    };
    c.run(&mut show);
    let popup = c.popup.as_ref().unwrap();
    let hidden = vec2(popup.anchor.center().x, popup.anchor.max.y + 1.0);
    assert!(c
        .hit_test(hidden)
        .is_none_or(|h| h.action != HitAction::ComboBox));
    c.set_viewport(PhysicalSize::new(640, 460), 1.0);
    c.run(&mut show);
    let popup = c.popup.as_ref().unwrap();
    assert!(popup.rect.max.x <= 640.0 && popup.rect.max.y <= 460.0);
    assert!(popup.rect.min.y >= 0.0 && popup.rect.min.x >= 0.0);
}

#[test]
fn captions_fit_and_center_inside_label_trigger_and_option_rows() {
    for scale in [1.0, 1.25, 2.0] {
        let mut c = setup();
        c.set_viewport(
            PhysicalSize::new((700.0 * scale) as u32, (500.0 * scale) as u32),
            scale,
        );
        let mut opts = options(100);
        for o in &mut opts {
            o.label = format!("Region {} / Регион {}", o.value, o.value);
        }
        let mut selected = Some(80);
        let r = draw(&mut c, &mut selected, &opts, false, false);
        let check = |c: &Context, paint: Id, rect: crate::Rect| {
            let bounds = c.cache[&paint].bounds.unwrap();
            assert!(
                bounds.min.y >= rect.min.y,
                "text escapes top: {bounds:?} in {rect:?}"
            );
            assert!(
                bounds.max.y <= rect.max.y,
                "text escapes bottom: {bounds:?} in {rect:?}"
            );
            assert!(
                (bounds.center().y - rect.center().y).abs() <= 2.0,
                "text is not vertically centered: {bounds:?} in {rect:?}"
            );
        };
        check(&c, r.id.with("caption"), r.rect);
        let label =
            crate::Rect::from_min_size(r.rect.min - vec2(0.0, 24.0), vec2(r.rect.size().x, 20.0));
        check(&c, r.id.with("label"), label);
        click(&mut c, r.rect.center());
        draw(&mut c, &mut selected, &opts, false, false);
        for o in &opts {
            let id = r.id.with(("option", o.id));
            if let Some(hit) = c.previous_hits.iter().find(|h| h.id == id) {
                check(&c, id.with("caption"), hit.rect);
            }
        }
    }
}

#[test]
fn cached_filter_observes_in_place_labels_ids_disabled_values_and_reordering() {
    let mut c = setup();
    let mut selected = Some(0);
    let mut opts = options(100);
    let r = draw(&mut c, &mut selected, &opts, true, false);
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, true, false);
    c.on_text_event("ключ");
    draw(&mut c, &mut selected, &opts, true, false);
    assert!(c.combo_boxes[&r.id].active.is_none());
    opts[80].label = "КЛЮЧ / 80".into();
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.combo_boxes[&r.id].active, Some(opts[80].id));
    let active = opts[80].id;
    opts.reverse();
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.combo_boxes[&r.id].active, Some(active));
    let index = opts.iter().position(|o| o.id == active).unwrap();
    opts[index].enabled = false;
    draw(&mut c, &mut selected, &opts, true, false);
    assert!(c.combo_boxes[&r.id].active.is_none());
    opts[index].enabled = true;
    opts[index].id = Id::new("replacement");
    opts[index].value = 1000;
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.combo_boxes[&r.id].active, Some(opts[index].id));
    key(&mut c, KeyCode::Enter);
    assert!(draw(&mut c, &mut selected, &opts, true, false).changed());
    assert_eq!(selected, Some(1000));
}

#[test]
fn five_rows_fit_without_a_spurious_trailing_gap_scrollbar() {
    let mut c = setup();
    let mut selected = Some(0);
    let opts = options(5);
    let r = draw(&mut c, &mut selected, &opts, false, false);
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, false, false);
    let popup = c.popup.as_ref().unwrap().id;
    let scroll = c
        .scrolling
        .states
        .values()
        .find(|s| s.window == popup)
        .unwrap();
    assert_eq!(scroll.max_offset().y, 0.0);
    assert!(!c
        .previous_hits
        .iter()
        .any(|h| h.window == popup && matches!(h.action, HitAction::ScrollThumb { .. })));
}

#[test]
fn rows_fill_popup_width_with_and_without_an_overlay_scrollbar() {
    for count in [1, 100] {
        let mut c = setup();
        let mut selected = Some(0);
        let opts = options(count);
        let r = draw(&mut c, &mut selected, &opts, true, false);
        click(&mut c, r.rect.center());
        draw(&mut c, &mut selected, &opts, true, false);
        let row = row_hit(&c, r.id, opts[0].id);
        let popup = c.popup.as_ref().unwrap();
        let right = popup.rect.max.x - c.style().combo_box.popup_padding.right;
        assert_eq!(row.rect.max.x, right);
        assert_eq!(row.clip.max.x, right);
        if count == 1 {
            let edge = vec2(right - 1.0, row.rect.center().y);
            assert_eq!(c.hit_test(edge).unwrap().id, row.id);
        } else {
            let thumb = c
                .previous_hits
                .iter()
                .find(|h| h.window == popup.id && matches!(h.action, HitAction::ScrollThumb { .. }))
                .unwrap();
            assert_eq!(thumb.rect.max.x, right);
            assert_eq!(c.hit_test(thumb.rect.center()).unwrap().id, thumb.id);
        }
    }
}

#[test]
fn spacer_anchors_popup_at_final_trigger_and_keeps_viewport_and_scrollbar_routing() {
    let mut c = setup();
    let mut selected = Some(0);
    let opts = options(100);
    let draw = |c: &mut Context, selected: &mut Option<usize>| {
        let mut response = None;
        c.run(|c| {
            Window::new("aligned combo")
                .default_size(vec2(700.0, 450.0))
                .show(c, |ui| {
                    response = Some(ui.horizontal_aligned(crate::Align::Center, |ui| {
                        ui.label("Mode");
                        ui.spacer();
                        ui.add(ComboBox::new(selected, &opts).width(140.0))
                    }));
                });
        });
        response.unwrap()
    };
    let r = draw(&mut c, &mut selected);
    let trigger = c.visual_rect(r.id, r.rect);
    click(&mut c, trigger.center());
    let r = draw(&mut c, &mut selected);
    let popup = c.popup.as_ref().unwrap();
    assert_eq!(popup.anchor, c.visual_rect(r.id, r.rect));
    assert_eq!(popup.rect.intersect(c.viewport()), popup.rect);
    let row = row_hit(&c, r.id, opts[0].id);
    assert!(popup.rect.contains(row.rect.center()));
    let thumb = c
        .previous_hits
        .iter()
        .find(|h| h.window == popup.id && matches!(h.action, HitAction::ScrollThumb { .. }))
        .unwrap();
    assert_eq!(c.hit_test(thumb.rect.center()).unwrap().id, thumb.id);
    let p = row.rect.center();
    click(&mut c, p);
    draw(&mut c, &mut selected);
    assert!(c.popup.is_none());
}
