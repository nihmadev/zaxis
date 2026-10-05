//! The system clipboard is shared by every process and test, so these checks run as one
//! ignored test, like the clipboard check of `TextEdit`:
//! `cargo test --test ui text::clipboard -- --ignored`.
use super::support::*;
use winit::keyboard::ModifiersState;
use zaxis::{RichText, SelectableLabel, TextEdit};

fn click_row(c: &mut Context, index: usize) {
    let popup = c.probe().popup.as_ref().expect("menu open").id;
    let rows: Vec<_> = c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| h.window == popup && h.action == HitAction::Activate)
        .map(|h| h.rect)
        .collect();
    click(c, rows[index].center());
}

fn copy_shortcuts(c: &mut Context) {
    let text = "tab\tsep  and\nnew line é👍🏽";
    let show = |c: &mut Context| {
        frame(c, |ui| {
            ui.selection_scope(|ui| (ui.selectable_label(text), ui.selectable_label("tail")))
        })
    };
    let (a, _) = show(c);
    c.copy_text("previous".into()).unwrap();
    click(c, a.rect.center());
    show(c);
    ctrl(c, KeyCode::KeyC);
    assert_eq!(c.clipboard_text().unwrap(), "previous", "nothing selected");
    ctrl(c, KeyCode::KeyA);
    show(c);
    ctrl(c, KeyCode::KeyC);
    assert_eq!(c.clipboard_text().unwrap(), format!("{text}\ntail"));
    c.copy_text("other".into()).unwrap();
    chord(c, ModifiersState::CONTROL, KeyCode::Insert);
    assert_eq!(c.clipboard_text().unwrap(), format!("{text}\ntail"));
}

fn menus(c: &mut Context) {
    let show = |c: &mut Context| {
        frame(c, |ui| {
            let link = ui.hyperlink_to("Docs", "https://example.com");
            let plain = ui.selectable_label("plain text");
            (link, plain)
        })
    };
    let (link, plain) = show(c);
    for (row, expected) in [(0, "https://example.com"), (1, "Docs")] {
        right_click(c, link.rect.center());
        show(c);
        show(c);
        click_row(c, row);
        show(c);
        assert_eq!(c.clipboard_text().unwrap(), expected);
    }
    click(c, plain.rect.center());
    ctrl(c, KeyCode::KeyA);
    show(c);
    right_click(c, plain.rect.center());
    show(c);
    show(c);
    click_row(c, 0);
    show(c);
    assert_eq!(c.clipboard_text().unwrap(), "plain text");
}

fn paste_into_a_field(c: &mut Context) {
    let mut field = String::new();
    let show = |c: &mut Context, field: &mut String| {
        frame(c, |ui| {
            let l = ui.selectable_label("pasted words");
            let f = ui.add(TextEdit::new(field));
            (l, f)
        })
    };
    let (l, f) = show(c, &mut field);
    click(c, l.rect.center());
    ctrl(c, KeyCode::KeyA);
    show(c, &mut field);
    ctrl(c, KeyCode::KeyC);
    click(c, f.rect.center());
    show(c, &mut field);
    ctrl(c, KeyCode::KeyV);
    show(c, &mut field);
    assert_eq!(field, "pasted words");
}

fn addresses_only_on_request(c: &mut Context) {
    let rich = || {
        RichText::new()
            .text("see ")
            .link("docs", "https://example.com")
            .text(" now")
    };
    for (urls, expected) in [
        (false, "see docs now"),
        (true, "see docs (https://example.com) now"),
    ] {
        let show = |c: &mut Context| {
            frame(c, |ui| {
                ui.add(SelectableLabel::rich(rich()).copy_url_in_selection(urls))
            })
        };
        let r = show(c);
        click(c, r.rect.min + Vec2::new(2.0, 8.0));
        ctrl(c, KeyCode::KeyA);
        show(c);
        ctrl(c, KeyCode::KeyC);
        assert_eq!(c.clipboard_text().unwrap(), expected);
    }
}

fn copy_button(c: &mut Context) {
    let show = |c: &mut Context| frame(c, |ui| ui.copyable_label("copy this"));
    let r = show(c);
    c.copy_text("old".into()).unwrap();
    click(c, Vec2::new(r.rect.max.x + 14.0, r.rect.center().y));
    show(c);
    assert_eq!(c.clipboard_text().unwrap(), "copy this");
}

#[test]
#[ignore = "requires a desktop system clipboard"]
fn selection_copy_reaches_the_system_clipboard() {
    let mut c = setup();
    let original = c.clipboard_text().ok();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        copy_shortcuts(&mut c);
        menus(&mut c);
        paste_into_a_field(&mut c);
        addresses_only_on_request(&mut c);
        copy_button(&mut c);
    }));
    if let Some(original) = original {
        c.copy_text(original).unwrap();
    }
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
