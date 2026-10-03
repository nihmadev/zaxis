use super::*;

#[test]
fn unicode_navigation_and_deletion_never_split_clusters() {
    let mut c = setup();
    let mut a = "Привет е\u{301}👨‍👩‍👧‍👦🇷🇺👍🏽".to_owned();
    let mut b = String::new();
    let r = draw(&mut c, &mut a, &mut b, true, false);
    click(&mut c, r[0].rect.center());
    key(&mut c, KeyCode::End, ModifiersState::empty());
    for expected in [
        "Привет е\u{301}👨‍👩‍👧‍👦🇷🇺",
        "Привет е\u{301}👨‍👩‍👧‍👦",
        "Привет е\u{301}",
        "Привет ",
    ] {
        key(&mut c, KeyCode::Backspace, ModifiersState::empty());
        assert!(draw(&mut c, &mut a, &mut b, true, false)[0].changed());
        assert_eq!(a, expected);
    }
    key(&mut c, KeyCode::Home, ModifiersState::empty());
    key(&mut c, KeyCode::ArrowRight, ModifiersState::SHIFT);
    c.on_text_event("Ж");
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "Жривет ");
    key(&mut c, KeyCode::Home, ModifiersState::empty());
    key(&mut c, KeyCode::Delete, ModifiersState::empty());
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "ривет ");
}

#[test]
fn words_shift_selection_and_read_only() {
    let mut c = setup();
    let mut a = "один два 👩‍💻".to_owned();
    let mut b = String::new();
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    key(&mut c, KeyCode::Home, ModifiersState::empty());
    key(&mut c, KeyCode::ArrowRight, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    let id = c.focused_widget.unwrap();
    assert_eq!(c.text_edits[&id].buffer.cursor, "один".len());
    key(
        &mut c,
        KeyCode::ArrowRight,
        ModifiersState::CONTROL | ModifiersState::SHIFT,
    );
    c.on_text_event("!");
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "один! 👩‍💻");
    key(&mut c, KeyCode::End, ModifiersState::empty());
    key(&mut c, KeyCode::Backspace, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "один! ");
    key(&mut c, KeyCode::KeyA, ModifiersState::CONTROL);
    c.on_text_event("replace");
    key(&mut c, KeyCode::Delete, ModifiersState::empty());
    assert!(!draw(&mut c, &mut a, &mut b, true, true)[0].changed());
    assert_eq!(a, "один! ");
    assert!(c.ime_cursor_area().is_none());
    key(&mut c, KeyCode::ArrowLeft, ModifiersState::empty());
    draw(&mut c, &mut a, &mut b, true, true);
    assert_eq!(c.text_edits[&id].buffer.cursor, 0);
}

#[test]
fn pointer_capture_selects_and_scrolls_using_font_metrics() {
    let mut c = setup();
    let mut a = "AV Привет 👩‍💻 world long long".to_owned();
    let mut b = String::new();
    let r = draw(&mut c, &mut a, &mut b, true, false);
    let start = r[0].rect.min + vec2(c.style.text_edit_padding.left, 10.0);
    click(&mut c, start);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(c.text_edits[&r[0].id].buffer.cursor, 0);
    let x = c
        .text_carets(&a, c.style.text_edit_font_size)
        .iter()
        .find(|(i, _)| *i == 2)
        .unwrap()
        .1;
    c.move_pointer(start);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(start + vec2(x, 0.0));
    c.primary_button(ElementState::Released);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(c.text_edits[&r[0].id].buffer.selection(), 0..2);
    c.on_text_event("Z");
    draw(&mut c, &mut a, &mut b, true, false);
    assert!(a.starts_with("Z Привет"));
    key(&mut c, KeyCode::End, ModifiersState::empty());
    draw(&mut c, &mut a, &mut b, true, false);
    assert!(c.text_edits[&r[0].id].scroll > 0.0);
    key(&mut c, KeyCode::Home, ModifiersState::empty());
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(c.text_edits[&r[0].id].scroll, 0.0);
}

