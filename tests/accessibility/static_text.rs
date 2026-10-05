//! Static text (labels, selectable and rich text, links) and passive widgets: images,
//! cards, badges, progress, spinners, placeholders, and the wrappers that move content.

use crate::support::*;
use accesskit_consumer::{NodeRef, Tree, TreeChangeHandler};
use std::{cell::Cell, time::Duration};

mod block;
mod links;
mod text;
mod widgets;
mod wrappers;

struct Ignore;
impl TreeChangeHandler for Ignore {
    fn node_added(&mut self, _: &NodeRef) {}
    fn node_updated(&mut self, _: &NodeRef, _: &NodeRef) {}
    fn focus_moved(&mut self, _: Option<&NodeRef>, _: Option<&NodeRef>) {}
    fn node_removed(&mut self, _: &NodeRef) {}
}

/// [`Harness::pass`] at a chosen time, for animations and deadlines.
fn pass_at(harness: &mut Harness, at: Instant, build: impl FnOnce(&mut Context)) -> Option<usize> {
    harness.context.run_at(at, build);
    let count = harness.tree.sync(&mut harness.context)?;
    let update = harness.tree.last.clone().expect("an update was applied");
    match &mut harness.consumer {
        Some(consumer) => consumer.update_and_process_changes(update, &mut Ignore),
        None => harness.consumer = Some(Tree::new(update, true)),
    }
    harness.tree.validate();
    Some(count)
}

/// Passes every 16 ms for `time`, starting after `from`. Returns the time of the last one
/// and how many of them published something.
fn advance(
    harness: &mut Harness,
    from: Instant,
    time: Duration,
    mut build: impl FnMut(&mut Context),
) -> (Instant, usize) {
    let step = Duration::from_millis(16);
    let (mut now, mut updates) = (from, 0);
    while now < from + time {
        now += step;
        updates += usize::from(pass_at(harness, now, &mut build).is_some());
    }
    (now, updates)
}

fn text_runs(tree: &AccessTree, id: NodeId) -> Vec<NodeId> {
    let children = tree.node(id).children().iter().copied();
    children
        .filter(|child| tree.node(*child).role() == Role::TextRun)
        .collect()
}

fn bounds(node: &Node) -> accesskit::Rect {
    node.bounds().expect("the node has bounds")
}

/// What every text node must satisfy, whatever the text: the runs spell exactly `value`,
/// their character tables agree with each other, and the model platform adapters use
/// reads the same text. Returns the runs. `ltr` texts also have ordered positions.
#[track_caller]
fn check_text(harness: &Harness, id: NodeId, value: &str, ltr: bool) -> Vec<Node> {
    let tree = &harness.tree;
    assert_eq!(
        tree.node(id).value(),
        Some(value),
        "the value is the whole text"
    );
    let ids = text_runs(tree, id);
    assert!(!ids.is_empty(), "a text node has at least one run");
    let runs: Vec<Node> = ids.iter().map(|id| tree.node(*id).clone()).collect();
    let mut spelled = String::new();
    for (k, run) in runs.iter().enumerate() {
        let text = run.value().expect("a run has text");
        let lengths = run.character_lengths();
        let total: usize = lengths.iter().map(|len| usize::from(*len)).sum();
        assert_eq!(total, text.len(), "lengths cover the run {text:?}");
        let mut at = 0;
        for len in lengths {
            assert!(
                *len > 0 && text.is_char_boundary(at),
                "a character of {text:?} is cut"
            );
            at += usize::from(*len);
        }
        let positions = run.character_positions().expect("positions");
        let widths = run.character_widths().expect("widths");
        assert_eq!(positions.len(), lengths.len(), "positions of {text:?}");
        assert_eq!(widths.len(), lengths.len(), "widths of {text:?}");
        assert!(positions
            .iter()
            .chain(widths)
            .all(|v| v.is_finite() && *v >= 0.0));
        if ltr {
            assert_eq!(
                run.text_direction(),
                Some(accesskit::TextDirection::LeftToRight)
            );
            assert!(
                positions.windows(2).all(|pair| pair[0] <= pair[1]),
                "{positions:?}"
            );
        }
        for start in run.word_starts() {
            assert!(
                usize::from(*start) < lengths.len(),
                "word start past the run {text:?}"
            );
        }
        let box_ = bounds(run);
        assert!(box_.x0.is_finite() && box_.x1 >= box_.x0 && box_.y1 >= box_.y0);
        // Lines are linked exactly when one visual line was split into several runs.
        let next = run.next_on_line();
        assert_eq!(
            next.is_some(),
            next == ids.get(k + 1).copied() && next.is_some()
        );
        if let Some(next) = next {
            assert_eq!(tree.node(next).previous_on_line(), Some(ids[k]));
        }
        if let Some(previous) = run.previous_on_line() {
            assert_eq!(tree.node(previous).next_on_line(), Some(ids[k]));
        }
        spelled.push_str(text);
    }
    assert_eq!(spelled, value, "the runs spell the value");
    let node = harness.consumer_node(id);
    if node.role() == Role::Label {
        assert!(node.supports_text_ranges());
        assert_eq!(node.document_range().text(), value);
    }
    runs
}

/// Runs that have a place on screen: at least one character with a width.
fn placed(runs: &[Node]) -> usize {
    let wide = |run: &&Node| {
        run.character_widths()
            .is_some_and(|w| w.iter().any(|w| *w > 0.0))
    };
    runs.iter().filter(wide).count()
}

/// Count one event of a widget built inside a closure that runs every pass.
fn count(counter: &Cell<u32>, happened: bool) {
    counter.set(counter.get() + u32::from(happened));
}

fn missing_names(context: &Context) -> usize {
    let nameless = |d: &&Diagnostic| d.kind == DiagnosticKind::MissingAccessibleName;
    context.diagnostics().iter().filter(nameless).count()
}
