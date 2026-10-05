use super::support::*;
use winit::window::CursorIcon;
use zaxis::{ScrollArea, SelectableLabel, SelectionScope, TextEdit};

fn label(c: &mut Context, text: &str) -> Response {
    frame(c, |ui| ui.selectable_label(text))
}

/// Screen x of the caret at `byte` of `text` in a label at `rect`.
fn x_at(c: &mut Context, rect: Rect, text: &str, byte: usize) -> f32 {
    let weight = c.style().typography.weights.body;
    let size = c.style().font_size;
    let carets = c.text_carets(text, size, weight);
    rect.min.x
        + carets
            .into_iter()
            .find(|(b, _)| *b == byte)
            .unwrap_or_else(|| panic!("no caret at {byte}"))
            .1
}

/// A press, two moves and one pass after each, the way a window delivers them.
fn drag(c: &mut Context, from: Vec2, to: Vec2, show: &mut dyn FnMut(&mut Context)) {
    down(c, from);
    show(c);
    c.move_pointer((from + to) * 0.5);
    show(c);
    c.move_pointer(to);
    show(c);
}

fn select(c: &mut Context, text: &str, a: usize, b: usize) -> Response {
    let r = label(c, text);
    let y = r.rect.center().y;
    let (x0, x1) = (x_at(c, r.rect, text, a), x_at(c, r.rect, text, b));
    drag(c, Vec2::new(x0, y), Vec2::new(x1, y), &mut |c| {
        label(c, text);
    });
    up(c);
    label(c, text)
}

#[test]
fn dragging_selects_grapheme_clusters_and_the_copy_is_exact() {
    let mut c = setup();
    let text = "héllo wörld";
    select(&mut c, text, 1, 8);
    assert_eq!(c.selected_text().as_deref(), Some("éllo w"));
}

#[test]
fn emoji_and_combining_marks_are_never_split() {
    let mut c = setup();
    let text = "a👍🏽e\u{301}b";
    let r = label(&mut c, text);
    let y = r.rect.center().y;
    let (x_start, x_end) = (
        x_at(&mut c, r.rect, text, 0),
        x_at(&mut c, r.rect, text, text.len()),
    );
    // Every release point lands on a boundary, so the copy is always whole clusters.
    let steps = 40;
    for i in 1..=steps {
        let x = x_start + (x_end - x_start) * i as f32 / steps as f32;
        c.clear_selection();
        label(&mut c, text);
        drag(&mut c, Vec2::new(x_start, y), Vec2::new(x, y), &mut |c| {
            label(c, text);
        });
        up(&mut c);
        label(&mut c, text);
        let copied = c.selected_text().unwrap_or_default();
        assert!(
            ["", "a", "a👍🏽", "a👍🏽e\u{301}", text].contains(&copied.as_str()),
            "step {i}: {copied:?}"
        );
    }
}

#[test]
fn double_click_selects_a_word_and_triple_click_a_paragraph() {
    let mut c = setup();
    let text = "alpha beta\nsecond line";
    let r = label(&mut c, text);
    let y = r.rect.min.y + 8.0;
    let beta = Vec2::new(x_at(&mut c, r.rect, "alpha beta", 8), y);
    click(&mut c, beta);
    label(&mut c, text);
    click(&mut c, beta);
    label(&mut c, text);
    assert_eq!(c.selected_text().as_deref(), Some("beta"));
    click(&mut c, beta);
    label(&mut c, text);
    assert_eq!(c.selected_text().as_deref(), Some("alpha beta"));
    // A fourth click starts over with a single click.
    click(&mut c, beta);
    label(&mut c, text);
    assert_eq!(c.selected_text().unwrap_or_default(), "");
}

#[test]
fn shift_click_extends_from_the_anchor() {
    let mut c = setup();
    let text = "one two three four";
    let r = select(&mut c, text, 0, 3);
    let y = r.rect.center().y;
    c.set_modifiers(winit::keyboard::ModifiersState::SHIFT);
    let at = Vec2::new(x_at(&mut c, r.rect, text, 13), y);
    click(&mut c, at);
    c.set_modifiers(Default::default());
    label(&mut c, text);
    assert_eq!(c.selected_text().as_deref(), Some("one two three"));
}

#[test]
fn ctrl_a_selects_everything_in_the_focused_label_only() {
    let mut c = setup();
    let text = "select me";
    let show = |c: &mut Context| {
        frame(c, |ui| {
            (ui.selectable_label(text), ui.selectable_label("other"))
        })
    };
    let (a, _) = show(&mut c);
    ctrl(&mut c, KeyCode::KeyA);
    show(&mut c);
    assert!(
        c.selected_text().is_none(),
        "without focus nothing is selected"
    );
    click(&mut c, a.rect.center());
    show(&mut c);
    ctrl(&mut c, KeyCode::KeyA);
    show(&mut c);
    assert_eq!(c.selected_text().as_deref(), Some(text));
}

