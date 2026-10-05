//! The selectable text engine: shortened text, the selection, very long blocks and the
//! copy button.

use super::*;
use winit::keyboard::ModifiersState;

fn window(context: &mut Context, build: impl FnOnce(&mut Ui<'_>)) {
    Window::new("Text").show(context, build);
}

fn only_label(harness: &Harness) -> NodeId {
    let labels = harness.tree.all(Role::Label);
    assert_eq!(labels.len(), 1, "one label");
    labels[0]
}

fn lines(runs: &[Node]) -> Vec<&str> {
    runs.iter().map(|run| run.value().unwrap()).collect()
}

/// The middle of character `index` of `run`, in logical pixels at scale 1.
fn point_in(run: &Node, index: usize) -> Vec2 {
    let area = bounds(run);
    let x =
        run.character_positions().unwrap()[index] + run.character_widths().unwrap()[index] * 0.5;
    Vec2::new(area.x0 as f32 + x, ((area.y0 + area.y1) * 0.5) as f32)
}

fn click(context: &mut Context, point: Vec2) {
    context.move_pointer(point);
    context.primary_button(ElementState::Pressed);
    context.primary_button(ElementState::Released);
}

fn select(harness: &mut Harness, id: NodeId, anchor: usize, focus: usize) -> bool {
    let run = text_runs(&harness.tree, id)[0];
    let at = |character_index| TextPosition {
        node: run,
        character_index,
    };
    let selection = TextSelection {
        anchor: at(anchor),
        focus: at(focus),
    };
    harness.act_with(
        id,
        Action::SetTextSelection,
        ActionData::SetTextSelection(selection),
    )
}

fn published(harness: &Harness, id: NodeId) -> Option<(usize, usize)> {
    let selection = harness.tree.node(id).text_selection()?;
    Some((
        selection.anchor.character_index,
        selection.focus.character_index,
    ))
}

#[test]
fn a_shortened_text_publishes_the_whole_value_and_the_line_it_shows() {
    let text = "A long sentence that will certainly not fit into the narrow column it was given.";
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.with_width(140.0, |ui| {
                ui.add(SelectableLabel::new(text).truncate(true))
            });
        });
    };
    harness.pass(build);
    let id = only_label(&harness);
    let runs = check_text(&harness, id, text, true);
    assert_eq!(
        runs.len(),
        2,
        "the shown line and the hidden rest: {:?}",
        lines(&runs)
    );
    let shown = runs[0].value().unwrap();
    assert!(
        text.starts_with(shown) && !shown.is_empty() && shown.len() < 40,
        "{shown:?}"
    );
    assert!(!shown.contains('…'), "the ellipsis is not part of the text");
    assert_eq!(placed(&runs), 1, "only the shown line has positions");
    assert_eq!(
        runs[0].next_on_line(),
        text_runs(&harness.tree, id).get(1).copied()
    );
    assert!(
        bounds(&runs[1]).x0 >= bounds(&runs[0]).x1 - 1.0,
        "the rest sits at the ellipsis"
    );
    assert_eq!(harness.pass(build), None);
}

#[test]
fn a_cut_at_a_line_break_keeps_every_paragraph_of_the_hidden_rest() {
    let mut harness = Harness::new();
    let text = "one\ntwo\nthree";
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.add(SelectableLabel::new(text).max_lines(1));
        });
    };
    harness.pass(build);
    let id = only_label(&harness);
    let runs = check_text(&harness, id, text, true);
    assert_eq!(lines(&runs), ["one", "\n", "two\n", "three"]);
    assert_eq!(placed(&runs), 1);
    // The break belongs to the line it ends; the hidden paragraphs are lines of their own.
    assert!(runs[1].previous_on_line().is_some() && runs[2].previous_on_line().is_none());
    assert_eq!(harness.pass(build), None);

    let mut harness = Harness::new();
    let text = "First paragraph that wraps over a few lines in this column.\nSecond.\nThird.";
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.with_width(120.0, |ui| ui.add(SelectableLabel::new(text).max_lines(2)));
        });
    };
    harness.pass(build);
    let runs = check_text(&harness, only_label(&harness), text, true);
    assert_eq!(placed(&runs), 2, "{:?}", lines(&runs));
    assert_eq!(harness.pass(build), None);
}

