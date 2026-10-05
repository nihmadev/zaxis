//! One change per choice, closing and focus, the highlight after filtering or resizing,
//! and options that change while the list is open.
use super::*;

/// Passes after a choice: the choice was reported in the pass that applied it, never again.
fn quiet(c: &mut Context, selected: &mut Option<usize>, opts: &[ComboBoxOption<usize>]) {
    for _ in 0..3 {
        assert!(!draw(c, selected, opts, false, false).changed());
    }
}

fn visible_row(c: &Context, combo: Id, option: Id) -> bool {
    c.probe()
        .previous_hits
        .iter()
        .find(|h| h.id == combo.with(("option", option)))
        .is_some_and(|h| !h.rect.intersect(h.clip).is_empty())
}

#[test]
fn pointer_and_keyboard_choices_change_once_and_rechoosing_changes_nothing() {
    let mut c = setup();
    let mut selected = Some(0);
    let opts = options(10);
    let r = draw(&mut c, &mut selected, &opts, false, false);
    // Pointer.
    click(&mut c, r.rect.center());
    assert!(!draw(&mut c, &mut selected, &opts, false, false).changed());
    let hit = row_hit(&c, r.id, opts[3].id);
    click(&mut c, hit.rect.intersect(hit.clip).center());
    assert!(draw(&mut c, &mut selected, &opts, false, false).changed());
    assert_eq!(selected, Some(3));
    assert!(c.probe().popup.is_none());
    assert_eq!(c.probe().focused_widget, Some(r.id));
    quiet(&mut c, &mut selected, &opts);
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, false, false);
    let hit = row_hit(&c, r.id, opts[3].id);
    click(&mut c, hit.rect.intersect(hit.clip).center());
    assert!(!draw(&mut c, &mut selected, &opts, false, false).changed());
    assert_eq!(selected, Some(3));
    assert!(c.probe().popup.is_none());
    quiet(&mut c, &mut selected, &opts);
    // Keyboard: the focused trigger opens on the selection, Enter chooses.
    key(&mut c, KeyCode::Enter);
    assert!(!draw(&mut c, &mut selected, &opts, false, false).changed());
    assert_eq!(c.probe().combo_boxes[&r.id].active, Some(opts[3].id));
    key(&mut c, KeyCode::ArrowDown);
    key(&mut c, KeyCode::Enter);
    assert!(draw(&mut c, &mut selected, &opts, false, false).changed());
    assert_eq!(selected, Some(4));
    assert!(c.probe().popup.is_none());
    assert_eq!(c.probe().focused_widget, Some(r.id));
    quiet(&mut c, &mut selected, &opts);
    key(&mut c, KeyCode::Space);
    draw(&mut c, &mut selected, &opts, false, false);
    assert!(c.probe().popup.is_some());
    key(&mut c, KeyCode::Space);
    assert!(!draw(&mut c, &mut selected, &opts, false, false).changed());
    assert_eq!(selected, Some(4));
    assert!(c.probe().popup.is_none());
    quiet(&mut c, &mut selected, &opts);
}

