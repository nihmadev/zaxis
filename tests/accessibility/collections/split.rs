use super::*;
use zaxis::accesskit::Orientation;

struct Split {
    harness: Harness,
    vertical: bool,
    resizable: bool,
    /// Panel sizes along the axis after the last pass.
    sizes: Vec<f32>,
    /// Boundaries that reported a change, counted over all passes.
    changes: usize,
}

impl Split {
    fn new(vertical: bool) -> Self {
        let mut split = Self {
            harness: Harness::new(),
            vertical,
            resizable: true,
            sizes: Vec::new(),
            changes: 0,
        };
        split.pass();
        split
    }
    fn pass(&mut self) -> Option<usize> {
        let (vertical, resizable) = (self.vertical, self.resizable);
        let (sizes, changes) = (&mut self.sizes, &mut self.changes);
        self.harness.pass(|ctx| {
            Root::new().padding(Padding::all(0.0)).show(ctx, |ui| {
                let pane = if vertical {
                    SplitPane::vertical("split")
                } else {
                    SplitPane::horizontal("split")
                };
                let panels = (0..3).map(|i| SplitPanel::new(i).min_size(100.0).max_size(500.0));
                let out = pane.panels(panels).resizable(resizable).show(ui, |split| {
                    for i in 0..3 {
                        split.panel(i, |ui| {
                            ui.button(format!("Panel {i}"));
                        });
                    }
                });
                *sizes = out.panels.iter().map(|panel| panel.size).collect();
                *changes += out
                    .boundaries
                    .iter()
                    .filter(|boundary| boundary.changed)
                    .count();
            });
        })
    }
    fn settle(&mut self) {
        for _ in 0..4 {
            self.pass();
        }
    }
    fn handles(&self) -> Vec<NodeId> {
        self.harness.tree.all(Role::Splitter)
    }
    fn value(&self, handle: NodeId) -> (f64, f64, f64) {
        let node = self.harness.tree.node(handle);
        let range = (node.min_numeric_value(), node.max_numeric_value());
        (
            node.numeric_value().unwrap(),
            range.0.unwrap(),
            range.1.unwrap(),
        )
    }
    fn set(&mut self, handle: NodeId, value: f64) -> bool {
        let accepted =
            self.harness
                .act_with(handle, Action::SetValue, ActionData::NumericValue(value));
        self.settle();
        accepted
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn boundaries_are_splitters_between_their_panels() {
    let mut split = Split::new(false);
    let handles = split.handles();
    assert_eq!(handles.len(), 2);
    let tree = &split.harness.tree;
    // Panel, boundary, panel: the order a reader meets them on screen.
    let parent = tree.parent(handles[0]).unwrap();
    let order: Vec<_> = tree
        .node(parent)
        .children()
        .iter()
        .map(|id| match tree.node(*id).role() {
            Role::Splitter => "splitter".to_owned(),
            _ => tree.name(tree.node(*id).children()[0]),
        })
        .collect();
    assert_eq!(
        order,
        ["Panel 0", "splitter", "Panel 1", "splitter", "Panel 2"]
    );
    for (i, handle) in handles.iter().enumerate() {
        let node = tree.node(*handle);
        assert_eq!(tree.name(*handle), "Resize");
        assert_eq!(
            node.orientation(),
            Some(Orientation::Vertical),
            "a bar between columns"
        );
        assert!(node.supports_action(Action::Focus));
        for action in [Action::Increment, Action::Decrement, Action::SetValue] {
            assert!(node.supports_action(action), "{action:?}");
        }
        assert_eq!(node.numeric_value_step(), Some(4.0));
        let (value, min, max) = split.value(*handle);
        assert_eq!(
            value,
            f64::from(split.sizes[i]),
            "the size of the panel before it"
        );
        // Both neighbours stay between 100 and 500 pixels.
        let pair = f64::from(split.sizes[i]) + f64::from(split.sizes[i + 1]);
        let expected = (100.0_f64.max(pair - 500.0), 500.0_f64.min(pair - 100.0));
        assert!(
            close(min, expected.0) && close(max, expected.1),
            "{min}..{max} vs {expected:?}"
        );
        // The boundary sits where the two panels meet.
        let panels = children(&split.harness, parent, Role::GenericContainer);
        let bar = logical(&split.harness, *handle).center().x;
        let (left, right) = (
            logical(&split.harness, panels[i]),
            logical(&split.harness, panels[i + 1]),
        );
        assert!(
            left.max.x - 1.0 <= bar && bar <= right.min.x + 1.0,
            "{left:?} {bar} {right:?}"
        );
        assert!((left.size().x - split.sizes[i]).abs() < 1.0);
    }
    assert_eq!(split.pass(), None, "an idle pass publishes nothing");
    assert_eq!(split.changes, 0);
}

#[test]
fn increment_and_decrement_are_the_arrow_keys() {
    let mut by_request = Split::new(false);
    let handle = by_request.handles()[0];
    let before = by_request.sizes.clone();
    assert!(by_request.harness.act(handle, Action::Increment));
    by_request.settle();
    assert_eq!(by_request.changes, 1, "one request, one change");
    assert_eq!(by_request.sizes[0], before[0] + 4.0);
    assert_eq!(by_request.sizes[1], before[1] - 4.0);
    assert_eq!(
        by_request.sizes[2], before[2],
        "the third panel is not part of it"
    );
    assert_eq!(by_request.value(handle).0, f64::from(before[0] + 4.0));

    let mut by_key = Split::new(false);
    let handle = by_key.handles()[0];
    assert!(by_key.harness.act(handle, Action::Focus));
    by_key.pass();
    assert_eq!(by_key.harness.tree.focus(), handle);
    press(&mut by_key.harness.context, KeyCode::ArrowRight);
    by_key.settle();
    assert_eq!(by_key.sizes, by_request.sizes);
    assert_eq!(by_key.changes, 1);

    let handle = by_request.handles()[0];
    for _ in 0..2 {
        assert!(by_request.harness.act(handle, Action::Decrement));
        by_request.settle();
    }
    assert_eq!(by_request.changes, 3);
    assert_eq!(by_request.sizes[0], before[0] - 4.0);
    assert_eq!(by_request.pass(), None);
}

#[test]
fn a_value_is_set_within_the_range_of_the_boundary() {
    let mut split = Split::new(false);
    let handle = split.handles()[1];
    let (_, min, max) = split.value(handle);
    let pair = split.sizes[1] + split.sizes[2];
    assert!(split.set(handle, 180.0));
    assert_eq!(split.sizes[1], 180.0);
    assert!(
        close(f64::from(split.sizes[2]), f64::from(pair - 180.0)),
        "the pair keeps its sum"
    );
    assert_eq!(split.changes, 1);
    assert!(split.set(handle, 1.0e12));
    assert_eq!(
        f64::from(split.sizes[1]),
        max,
        "clamped to the largest size"
    );
    assert!(split.set(handle, -40.0));
    assert_eq!(
        f64::from(split.sizes[1]),
        min,
        "clamped to the smallest size"
    );
    let changes = split.changes;
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        split.set(handle, bad);
        assert_eq!(f64::from(split.sizes[1]), min, "{bad} is ignored");
    }
    assert_eq!(split.changes, changes);
    // The range of the other boundary follows: its second panel just changed size.
    let other = split.handles()[0];
    let pair = f64::from(split.sizes[0]) + f64::from(split.sizes[1]);
    assert!(close(split.value(other).2, 500.0_f64.min(pair - 100.0)));
    assert_eq!(split.pass(), None);
}

#[test]
fn a_vertical_split_has_horizontal_bars_that_move_down() {
    let mut split = Split::new(true);
    let handle = split.handles()[0];
    assert_eq!(
        split.harness.tree.node(handle).orientation(),
        Some(Orientation::Horizontal)
    );
    let before = split.sizes[0];
    assert!(split.harness.act(handle, Action::Increment));
    split.settle();
    assert_eq!(split.sizes[0], before + 4.0);
    let parent = split.harness.tree.parent(handle).unwrap();
    let panels = children(&split.harness, parent, Role::GenericContainer);
    let bar = logical(&split.harness, handle).center().y;
    let (above, below) = (
        logical(&split.harness, panels[0]),
        logical(&split.harness, panels[1]),
    );
    assert!(
        above.max.y - 1.0 <= bar && bar <= below.min.y + 1.0,
        "{above:?} {bar} {below:?}"
    );
}

#[test]
fn a_boundary_that_cannot_be_moved_refuses_requests() {
    let mut split = Split::new(false);
    let before = split.sizes.clone();
    split.resizable = false;
    split.settle();
    let handle = split.handles()[0];
    assert!(split.harness.tree.node(handle).is_disabled());
    assert!(!split.harness.act(handle, Action::Increment));
    assert!(!split.set(handle, 150.0));
    assert!(!split.harness.act(handle, Action::Focus));
    split.settle();
    assert_eq!(split.sizes, before);
    assert_eq!(split.changes, 0);
}