#[test]
fn the_selection_is_published_and_set_like_the_pointer_sets_it() {
    let text = "Hello brave new world";
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| window(ctx, |ui| _ = ui.selectable_label(text));
    harness.pass(build);
    let id = only_label(&harness);
    assert!(harness
        .tree
        .node(id)
        .supports_action(Action::SetTextSelection));
    assert_eq!(published(&harness, id), None);

    assert!(select(&mut harness, id, 6, 11));
    harness.settle(build);
    assert_eq!(harness.context.selected_text().as_deref(), Some("brave"));
    assert_eq!(published(&harness, id), Some((6, 11)));
    assert_eq!(
        harness.consumer_node(id).text_selection().unwrap().text(),
        "brave"
    );
    assert_eq!(
        harness.pass(build),
        None,
        "a selection at rest publishes nothing"
    );

    // Backwards: the anchor stays where the request put it.
    assert!(select(&mut harness, id, 15, 6));
    harness.settle(build);
    assert_eq!(
        harness.context.selected_text().as_deref(),
        Some("brave new")
    );
    assert_eq!(published(&harness, id), Some((15, 6)));

    // A double click selects a word through the same state, and the tree shows it.
    let run = harness.tree.node(text_runs(&harness.tree, id)[0]).clone();
    for _ in 0..2 {
        click(&mut harness.context, point_in(&run, 18));
        harness.pass(build);
    }
    harness.settle(build);
    assert_eq!(harness.context.selected_text().as_deref(), Some("world"));
    assert_eq!(published(&harness, id), Some((16, 21)));
    // Asking for the range that is already selected changes nothing.
    assert!(select(&mut harness, id, 16, 21));
    assert_eq!(harness.pass(build), None);
    assert_eq!(harness.context.selected_text().as_deref(), Some("world"));

    // A collapsed range is no selection; positions past the text are clamped.
    assert!(select(&mut harness, id, 3, 3));
    harness.settle(build);
    assert_eq!(harness.context.selected_text(), None);
    assert_eq!(published(&harness, id), None);
    assert!(select(&mut harness, id, 16, 999));
    harness.settle(build);
    assert_eq!(harness.context.selected_text().as_deref(), Some("world"));
}

#[test]
fn selection_requests_need_a_selectable_enabled_label() {
    let text = "Not for selecting";
    for enabled in [true, false] {
        let mut harness = Harness::new();
        let build = |ctx: &mut Context| {
            window(ctx, |ui| {
                let label = SelectableLabel::new(text);
                ui.add(if enabled {
                    label.selectable(false)
                } else {
                    label.enabled(false)
                });
            });
        };
        harness.pass(build);
        let id = only_label(&harness);
        check_text(&harness, id, text, true);
        let node = harness.tree.node(id);
        assert_eq!(node.is_disabled(), !enabled);
        assert!(!node.supports_action(Action::SetTextSelection));
        assert!(
            !node.supports_action(Action::Focus),
            "it is not a Tab stop either"
        );
        assert!(!select(&mut harness, id, 0, 3), "the request is refused");
        harness.settle(build);
        assert_eq!(harness.context.selected_text(), None);
    }
    // A request that names a run of another node is refused as well.
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.selectable_label("first");
            ui.selectable_label("second");
        });
    };
    harness.pass(build);
    let labels = harness.tree.all(Role::Label);
    let foreign = TextPosition {
        node: text_runs(&harness.tree, labels[1])[0],
        character_index: 2,
    };
    let selection = TextSelection {
        anchor: foreign,
        focus: foreign,
    };
    let data = ActionData::SetTextSelection(selection);
    assert!(!harness.act_with(labels[0], Action::SetTextSelection, data));
}

#[test]
fn a_selection_across_a_scope_is_published_by_every_label_it_covers() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.selection_scope(|ui| {
                ui.selectable_label("First");
                ui.selectable_label("Second");
                ui.selectable_label("Third");
            });
        });
    };
    harness.pass(build);
    let labels = harness.tree.all(Role::Label);
    let window = harness.tree.expect(Role::Window, "Text");
    assert_eq!(labels.len(), 3);
    for label in &labels {
        assert_eq!(
            harness.tree.parent(*label),
            Some(window),
            "the scope adds no node"
        );
    }
    assert!(harness.act(labels[1], Action::Focus));
    harness.pass(build);
    assert_eq!(harness.tree.focus(), labels[1]);
    harness.context.set_modifiers(ModifiersState::CONTROL);
    harness
        .context
        .on_key_event(KeyCode::KeyA, ElementState::Pressed, false);
    harness
        .context
        .on_key_event(KeyCode::KeyA, ElementState::Released, false);
    harness.context.set_modifiers(ModifiersState::empty());
    harness.settle(build);
    assert_eq!(
        harness.context.selected_text().as_deref(),
        Some("First\nSecond\nThird")
    );
    let spans: Vec<_> = labels.iter().map(|id| published(&harness, *id)).collect();
    assert_eq!(spans, [Some((0, 5)), Some((0, 6)), Some((0, 5))]);
    // A range inside the last label replaces the selection of the whole scope.
    assert!(select(&mut harness, labels[2], 0, 2));
    harness.settle(build);
    assert_eq!(harness.context.selected_text().as_deref(), Some("Th"));
    let spans: Vec<_> = labels.iter().map(|id| published(&harness, *id)).collect();
    assert_eq!(spans, [None, None, Some((0, 2))]);
    assert_eq!(harness.pass(build), None);
}

