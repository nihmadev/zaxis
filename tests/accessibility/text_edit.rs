use crate::support::*;
use winit::keyboard::ModifiersState;

fn field(context: &mut Context, text: &mut String) -> Response {
    let mut response = None;
    Window::new("Form").show(context, |ui| {
        response = Some(ui.add(TextEdit::new(text).placeholder("Name").id_source("name")));
    });
    response.unwrap()
}

fn area(context: &mut Context, text: &mut String) -> Response {
    let mut response = None;
    Window::new("Form").show(context, |ui| {
        let edit = TextEdit::new(text)
            .multiline()
            .rows(12.0)
            .placeholder("Notes")
            .width(160.0);
        response = Some(ui.add(edit.id_source("notes")));
    });
    response.unwrap()
}

/// The runs of `input` checked against the rules AccessKit states for them, and the text
/// they spell.
fn runs(harness: &Harness, input: NodeId) -> String {
    let mut text = String::new();
    for id in harness.tree.node(input).children() {
        let run = harness.tree.node(*id);
        assert_eq!(run.role(), Role::TextRun);
        let value = run.value().expect("a run has text");
        let lengths = run.character_lengths();
        let total: usize = lengths.iter().map(|len| usize::from(*len)).sum();
        assert_eq!(total, value.len(), "lengths cover the run {value:?}");
        let mut at = 0;
        for len in lengths {
            assert!(
                value.is_char_boundary(at),
                "a character starts inside a scalar"
            );
            at += usize::from(*len);
        }
        let positions = run.character_positions().expect("positions");
        let widths = run.character_widths().expect("widths");
        assert_eq!(positions.len(), lengths.len());
        assert_eq!(widths.len(), lengths.len());
        assert!(positions
            .iter()
            .chain(widths)
            .all(|v| v.is_finite() && *v >= 0.0));
        assert!(run
            .word_starts()
            .iter()
            .all(|i| usize::from(*i) < lengths.len().max(1)));
        text.push_str(value);
    }
    text
}

fn selection(harness: &Harness, input: NodeId) -> (usize, usize) {
    let node = harness.consumer_node(input);
    let range = node.text_selection().expect("a text selection");
    let offset = |position: accesskit_consumer::TextPosition<'_>| position.to_global_usv_index();
    (offset(range.start()), offset(range.end()))
}

#[test]
fn a_field_publishes_its_text_as_grapheme_runs() {
    let mut harness = Harness::new();
    let mut text = String::from("a👨‍👩‍👧e\u{301}b");
    harness.pass(|ctx| field(ctx, &mut text));
    let input = harness.tree.expect(Role::TextInput, "Name");
    let node = harness.tree.node(input);
    assert_eq!(node.value(), Some(text.as_str()));
    assert_eq!(node.placeholder(), Some("Name"));
    assert_eq!(runs(&harness, input), text);
    let run = harness.tree.node(node.children()[0]);
    assert_eq!(
        run.character_lengths(),
        &[1, 18, 3, 1],
        "one entry per grapheme cluster"
    );
    // The consumer reads the same document and puts the caret at its end.
    let consumer = harness.consumer_node(input);
    assert!(consumer.supports_text_ranges());
    assert_eq!(consumer.document_range().text(), text);
    let end = text.chars().count();
    assert_eq!(selection(&harness, input), (end, end));
}

#[test]
fn character_positions_are_the_shaped_caret_positions() {
    for scale in [1.0, 1.5] {
        let mut harness = Harness::with_scale(scale);
        let mut text = String::from("Wave AV fi");
        harness.pass(|ctx| field(ctx, &mut text));
        let input = harness.tree.expect(Role::TextInput, "Name");
        let run = harness
            .tree
            .node(harness.tree.node(input).children()[0])
            .clone();
        let size = harness.context.style().text_edit_font_size;
        let weight = harness.context.style().typography.weights.body;
        let carets = harness.context.text_carets(&text, size, weight);
        let positions = run.character_positions().expect("positions");
        let widths = run.character_widths().expect("widths");
        assert_eq!(positions.len(), text.len());
        for (i, position) in positions.iter().enumerate() {
            let (start, end) = (carets[i].1 * scale as f32, carets[i + 1].1 * scale as f32);
            assert!(
                (position - start).abs() < 0.01,
                "character {i} starts at its caret"
            );
            assert!(
                (widths[i] - (end - start)).abs() < 0.01,
                "and ends at the next one"
            );
        }
    }
}