#[test]
fn ime_preedit_commit_and_focus_are_independent_of_changed_and_enter() {
    let mut c = setup();
    let mut a = "hello".to_owned();
    let mut b = String::new();
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    key(&mut c, KeyCode::KeyA, ModifiersState::CONTROL);
    c.on_window_event(&WindowEvent::Ime(Ime::Preedit(
        "にほん".into(),
        Some((3, 6)),
    )));
    assert!(!draw(&mut c, &mut a, &mut b, true, false)[0].changed());
    assert_eq!(a, "hello");
    let id = c.focused_widget.unwrap();
    assert!(c.text_edits[&id].preedit.is_some());
    assert!(c.ime_cursor_area().is_some());
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    c.on_window_event(&WindowEvent::Ime(Ime::Commit("日本🇯🇵".into())));
    let r = draw(&mut c, &mut a, &mut b, true, false);
    assert!(r[0].changed());
    assert!(!r[0].submitted());
    assert_eq!(a, "日本🇯🇵");
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    c.on_text_event("second");
    let r = draw(&mut c, &mut a, &mut b, true, false);
    assert!(r[0].submitted());
    assert!(r[0].lost_focus());
    assert!(c.needs_repaint());
    assert!(!r[0].changed());
    assert!(r[1].changed());
    assert_eq!(b, "second");
    c.on_window_event(&WindowEvent::Ime(Ime::Preedit("draft".into(), None)));
    c.on_window_event(&WindowEvent::Focused(false));
    let r = draw(&mut c, &mut a, &mut b, true, false);
    assert!(r[1].lost_focus());
    assert_eq!(b, "second");
    assert!(c.ime_cursor_area().is_none());
}

#[test]
fn events_survive_focus_switches_before_redraw_and_disabled_fields_dont_edit() {
    let mut c = setup();
    let mut a = String::new();
    let mut b = String::new();
    let r = draw(&mut c, &mut a, &mut b, true, false);
    click(&mut c, r[0].rect.center());
    c.on_text_event("А");
    click(&mut c, r[1].rect.center());
    c.on_text_event("Б");
    let r = draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "А");
    assert_eq!(b, "Б");
    assert!(r[0].lost_focus());
    click(&mut c, r[0].rect.center());
    c.on_text_event("ignored");
    assert!(!draw(&mut c, &mut a, &mut b, false, false)[0].changed());
    assert_eq!(a, "А");
    assert_ne!(c.focused_widget, Some(r[0].id));
}

#[test]
fn cursor_deadlines_and_external_replacement_do_not_spin_or_panic() {
    let mut c = setup();
    let mut a = "long text long text long text".to_owned();
    let mut b = String::new();
    draw(&mut c, &mut a, &mut b, true, false);
    assert!(c.next_repaint().is_none());
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    draw(&mut c, &mut a, &mut b, true, false);
    assert!(!c.dirty);
    assert!(c.next_repaint().unwrap() > Instant::now());
    let deadline = c.next_repaint().unwrap();
    draw(&mut c, &mut a, &mut b, true, false);
    assert!(!c.dirty);
    assert!(
        deadline
            .max(c.next_repaint().unwrap())
            .duration_since(deadline.min(c.next_repaint().unwrap()))
            < std::time::Duration::from_millis(1)
    );
    a = "е\u{301}".into();
    key(&mut c, KeyCode::Backspace, ModifiersState::empty());
    draw(&mut c, &mut a, &mut b, true, false);
    assert!(a.is_empty());
    c.on_window_event(&WindowEvent::Focused(false));
    draw(&mut c, &mut a, &mut b, true, false);
    c.next_repaint = None;
    draw(&mut c, &mut a, &mut b, true, false);
    assert!(c.next_repaint().is_none());
    assert!(!c.needs_repaint());
}