#[test]
fn a_copy_keeps_explicit_breaks_tabs_and_spaces_but_not_wrapping() {
    let mut c = setup();
    let text = "x\ty  z\nsecond paragraph that is long enough to wrap onto more lines";
    let show =
        |c: &mut Context| frame(c, |ui| ui.with_width(120.0, |ui| ui.selectable_label(text)));
    let r = show(&mut c);
    click(&mut c, r.rect.center());
    show(&mut c);
    ctrl(&mut c, KeyCode::KeyA);
    show(&mut c);
    assert_eq!(c.selected_text().as_deref(), Some(text));
    assert!(r.rect.size().y > 3.0 * 17.0, "it did wrap");
}

#[test]
fn monospace_text_copies_without_distortion() {
    let mut c = setup();
    let text = "fn  main() {\n\tlet x =   1;\n}";
    let show = |c: &mut Context| frame(c, |ui| ui.add(SelectableLabel::new(text).monospace()));
    let r = show(&mut c);
    click(&mut c, r.rect.center());
    show(&mut c);
    ctrl(&mut c, KeyCode::KeyA);
    show(&mut c);
    assert_eq!(c.selected_text().as_deref(), Some(text));
}

#[test]
fn an_ellipsis_label_selects_and_copies_the_whole_text() {
    let mut c = setup();
    let text = "a very long file name that cannot possibly fit into the narrow column.txt";
    let show = |c: &mut Context| {
        frame(c, |ui| {
            ui.with_width(100.0, |ui| {
                ui.add(SelectableLabel::new(text).truncate(true))
            })
        })
    };
    let r = show(&mut c);
    assert!(
        r.rect.size().x <= 100.5 && r.rect.size().y < 24.0,
        "one line: {:?}",
        r.rect
    );
    click(&mut c, r.rect.center());
    show(&mut c);
    ctrl(&mut c, KeyCode::KeyA);
    show(&mut c);
    assert_eq!(c.selected_text().as_deref(), Some(text));
}

#[test]
fn changing_the_text_under_a_selection_never_panics_and_clamps() {
    let mut c = setup();
    let long = "ünïcödé text with ✨ emoji 👍🏽 inside";
    select(&mut c, long, 0, long.len());
    assert_eq!(c.selected_text().as_deref(), Some(long));
    for text in ["short ✨", "é", "", "x", long] {
        label(&mut c, text);
        let copied = c.selected_text().unwrap_or_default();
        assert!(
            text.contains(copied.as_str()) || copied.is_empty(),
            "{copied:?} of {text:?}"
        );
    }
}

#[test]
fn a_press_in_another_label_clears_the_previous_selection() {
    let mut c = setup();
    let two = |c: &mut Context| {
        frame(c, |ui| {
            (
                ui.selectable_label("first label"),
                ui.selectable_label("second label"),
            )
        })
    };
    let (a, b) = two(&mut c);
    drag(
        &mut c,
        a.rect.min + Vec2::new(2.0, 6.0),
        a.rect.max - Vec2::new(2.0, 6.0),
        &mut |c| {
            two(c);
        },
    );
    up(&mut c);
    two(&mut c);
    assert!(c.selected_text().is_some_and(|t| t.starts_with("first")));
    click(&mut c, b.rect.center());
    two(&mut c);
    assert_eq!(
        c.selected_text().unwrap_or_default(),
        "",
        "one selection per window"
    );
    // Clicking empty space clears too.
    drag(
        &mut c,
        a.rect.min + Vec2::new(2.0, 6.0),
        a.rect.max - Vec2::new(2.0, 6.0),
        &mut |c| {
            two(c);
        },
    );
    up(&mut c);
    two(&mut c);
    click(&mut c, Vec2::new(600.0, 440.0));
    two(&mut c);
    assert!(c.selected_text().is_none());
}

#[test]
fn it_is_not_editable_and_leaves_typing_alone() {
    let mut c = setup();
    let r = label(&mut c, "static");
    click(&mut c, r.rect.center());
    for code in [
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::KeyX,
        KeyCode::Enter,
    ] {
        assert!(
            !c.on_key_event(code, ElementState::Pressed, false).consumed,
            "{code:?}"
        );
        c.on_key_event(code, ElementState::Released, false);
    }
    assert!(!c.on_text_event("x").consumed);
    assert_eq!(c.cursor_icon(), CursorIcon::Text);
}

#[test]
fn shortcuts_stay_with_a_text_edit_that_has_focus() {
    let mut c = setup();
    let mut field = String::from("typed text");
    let show = |c: &mut Context, field: &mut String| {
        frame(c, |ui| {
            let l = ui.selectable_label("static label");
            let f = ui.add(TextEdit::new(field));
            (l, f)
        })
    };
    let (l, f) = show(&mut c, &mut field);
    click(&mut c, l.rect.center());
    show(&mut c, &mut field);
    ctrl(&mut c, KeyCode::KeyA);
    show(&mut c, &mut field);
    assert_eq!(c.selected_text().as_deref(), Some("static label"));
    click(&mut c, f.rect.center());
    show(&mut c, &mut field);
    assert!(c.selected_text().is_none(), "a press elsewhere cleared it");
    ctrl(&mut c, KeyCode::KeyA);
    show(&mut c, &mut field);
    assert!(
        c.selected_text().is_none(),
        "the field handled its own Ctrl+A"
    );
    assert!(c
        .probe()
        .text_edits
        .values()
        .any(|s| !s.buffer.selection().is_empty()));
}

