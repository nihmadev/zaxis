//! The text model of a plain label and of the selectable text engine, over texts that
//! break naive assumptions: clusters of several scalars, right-to-left runs, wrapping,
//! very long lines, line breaks and no room at all.

use super::*;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Plain,
    Selectable,
}

const KINDS: [Kind; 2] = [Kind::Plain, Kind::Selectable];

/// One label of `kind` in a column `width` wide; `wrap` off keeps each paragraph whole.
fn show(context: &mut Context, kind: Kind, text: &str, width: f32, wrap: bool) {
    Window::new("Text").show(context, |ui| {
        ui.with_width(width, |ui| match kind {
            Kind::Plain => ui.add(Text::new(text).wrap(wrap)),
            Kind::Selectable => ui.add(SelectableLabel::new(text).wrap(wrap)),
        });
    });
}

/// The label of a fresh window holding `text`, checked, with its runs.
#[track_caller]
fn label(kind: Kind, text: &str, width: f32, wrap: bool, ltr: bool) -> (Harness, Vec<Node>) {
    let mut harness = Harness::new();
    harness.pass(|ctx| show(ctx, kind, text, width, wrap));
    assert_eq!(
        harness.pass(|ctx| show(ctx, kind, text, width, wrap)),
        None,
        "{kind:?} idle"
    );
    let labels = harness.tree.all(Role::Label);
    assert_eq!(labels.len(), 1, "{kind:?}: one node per text");
    let runs = check_text(&harness, labels[0], text, ltr);
    (harness, runs)
}

fn lengths(runs: &[Node]) -> Vec<u8> {
    runs.iter()
        .flat_map(|run| run.character_lengths().to_vec())
        .collect()
}

#[test]
fn an_empty_text_is_one_empty_run() {
    for kind in KINDS {
        let (_, runs) = label(kind, "", 200.0, true, true);
        assert_eq!(runs.len(), 1, "{kind:?}");
        assert!(runs[0].character_lengths().is_empty());
    }
}

#[test]
fn a_cluster_of_several_scalars_is_one_character() {
    let family = "👨‍👩‍👧";
    let text = format!("{family} e\u{301}clair cafe\u{301}");
    for kind in KINDS {
        let (_, runs) = label(kind, &text, 400.0, true, true);
        let lengths = lengths(&runs);
        assert_eq!(
            usize::from(lengths[0]),
            family.len(),
            "{kind:?}: a ZWJ sequence"
        );
        assert_eq!(
            lengths[1..3],
            [1, 3],
            "{kind:?}: a letter and its combining mark"
        );
        assert_eq!(lengths.len(), text.graphemes(true).count(), "{kind:?}");
        assert_eq!(
            runs[0].word_starts()[..2],
            [0, 2],
            "{kind:?}: words start at clusters"
        );
    }
}

#[test]
fn right_to_left_and_mixed_text_keep_logical_order() {
    for kind in KINDS {
        let (_, runs) = label(kind, "שלום עולם", 400.0, true, false);
        assert_eq!(runs.len(), 1);
        assert_eq!(
            runs[0].text_direction(),
            Some(accesskit::TextDirection::RightToLeft)
        );
        // Distances from the leading (right) edge grow with the logical order.
        let positions = runs[0].character_positions().unwrap();
        assert!(
            positions.windows(2).all(|pair| pair[0] <= pair[1]),
            "{kind:?}: {positions:?}"
        );
        assert_eq!(runs[0].word_starts(), [0, 5]);

        let (_, runs) = label(kind, "abc שלום def", 400.0, true, false);
        assert_eq!(runs.len(), 1, "{kind:?}: one visual line is one run");
        assert_eq!(
            runs[0].text_direction(),
            Some(accesskit::TextDirection::LeftToRight)
        );
        assert_eq!(runs[0].word_starts(), [0, 4, 9]);
        let widths = runs[0].character_widths().unwrap();
        assert!(
            widths.iter().filter(|w| **w > 0.0).count() >= 10,
            "{kind:?}: {widths:?}"
        );
    }
}

#[test]
fn a_wrapped_paragraph_is_one_run_per_visual_line() {
    let text = "The quick brown fox jumps over the lazy dog and keeps running through the \
                field until the sun goes down behind the hills.";
    for kind in KINDS {
        let (_, runs) = label(kind, text, 150.0, true, true);
        assert!(runs.len() >= 3, "{kind:?}: {} runs", runs.len());
        for pair in runs.windows(2) {
            // A soft wrap starts a new line; only a split long line links its runs.
            assert!(pair[0].next_on_line().is_none() && pair[1].previous_on_line().is_none());
            assert!(
                bounds(&pair[1]).y0 >= bounds(&pair[0]).y1 - 1.0,
                "{kind:?}: lines stack"
            );
            assert!(
                !pair[0].value().unwrap().ends_with('\n'),
                "a soft wrap is no line break"
            );
        }
        assert!(runs
            .iter()
            .all(|run| bounds(run).x1 - bounds(run).x0 <= 151.0));
    }
}