#[test]
fn shift_click_cache_reuse_stable_ids_and_removed_state() {
    let mut c = setup();
    let mut a = "abcdef".to_owned();
    let mut b = String::new();
    let r = draw(&mut c, &mut a, &mut b, true, false);
    let start = r[0].rect.min + vec2(c.style.text_edit_padding.left, 10.0);
    click(&mut c, start);
    let x = c
        .text_carets(&a, c.style.text_edit_font_size)
        .iter()
        .find(|(i, _)| *i == 3)
        .unwrap()
        .1;
    c.input.modifiers = ModifiersState::SHIFT;
    click(&mut c, start + vec2(x, 0.0));
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(c.text_edits[&r[0].id].buffer.selection(), 0..3);
    let stats = c.cache_stats();
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(
        c.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    c.run(|c| {
        Window::new("Edit test").show(c, |ui| {
            ui.label("Inserted label");
            let response = ui.add(TextEdit::new(&mut a).id_source("a").width(120.0));
            assert_eq!(response.id, r[0].id);
            assert!(response.has_focus);
        });
    });
    assert_eq!(c.text_edits[&r[0].id].buffer.selection(), 0..3);
    assert!(!c.text_edits.contains_key(&r[1].id));
    c.run(|_| {});
    assert!(c.text_edits.is_empty());
    assert!(c.focused_widget.is_none());
    assert!(c.ime_cursor_area().is_none());
}

#[test]
#[ignore = "requires a desktop system clipboard"]
fn system_clipboard_copy_cut_paste_and_read_only() {
    let mut c = setup();
    let mut a = "Кириллица 👩‍💻".to_owned();
    let mut b = String::new();
    let original = c.clipboard_text().ok();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        draw(&mut c, &mut a, &mut b, true, false);
        key(&mut c, KeyCode::Tab, ModifiersState::empty());
        key(&mut c, KeyCode::KeyA, ModifiersState::CONTROL);
        key(&mut c, KeyCode::KeyC, ModifiersState::CONTROL);
        draw(&mut c, &mut a, &mut b, true, false);
        let copied = c.clipboard_text().unwrap();
        assert_eq!(copied, a);
        key(&mut c, KeyCode::KeyX, ModifiersState::CONTROL);
        assert!(draw(&mut c, &mut a, &mut b, true, false)[0].changed());
        assert!(a.is_empty());
        key(&mut c, KeyCode::KeyV, ModifiersState::CONTROL);
        draw(&mut c, &mut a, &mut b, true, false);
        assert_eq!(a, copied);
        key(&mut c, KeyCode::KeyA, ModifiersState::CONTROL);
        key(&mut c, KeyCode::KeyX, ModifiersState::CONTROL);
        key(&mut c, KeyCode::KeyV, ModifiersState::CONTROL);
        assert!(!draw(&mut c, &mut a, &mut b, true, true)[0].changed());
        assert_eq!(a, copied);
        c.copy_text("a\r\nb\tc".into()).unwrap();
        key(&mut c, KeyCode::KeyV, ModifiersState::CONTROL);
        draw(&mut c, &mut a, &mut b, true, false);
        assert_eq!(a, "abc");
    }));
    if let Some(original) = original {
        c.copy_text(original).unwrap();
    }
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

#[test]
fn undo_redo_groups_typing_and_restores_selection() {
    let mut c = setup();
    let mut a = "one".to_owned();
    let mut b = String::new();
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    for (code, text) in [(KeyCode::KeyA, "a"), (KeyCode::KeyB, "б")] {
        key(&mut c, code, ModifiersState::empty());
        c.on_text_event(text);
        draw(&mut c, &mut a, &mut b, true, false);
    }
    assert_eq!(a, "oneaб");
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    assert!(draw(&mut c, &mut a, &mut b, true, false)[0].changed());
    assert_eq!(a, "one");
    key(
        &mut c,
        KeyCode::KeyZ,
        ModifiersState::CONTROL | ModifiersState::SHIFT,
    );
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "oneaб");
    key(&mut c, KeyCode::KeyA, ModifiersState::CONTROL);
    c.on_text_event("👩‍💻");
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    let id = c.focused_widget.unwrap();
    assert_eq!(a, "oneaб");
    assert_eq!(c.text_edits[&id].buffer.selection(), 0..a.len());
    key(&mut c, KeyCode::KeyY, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "👩‍💻");
    assert_eq!(c.text_edits[&id].buffer.cursor, a.len());
    key(&mut c, KeyCode::KeyZ, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    c.on_text_event("new");
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::KeyY, ModifiersState::CONTROL);
    assert!(!draw(&mut c, &mut a, &mut b, true, false)[0].changed());
    assert_eq!(a, "new");
}