#[test]
fn a_disabled_label_takes_no_selection() {
    let mut c = setup();
    let show = |c: &mut Context| frame(c, |ui| ui.add(SelectableLabel::new("nope").enabled(false)));
    let r = show(&mut c);
    c.move_pointer(r.rect.center());
    assert_eq!(c.cursor_icon(), CursorIcon::Default);
    drag(&mut c, r.rect.min, r.rect.max, &mut |c| {
        show(c);
    });
    up(&mut c);
    show(&mut c);
    ctrl(&mut c, KeyCode::KeyA);
    assert!(c.selected_text().is_none());
}

#[test]
fn selection_is_dimmed_without_window_focus_and_dropped_with_the_widget() {
    let mut c = setup();
    let text = "highlight";
    let r = select(&mut c, text, 0, 9);
    let fill = |c: &Context, id: Id| {
        let cached = &c.probe().cache[&id.with("selection")];
        match &cached.paint[0] {
            Paint::Shape(Shape::Rect { fill, .. }) => fill.0[3],
            other => panic!("{other:?}"),
        }
    };
    let active = fill(&c, r.id);
    c.on_window_event(&winit::event::WindowEvent::Focused(false));
    label(&mut c, text);
    assert!(fill(&c, r.id) < active, "muted in an inactive window");
    frame(&mut c, |_| {});
    assert!(
        c.selected_text().is_none(),
        "the selection leaves with its widget"
    );
}

#[test]
fn a_selection_scope_selects_across_labels_in_document_order() {
    let mut c = setup();
    let show = |c: &mut Context| {
        frame(c, |ui| {
            ui.selection_scope(|ui| {
                (
                    ui.selectable_label("alpha one"),
                    ui.selectable_label("beta two"),
                    ui.selectable_label("gamma three"),
                )
            })
        })
    };
    let (a, b, g) = show(&mut c);
    let from = Vec2::new(x_at(&mut c, a.rect, "alpha one", 6), a.rect.center().y);
    let to = Vec2::new(x_at(&mut c, g.rect, "gamma three", 5), g.rect.center().y);
    drag(&mut c, from, to, &mut |c| {
        show(c);
    });
    up(&mut c);
    show(&mut c);
    assert_eq!(c.selected_text().as_deref(), Some("one\nbeta two\ngamma"));
    // Upward gestures keep document order.
    c.clear_selection();
    show(&mut c);
    drag(&mut c, to, from, &mut |c| {
        show(c);
    });
    up(&mut c);
    show(&mut c);
    assert_eq!(c.selected_text().as_deref(), Some("one\nbeta two\ngamma"));
    click(&mut c, b.rect.center());
    show(&mut c);
    ctrl(&mut c, KeyCode::KeyA);
    show(&mut c);
    assert_eq!(
        c.selected_text().as_deref(),
        Some("alpha one\nbeta two\ngamma three")
    );
}

#[test]
fn a_scope_separator_joins_the_copy() {
    let mut c = setup();
    let show = |c: &mut Context| {
        frame(c, |ui| {
            SelectionScope::new().separator(" | ").show(ui, |ui| {
                (ui.selectable_label("left"), ui.selectable_label("right"))
            })
        })
    };
    let (l, _) = show(&mut c);
    click(&mut c, l.rect.center());
    show(&mut c);
    ctrl(&mut c, KeyCode::KeyA);
    show(&mut c);
    assert_eq!(c.selected_text().as_deref(), Some("left | right"));
}

#[test]
fn dragging_below_a_scroll_area_scrolls_and_extends_the_selection() {
    let mut c = setup();
    let start = Instant::now();
    let show = |c: &mut Context, at: Instant| {
        frame_at(c, at, |ui| {
            ScrollArea::vertical()
                .id_source("list")
                .max_height(120.0)
                .show(ui, |ui| {
                    ui.selection_scope(|ui| {
                        for i in 0..30 {
                            ui.selectable_label(format!("row {i:02}"));
                        }
                    });
                });
        })
    };
    show(&mut c, start);
    show(&mut c, start);
    let first = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::StaticText)
        .unwrap()
        .rect;
    down(&mut c, first.center());
    show(&mut c, start);
    let below = Vec2::new(first.center().x, first.min.y + 160.0);
    c.move_pointer(below);
    let mut t = start;
    for _ in 0..30 {
        t += std::time::Duration::from_millis(16);
        show(&mut c, t);
    }
    up(&mut c);
    show(&mut c, t);
    let copied = c.selected_text().unwrap();
    let rows = copied.lines().count();
    assert!(
        rows > 8,
        "autoscroll extended the selection beyond the viewport: {rows} rows"
    );
    assert!(copied.contains("row 10"), "{copied:?}");
}