#[cfg(feature = "accesskit")]
#[test]
fn requests_of_assistive_technology_change_once_and_close_the_popup() {
    use zaxis::accessibility::testing::AccessTree;
    use zaxis::accesskit::{Action, ActionData, Role};
    let mut c = setup();
    let mut tree = AccessTree::attach(&mut c);
    let mut selected = Some(0);
    let opts = options(10);
    let r = draw(&mut c, &mut selected, &opts, false, false);
    tree.sync(&mut c);
    let combo = tree.expect(Role::ComboBox, "Label");
    let set = |tree: &AccessTree, c: &mut Context, text: &str| {
        assert!(tree.act(
            c,
            combo,
            Action::SetValue,
            Some(ActionData::Value(text.into()))
        ));
    };
    // While open: one change, the popup closes and focus is back on the trigger.
    assert!(tree.act(&mut c, combo, Action::Expand, None));
    assert!(!draw(&mut c, &mut selected, &opts, false, false).changed());
    tree.sync(&mut c);
    assert!(c.probe().popup.is_some());
    set(&tree, &mut c, "Option 6");
    assert!(draw(&mut c, &mut selected, &opts, false, false).changed());
    tree.sync(&mut c);
    assert_eq!(selected, Some(6));
    assert!(c.probe().popup.is_none());
    assert_eq!(c.probe().focused_widget, Some(r.id));
    quiet(&mut c, &mut selected, &opts);
    tree.sync(&mut c);
    // The selected value again: nothing changes.
    set(&tree, &mut c, "Option 6");
    assert!(!draw(&mut c, &mut selected, &opts, false, false).changed());
    tree.sync(&mut c);
    // While closed: one change and the popup stays closed.
    set(&tree, &mut c, "Option 2");
    assert!(draw(&mut c, &mut selected, &opts, false, false).changed());
    tree.sync(&mut c);
    assert_eq!(selected, Some(2));
    assert!(c.probe().popup.is_none());
    quiet(&mut c, &mut selected, &opts);
}

#[test]
fn a_choice_closes_the_popup_and_returns_focus_once() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.motion.reduced_motion = false;
    c.set_style(style);
    let mut selected = Some(0);
    let opts = options(10);
    let start = Instant::now();
    let mut ms = 0;
    let mut pass = |c: &mut Context, selected: &mut Option<usize>| {
        ms += 16;
        let mut response = None;
        c.run_at(start + Duration::from_millis(ms), |c| {
            Window::new("test").show(c, |ui| {
                response = Some(ui.add(ComboBox::new(selected, &opts).id_source("combo")));
                ui.button("After");
            });
        });
        response.unwrap()
    };
    let r = pass(&mut c, &mut selected);
    key(&mut c, KeyCode::Tab);
    key(&mut c, KeyCode::Enter);
    for _ in 0..20 {
        pass(&mut c, &mut selected);
    }
    assert!(c.probe().popup.is_some());
    key(&mut c, KeyCode::ArrowDown);
    key(&mut c, KeyCode::Enter);
    assert!(pass(&mut c, &mut selected).changed());
    assert!(c.probe().popup.is_none());
    assert_eq!(c.probe().focused_widget, Some(r.id));
    // Focus moved away while the popup animates closed stays where it went.
    key(&mut c, KeyCode::Tab);
    let elsewhere = c.probe().focused_widget;
    assert_ne!(elsewhere, Some(r.id));
    for _ in 0..20 {
        assert!(!pass(&mut c, &mut selected).changed());
        assert_eq!(c.probe().focused_widget, elsewhere);
    }
    assert!(c.next_repaint().is_none());
    // Closed by its choice, it opens again as usual and keeps the chosen value.
    c.set_focus(Some(r.id));
    key(&mut c, KeyCode::Enter);
    pass(&mut c, &mut selected);
    assert!(c.probe().popup.is_some());
    assert!(c.probe().combo_boxes[&r.id].open);
    assert_eq!(selected, Some(1));
}

#[test]
fn the_highlight_moves_to_the_first_enabled_match_when_the_filter_hides_it() {
    let mut c = setup();
    let mut selected = Some(1);
    let mut opts = options(30);
    opts[2].enabled = false;
    let r = draw(&mut c, &mut selected, &opts, true, false);
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.probe().combo_boxes[&r.id].active, Some(opts[1].id));
    // "2" lists 2 (disabled), 12, 20..29: the highlight goes to 12.
    assert!(c.on_text_event("2").consumed);
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.probe().combo_boxes[&r.id].active, Some(opts[12].id));
    key(&mut c, KeyCode::ArrowDown);
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.probe().combo_boxes[&r.id].active, Some(opts[20].id));
    assert!(c.on_text_event("1").consumed);
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.probe().combo_boxes[&r.id].active, Some(opts[21].id));
    assert!(visible_row(&c, r.id, opts[21].id));
    // Clearing the filter keeps the highlight and scrolls it into view.
    key(&mut c, KeyCode::Backspace);
    key(&mut c, KeyCode::Backspace);
    draw(&mut c, &mut selected, &opts, true, false);
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(c.probe().combo_boxes[&r.id].query, "");
    assert_eq!(c.probe().combo_boxes[&r.id].active, Some(opts[21].id));
    assert!(visible_row(&c, r.id, opts[21].id));
    key(&mut c, KeyCode::Enter);
    assert!(draw(&mut c, &mut selected, &opts, true, false).changed());
    assert_eq!(selected, Some(21));
}