#[test]
fn set_value_goes_through_the_buffer_and_undo() {
    let mut harness = Harness::new();
    let mut text = String::from("old");
    harness.pass(|ctx| field(ctx, &mut text));
    let input = harness.tree.expect(Role::TextInput, "Name");
    assert!(harness.act(input, Action::Focus));
    harness.pass(|ctx| field(ctx, &mut text));
    assert!(harness.act_with(
        input,
        Action::SetValue,
        ActionData::Value("new\r\nvalue 👍".into())
    ));
    let mut changes = 0;
    for _ in 0..3 {
        harness.pass(|ctx| changes += u32::from(field(ctx, &mut text).changed()));
    }
    assert_eq!(text, "newvalue 👍", "normalized like typed text");
    assert_eq!(changes, 1, "one request is one change");
    assert_eq!(harness.tree.node(input).value(), Some(text.as_str()));
    assert_eq!(runs(&harness, input), text);
    harness.context.set_modifiers(ModifiersState::CONTROL);
    harness
        .context
        .key(KeyCode::KeyZ, ElementState::Pressed, false);
    harness
        .context
        .key(KeyCode::KeyZ, ElementState::Released, false);
    harness.context.set_modifiers(ModifiersState::empty());
    harness.settle(|ctx| {
        field(ctx, &mut text);
    });
    assert_eq!(text, "old", "the request is one undo step");
}

#[test]
fn selection_requests_select_and_replace_whole_graphemes() {
    let mut harness = Harness::new();
    let mut text = String::from("ab👨‍👩‍👧cd");
    harness.pass(|ctx| field(ctx, &mut text));
    let input = harness.tree.expect(Role::TextInput, "Name");
    let run = harness.tree.node(input).children()[0];
    let at = |character_index| TextPosition {
        node: run,
        character_index,
    };
    let select = ActionData::SetTextSelection(TextSelection {
        anchor: at(2),
        focus: at(3),
    });
    assert!(harness.act_with(input, Action::SetTextSelection, select));
    harness.settle(|ctx| {
        field(ctx, &mut text);
    });
    let published = *harness
        .tree
        .node(input)
        .text_selection()
        .expect("selection");
    assert_eq!((published.anchor, published.focus), (at(2), at(3)));
    assert!(harness.act_with(
        input,
        Action::ReplaceSelectedText,
        ActionData::Value("é".into())
    ));
    harness.settle(|ctx| {
        field(ctx, &mut text);
    });
    assert_eq!(text, "abécd");
    let caret = *harness.tree.node(input).text_selection().expect("caret");
    assert_eq!((caret.anchor, caret.focus), (at(3), at(3)));
    // A position past the end and a run of another node are harmless.
    let stray = ActionData::SetTextSelection(TextSelection {
        anchor: at(400),
        focus: TextPosition {
            node: NodeId(0x5eed),
            character_index: 0,
        },
    });
    assert!(!harness.act_with(input, Action::SetTextSelection, stray));
    let past = ActionData::SetTextSelection(TextSelection {
        anchor: at(400),
        focus: at(400),
    });
    assert!(harness.act_with(input, Action::SetTextSelection, past));
    harness.settle(|ctx| {
        field(ctx, &mut text);
    });
    let end = text.chars().count();
    assert_eq!(selection(&harness, input), (end, end));
}

#[test]
fn an_empty_field_has_one_empty_line_and_a_valid_caret() {
    let mut harness = Harness::new();
    let mut text = String::new();
    harness.pass(|ctx| field(ctx, &mut text));
    let input = harness.tree.expect(Role::TextInput, "Name");
    assert_eq!(harness.tree.node(input).children().len(), 1);
    assert_eq!(runs(&harness, input), "");
    assert_eq!(selection(&harness, input), (0, 0));
    assert_eq!(harness.pass(|ctx| field(ctx, &mut text)), None, "idle pass");
}

#[test]
fn read_only_and_disabled_fields_refuse_edits() {
    let mut harness = Harness::new();
    let mut text = String::from("fixed");
    let build = |ctx: &mut Context, text: &mut String, enabled: bool| {
        Window::new("Form").show(ctx, |ui| {
            let edit = TextEdit::new(text).read_only(true).enabled(enabled);
            ui.add(edit.placeholder("Name"));
        });
    };
    harness.pass(|ctx| build(ctx, &mut text, true));
    let input = harness.tree.expect(Role::TextInput, "Name");
    assert!(harness.tree.node(input).is_read_only());
    let value = || ActionData::Value("changed".into());
    assert!(!harness.act_with(input, Action::SetValue, value()));
    assert!(!harness.act_with(input, Action::ReplaceSelectedText, value()));
    harness.pass(|ctx| build(ctx, &mut text, false));
    assert!(harness.tree.node(input).is_disabled());
    assert!(!harness.act(input, Action::Focus));
    harness.settle(|ctx| build(ctx, &mut text, false));
    assert_eq!(text, "fixed");
}

