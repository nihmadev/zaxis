use super::*;

#[test]
fn history_is_per_field_and_external_changes_reset_it() {
    let mut c = setup();
    let mut a = "a".to_owned();
    let mut b = "b".to_owned();
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    c.on_text_event("1");
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    c.on_text_event("2");
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!((&*a, &*b), ("a1", "b"));
    key(&mut c, KeyCode::Tab, ModifiersState::SHIFT);
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    assert!(!draw(&mut c, &mut a, &mut b, true, true)[0].changed());
    assert_eq!(a, "a1");
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "a");
    a = "external".into();
    key(&mut c, KeyCode::KeyY, ModifiersState::CONTROL);
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    assert!(!draw(&mut c, &mut a, &mut b, true, false)[0].changed());
    assert_eq!(a, "external");
}

#[test]
fn double_click_word_drag_and_triple_click_line() {
    let mut c = setup();
    let mut a = "one два е\u{301}!".to_owned();
    let mut b = String::new();
    let r = draw(&mut c, &mut a, &mut b, true, false);
    let start = r[0].rect.min + vec2(c.probe().style.text_edit_padding.left, 10.0);
    let points = c.text_carets(
        &a,
        c.probe().style.text_edit_font_size,
        zaxis::FontWeight::REGULAR,
    );
    let p = |byte| {
        start
            + vec2(
                points.iter().find(|(i, _)| *i == byte).unwrap().1 + 1.0,
                0.0,
            )
    };
    click(&mut c, p(4));
    c.move_pointer(p(4));
    c.primary_button(ElementState::Pressed);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(&a[c.probe().text_edits[&r[0].id].buffer.selection()], "два");
    c.move_pointer(p(0));
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(
        &a[c.probe().text_edits[&r[0].id].buffer.selection()],
        "one два"
    );
    c.move_pointer(p(4));
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(&a[c.probe().text_edits[&r[0].id].buffer.selection()], "два");
    c.move_pointer(p(11));
    c.primary_button(ElementState::Released);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(
        &a[c.probe().text_edits[&r[0].id].buffer.selection()],
        "два е\u{301}"
    );
    for _ in 0..3 {
        click(&mut c, p(0));
    }
    assert!(!draw(&mut c, &mut a, &mut b, true, true)[0].changed());
    assert_eq!(
        c.probe().text_edits[&r[0].id].buffer.selection(),
        0..a.len()
    );
}

#[test]
fn ime_commit_is_one_undo_step_and_deletions_are_undoable() {
    let mut c = setup();
    let mut a = "е\u{301}👩‍💻".to_owned();
    let mut b = String::new();
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    key(&mut c, KeyCode::Backspace, ModifiersState::empty());
    c.on_window_event(&WindowEvent::Ime(Ime::Preedit("に".into(), Some((3, 3)))));
    c.on_window_event(&WindowEvent::Ime(Ime::Commit("日本".into())));
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "е\u{301}日本");
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "е\u{301}");
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "е\u{301}👩‍💻");
    c.on_window_event(&WindowEvent::Ime(Ime::Commit("日".into())));
    c.on_text_event("a");
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "е\u{301}👩‍💻日");
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "е\u{301}👩‍💻");
}
