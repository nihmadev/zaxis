use super::*;
use crate::{vec2, ComboBox, ComboBoxOption, Popup, Response, ScrollArea, Window};
use std::time::{Duration, Instant};
use winit::{dpi::PhysicalSize, event::ElementState};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(700, 500), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
fn options(count: usize) -> Vec<ComboBoxOption<usize>> {
    (0..count)
        .map(|i| ComboBoxOption::new(i, i, format!("Option {i}")))
        .collect()
}
fn draw(
    c: &mut Context,
    selected: &mut Option<usize>,
    options: &[ComboBoxOption<usize>],
    filter: bool,
    disabled: bool,
) -> Response {
    let mut result = None;
    c.run(|c| {
        Window::new("test")
            .default_size(vec2(500.0, 350.0))
            .show(c, |ui| {
                result = Some(
                    ui.add(
                        ComboBox::new(selected, options)
                            .id_source("combo")
                            .label("Label")
                            .filterable(filter)
                            .disabled(disabled),
                    ),
                );
                ui.button("Under popup");
            });
    });
    result.unwrap()
}
fn key(c: &mut Context, code: KeyCode) {
    assert!(c.on_key_event(code, ElementState::Pressed, false).consumed);
    c.on_key_event(code, ElementState::Released, false);
}
fn click(c: &mut Context, point: Vec2) {
    c.move_pointer(point);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}
fn row_hit(c: &Context, combo: Id, option: Id) -> HitRegion {
    *c.previous_hits
        .iter()
        .find(|h| h.id == combo.with(("option", option)))
        .unwrap()
}

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