#[test]
fn a_text_area_publishes_one_run_per_visual_line() {
    let mut harness = Harness::new();
    let mut text = String::from("first line wraps around the narrow field\n\nשלום עולם\nlast");
    harness.settle(|ctx| {
        area(ctx, &mut text);
    });
    let input = harness.tree.expect(Role::MultilineTextInput, "Notes");
    assert_eq!(runs(&harness, input), text);
    let lines: Vec<_> = harness
        .tree
        .node(input)
        .children()
        .iter()
        .map(|id| harness.tree.node(*id).clone())
        .collect();
    assert!(
        lines.len() > 4,
        "the first paragraph wraps: {} runs",
        lines.len()
    );
    // Lines stack downwards, a hard break is one character at the end of its line, and
    // the empty paragraph is a line of its own.
    let tops: Vec<f64> = lines.iter().map(|run| run.bounds().unwrap().y0).collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{tops:?}");
    assert!(lines.iter().any(|run| run.value() == Some("\n")));
    let hebrew = lines
        .iter()
        .find(|run| run.value() == Some("שלום עולם\n"))
        .expect("line");
    assert_eq!(
        hebrew.text_direction(),
        Some(accesskit::TextDirection::RightToLeft)
    );
    assert_eq!(hebrew.character_lengths().last(), Some(&1));
    assert_eq!(harness.consumer_node(input).document_range().text(), text);
    assert_eq!(harness.pass(|ctx| area(ctx, &mut text)), None, "idle pass");
}

#[test]
fn a_text_area_edit_replaces_only_the_field_and_its_lines() {
    let mut harness = Harness::new();
    let mut text = String::from("one\ntwo");
    harness.settle(|ctx| {
        area(ctx, &mut text);
    });
    let input = harness.tree.expect(Role::MultilineTextInput, "Notes");
    assert!(harness.act_with(
        input,
        Action::SetValue,
        ActionData::Value("one\r\ntwo\rthree".into())
    ));
    harness.settle(|ctx| {
        area(ctx, &mut text);
    });
    assert_eq!(text, "one\ntwo\nthree", "line breaks are normalized");
    assert_eq!(runs(&harness, input), text);
    assert_eq!(harness.tree.node(input).children().len(), 3);
}

#[test]
fn composition_keeps_the_tree_valid() {
    let mut harness = Harness::new();
    let mut text = String::from("ab");
    harness.pass(|ctx| field(ctx, &mut text));
    let input = harness.tree.expect(Role::TextInput, "Name");
    harness.act(input, Action::Focus);
    harness.pass(|ctx| field(ctx, &mut text));
    let ime = |event| winit::event::WindowEvent::Ime(event);
    harness
        .context
        .on_window_event(&ime(winit::event::Ime::Enabled));
    let preedit = winit::event::Ime::Preedit("にほ".into(), Some((3, 3)));
    harness.context.on_window_event(&ime(preedit));
    harness.pass(|ctx| field(ctx, &mut text));
    assert_eq!(text, "ab", "the composition is not in the string yet");
    assert_eq!(
        runs(&harness, input),
        "abにほ",
        "but it is what the field shows"
    );
    harness
        .context
        .on_window_event(&ime(winit::event::Ime::Commit("日本".into())));
    harness.settle(|ctx| {
        field(ctx, &mut text);
    });
    assert_eq!(text, "ab日本");
    assert_eq!(runs(&harness, input), text);
}

#[test]
fn a_long_document_publishes_the_lines_in_view() {
    let mut harness = Harness::new();
    let mut text: String = (0..3000)
        .map(|i| {
            format!(
                "line {i}
"
            )
        })
        .collect();
    harness.settle(|ctx| area(ctx, &mut text));
    let input = harness.tree.expect(Role::MultilineTextInput, "Notes");
    let node = harness.tree.node(input);
    assert_eq!(
        node.value(),
        Some(text.as_str()),
        "the value is the whole text"
    );
    let shown = runs(&harness, input);
    assert!(shown.starts_with(
        "line 0
line 1
"
    ));
    assert!(
        node.children().len() < 40,
        "{} lines for a 12 row field",
        node.children().len()
    );
    assert_eq!(harness.pass(|ctx| area(ctx, &mut text)), None, "idle pass");
    // Moving the caret to the end brings the last lines into view and into the tree.
    assert!(harness.act(input, Action::Focus));
    harness.pass(|ctx| area(ctx, &mut text));
    // A new field has its caret at the end without showing it: go home first.
    for key in [KeyCode::Home, KeyCode::End] {
        harness.context.set_modifiers(ModifiersState::CONTROL);
        harness.context.key(key, ElementState::Pressed, false);
        harness.context.key(key, ElementState::Released, false);
        harness.context.set_modifiers(ModifiersState::empty());
        harness.settle(|ctx| area(ctx, &mut text));
    }
    let shown = runs(&harness, input);
    assert!(
        shown.ends_with(
            "line 2999
"
        ),
        "{shown:?}"
    );
    assert!(harness.tree.node(input).children().len() < 40);
    let node = harness.consumer_node(input);
    let caret = node.text_selection_focus().expect("a caret");
    assert!(
        caret.is_document_end(),
        "the caret is at the end of what is published"
    );
}