#[test]
fn resizing_the_open_list_scrolls_the_highlight_back_into_view() {
    let mut c = setup();
    let mut selected = Some(80);
    let opts = options(100);
    let draw = |c: &mut Context, selected: &mut Option<usize>, width: f32| {
        let mut response = None;
        c.run(|c| {
            Window::new("test")
                .default_size(vec2(500.0, 350.0))
                .show(c, |ui| {
                    let combo = ComboBox::new(selected, &opts).id_source("combo");
                    response = Some(ui.add(combo.width(width)));
                });
        });
        response.unwrap()
    };
    let r = draw(&mut c, &mut selected, 240.0);
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, 240.0);
    assert!(visible_row(&c, r.id, opts[80].id));
    let popup = c.probe().popup.as_ref().unwrap().rect;
    c.move_pointer(popup.center());
    assert!(c.scroll_wheel(vec2(0.0, 4000.0)));
    draw(&mut c, &mut selected, 240.0);
    draw(&mut c, &mut selected, 240.0);
    assert!(!visible_row(&c, r.id, opts[80].id));
    draw(&mut c, &mut selected, 300.0);
    draw(&mut c, &mut selected, 300.0);
    assert!(visible_row(&c, r.id, opts[80].id));
    assert_eq!(c.probe().combo_boxes[&r.id].active, Some(opts[80].id));
}

#[test]
fn options_that_change_while_open_update_the_open_list() {
    let mut c = setup();
    let mut selected = Some(1);
    let mut opts = options(3);
    let r = draw(&mut c, &mut selected, &opts, true, false);
    click(&mut c, r.rect.center());
    draw(&mut c, &mut selected, &opts, true, false);
    let popup = c.probe().popup.as_ref().unwrap();
    let (popup_id, short) = (popup.id, popup.rect.size().y);
    let rows = |c: &Context| {
        c.probe()
            .previous_hits
            .iter()
            .filter(|h| h.window == popup_id && h.action == HitAction::Activate)
            .count()
    };
    assert_eq!(rows(&c), 3);
    // A new option is listed at once and the list grows.
    opts.push(ComboBoxOption::new(3, 3, "Zebra"));
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(rows(&c), 4);
    assert!(c.probe().popup.as_ref().unwrap().rect.size().y > short);
    assert!(visible_row(&c, r.id, opts[3].id));
    // The filter sees labels as they are now.
    assert!(c.on_text_event("zeb").consumed);
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(rows(&c), 1);
    assert_eq!(c.probe().combo_boxes[&r.id].active, Some(opts[3].id));
    opts[3].label = "Yak".into();
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(rows(&c), 0);
    assert_eq!(c.probe().combo_boxes[&r.id].active, None);
    opts[0].label = "Zebu".into();
    draw(&mut c, &mut selected, &opts, true, false);
    assert_eq!(rows(&c), 1);
    assert_eq!(c.probe().combo_boxes[&r.id].active, Some(opts[0].id));
    assert!(c.probe().popup.is_some());
    key(&mut c, KeyCode::Enter);
    assert!(draw(&mut c, &mut selected, &opts, true, false).changed());
    assert_eq!(selected, Some(0));
}