#[test]
fn a_line_longer_than_a_run_is_split_into_linked_runs() {
    let text = "word ".repeat(130);
    assert!(text.len() > 600);
    for kind in KINDS {
        let (harness, runs) = label(kind, &text, 300.0, false, true);
        assert_eq!(
            runs.len(),
            3,
            "{kind:?}: 650 characters in runs of at most 240"
        );
        assert!(runs.iter().all(|run| run.character_lengths().len() <= 240));
        assert!(runs[0].previous_on_line().is_none() && runs[2].next_on_line().is_none());
        assert!(runs[0].next_on_line().is_some() && runs[1].next_on_line().is_some());
        let (first, last) = (bounds(&runs[0]), bounds(&runs[2]));
        assert_eq!(
            (first.y0, first.y1),
            (last.y0, last.y1),
            "{kind:?}: one visual line"
        );
        assert!(
            last.x0 >= first.x1 - 1.0,
            "{kind:?}: the parts follow each other"
        );
        // The adapter's model walks the three runs as one line.
        let id = harness.tree.all(Role::Label)[0];
        let line = harness
            .consumer_node(id)
            .document_start()
            .forward_to_line_end();
        assert!(line.is_document_end(), "{kind:?}");
    }
}

#[test]
fn hard_line_breaks_end_runs_and_empty_lines_are_kept() {
    let text = "one\ntwo\r\nthree\n\nfive\n";
    for kind in KINDS {
        let (_, runs) = label(kind, text, 300.0, true, true);
        let lines: Vec<&str> = runs.iter().map(|run| run.value().unwrap()).collect();
        assert_eq!(
            lines,
            ["one\n", "two\r\n", "three\n", "\n", "five\n", ""],
            "{kind:?}"
        );
        // The break is one character at the end of its line, without a width.
        assert_eq!(runs[1].character_lengths(), [1, 1, 1, 2]);
        assert_eq!(runs[1].character_widths().unwrap()[3], 0.0);
        assert!(
            bounds(&runs[4]).y0 > bounds(&runs[2]).y0,
            "{kind:?}: the empty line has height"
        );
    }
}

#[test]
fn no_room_at_all_still_publishes_valid_runs() {
    let text = "Narrow column of text";
    for kind in KINDS {
        for width in [0.0, 1.0, 7.0] {
            let (_, runs) = label(kind, text, width, true, true);
            assert!(!runs.is_empty(), "{kind:?} at {width}");
        }
    }
}

#[test]
fn character_positions_are_the_shaped_clusters() {
    let text = "Hello wide world";
    for scale in [1.0, 1.5] {
        for kind in KINDS {
            let mut harness = Harness::with_scale(scale);
            let build = |ctx: &mut Context| {
                Window::new("Text").show(ctx, |ui| match kind {
                    Kind::Plain => ui.add(Text::new(text).size(18.0).weight(FontWeight::BOLD)),
                    Kind::Selectable => ui.add(
                        SelectableLabel::new(text)
                            .size(18.0)
                            .weight(FontWeight::BOLD),
                    ),
                });
            };
            harness.pass(build);
            let id = harness.tree.all(Role::Label)[0];
            let runs = check_text(&harness, id, text, true);
            assert_eq!(runs.len(), 1);
            let layout = harness.context.paragraph_layout(
                text,
                18.0,
                FontWeight::BOLD,
                f32::INFINITY,
                DEFAULT_TAB,
            );
            let clusters = &layout.lines[0].clusters;
            let positions = runs[0].character_positions().unwrap();
            let widths = runs[0].character_widths().unwrap();
            assert_eq!(clusters.len(), positions.len());
            let left = clusters.iter().map(|c| c.x0).fold(f32::INFINITY, f32::min);
            let scale = scale as f32;
            for (i, cluster) in clusters.iter().enumerate() {
                let x = (cluster.x0 - left) * scale;
                let width = (cluster.x1 - cluster.x0) * scale;
                assert!(
                    (positions[i] - x).abs() < 0.01,
                    "{kind:?} {i}: {} vs {x}",
                    positions[i]
                );
                assert!((widths[i] - width).abs() < 0.01, "{kind:?} {i}");
            }
            // The run starts where the label does, and the adapter's model resolves the
            // box of one word from those positions.
            let (node, run) = (bounds(harness.tree.node(id)), bounds(&runs[0]));
            assert!(
                (run.x0 - node.x0).abs() <= f64::from(left * scale) + 1.0,
                "{kind:?}"
            );
            let start = harness
                .consumer_node(id)
                .document_start()
                .forward_to_word_end();
            let word = start.to_degenerate_range();
            let mut word = word;
            word.set_end(start.forward_to_word_end());
            assert_eq!(word.text(), "wide ");
            let boxes = word.bounding_boxes();
            assert_eq!(boxes.len(), 1);
            let expected = node.x0 + f64::from((clusters[6].x0) * scale);
            assert!(
                (boxes[0].x0 - expected).abs() <= 1.0,
                "{kind:?}: {boxes:?} vs {expected}"
            );
        }
    }
}

#[test]
fn headings_keep_their_level_in_both_kinds_of_text() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        Window::new("Text").show(ctx, |ui| {
            ui.title("Report");
            ui.add(SelectableLabel::new("Summary").typography(TypographyRole::Heading));
        });
    };
    harness.pass(build);
    assert_eq!(harness.node(Role::Heading, "Report").level(), Some(1));
    assert_eq!(harness.node(Role::Heading, "Summary").level(), Some(2));
    assert_eq!(harness.pass(build), None);
}