#[test]
fn a_very_long_block_positions_only_the_paragraphs_in_view() {
    let text: String = (0..2000).map(|i| format!("Line number {i}\n")).collect();
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            let area = ScrollArea::vertical().id_source("log").max_height(160.0);
            area.show(ui, |ui| _ = ui.add(SelectableLabel::new(text.as_str())));
        });
    };
    harness.pass(build);
    let id = only_label(&harness);
    let runs = check_text(&harness, id, &text, true);
    assert_eq!(runs.len(), 2001, "every paragraph is a line, placed or not");
    let first_placed = |runs: &[Node]| {
        let wide = |run: &Node| run.character_widths().unwrap().iter().any(|w| *w > 0.0);
        runs.iter().position(wide).expect("some lines are placed")
    };
    assert_eq!(first_placed(&runs), 0);
    assert!(
        (8..=96).contains(&placed(&runs)),
        "{} lines placed",
        placed(&runs)
    );
    assert_eq!(
        harness.pass(build),
        None,
        "an idle long block rebuilds nothing"
    );

    // Scrolling far moves the placed window; the text and its runs stay whole.
    let inside = bounds(&runs[2]);
    harness
        .context
        .move_pointer(Vec2::new(inside.x0 as f32 + 4.0, inside.y0 as f32 + 4.0));
    assert!(harness.context.scroll_wheel(Vec2::new(0.0, 9000.0)));
    advance(
        &mut harness,
        Instant::now(),
        Duration::from_millis(1500),
        build,
    );
    let runs = check_text(&harness, id, &text, true);
    assert_eq!(runs.len(), 2001);
    assert!(
        first_placed(&runs) >= 64,
        "the window followed the view: {}",
        first_placed(&runs)
    );
    assert!((8..=96).contains(&placed(&runs)));
    assert_eq!(harness.pass(build), None);
}

#[test]
fn the_lines_follow_the_text_and_the_width() {
    let mut harness = Harness::new();
    let mut text = String::from("A sentence that wraps in a narrow column.");
    let mut width = 400.0;
    let build = |ctx: &mut Context, text: &str, width: f32| {
        window(ctx, |ui| {
            ui.with_width(width, |ui| ui.selectable_label(text));
        });
    };
    harness.pass(|ctx| build(ctx, &text, width));
    let id = only_label(&harness);
    assert_eq!(check_text(&harness, id, &text, true).len(), 1);
    width = 110.0;
    assert!(harness.pass(|ctx| build(ctx, &text, width)).is_some());
    let wrapped = check_text(&harness, id, &text, true).len();
    assert!(wrapped >= 3, "{wrapped} lines");
    assert_eq!(harness.pass(|ctx| build(ctx, &text, width)), None);
    text.push_str(" And one more.");
    assert!(harness.pass(|ctx| build(ctx, &text, width)).is_some());
    assert!(check_text(&harness, id, &text, true).len() >= wrapped);
    assert_eq!(harness.pass(|ctx| build(ctx, &text, width)), None);
}

#[test]
fn the_copy_button_is_a_button_that_copies_once() {
    let mut harness = Harness::new();
    let clipboard = MemoryClipboard::new();
    harness.context.set_clipboard(clipboard.clone());
    let copies = Cell::new(0);
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            let output = SelectableLabel::new("token-123").copy_button().show(ui);
            count(&copies, output.copied);
        });
    };
    harness.pass(build);
    let copy = harness.tree.expect(Role::Button, "Copy");
    let label = only_label(&harness);
    assert_eq!(
        harness.tree.parent(copy),
        harness.tree.parent(label),
        "beside the text"
    );
    assert!(harness.act(copy, Action::Click));
    harness.settle(build);
    assert_eq!(copies.get(), 1);
    assert_eq!(clipboard.get().as_deref(), Some("token-123"));
}
