use super::*;
use std::collections::HashSet;
use zaxis::accesskit::Point;

const ROW: f32 = 24.0;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Item,
    Disabled,
    Header,
    Separator,
}

pub(crate) struct List {
    pub harness: Harness,
    rows: Vec<(u32, String, Kind)>,
    mode: ListMode,
    enabled: bool,
    empty: String,
    pub selection: HashSet<Id>,
    pub events: Vec<ListEvent>,
    pub built: usize,
}

impl List {
    pub fn numbered(count: u32, mode: ListMode) -> Self {
        let rows = (0..count)
            .map(|i| (i, format!("Row {i}"), Kind::Item))
            .collect();
        Self::new(rows, mode)
    }
    fn new(rows: Vec<(u32, String, Kind)>, mode: ListMode) -> Self {
        Self {
            harness: Harness::new(),
            rows,
            mode,
            enabled: true,
            empty: String::new(),
            selection: HashSet::new(),
            events: Vec::new(),
            built: 0,
        }
    }
    /// One pass; returns what the harness returns.
    pub fn pass(&mut self) -> Option<usize> {
        let (rows, selection, events, built) = (
            &self.rows,
            &mut self.selection,
            &mut self.events,
            &mut self.built,
        );
        let list = ListBox::new("files")
            .accessible_label("Files")
            .mode(self.mode)
            .enabled(self.enabled)
            .empty_text(self.empty.as_str())
            .selection(selection)
            .row_height(ROW)
            .max_height(240.0);
        self.harness.pass(|ctx| {
            Root::new().show(ctx, |ui| {
                let out = list.show_slice(
                    ui,
                    rows,
                    |row| match row.2 {
                        Kind::Header => ListEntry::header(row.0, &row.1),
                        Kind::Separator => ListEntry::separator(row.0),
                        kind => ListEntry::item(row.0, &row.1).enabled(kind == Kind::Item),
                    },
                    |ui, row, _| {
                        ui.label(row.text);
                    },
                );
                events.extend(out.events.iter().copied());
                *built = out.rows_built;
            });
        })
    }
    pub fn settle(&mut self) {
        for _ in 0..4 {
            self.pass();
        }
    }
    pub fn list(&self) -> NodeId {
        self.harness.tree.expect(Role::ListBox, "Files")
    }
    pub fn option(&self, name: &str) -> NodeId {
        self.harness.tree.expect(Role::ListBoxOption, name)
    }
    fn selected(&self) -> Vec<u32> {
        let mut keys: Vec<u32> = self.rows.iter().map(|row| row.0).collect();
        keys.retain(|key| self.selection.contains(&Id::new(key)));
        keys
    }
}

#[test]
fn a_list_publishes_its_options_with_selection_and_position() {
    let mut list = List::numbered(6, ListMode::Single);
    list.rows[4].2 = Kind::Disabled;
    list.selection.insert(Id::new(2u32));
    list.pass();
    let (tree, root) = (&list.harness.tree, list.list());
    assert!(!tree.node(root).is_multiselectable());
    assert!(tree.node(root).clips_children());
    assert_eq!(tree.node(root).scroll_y(), Some(0.0));
    assert!(
        tree.all(Role::ScrollView).is_empty(),
        "the list is the scroll container"
    );
    let options = children(&list.harness, root, Role::ListBoxOption);
    assert_eq!(options.len(), 6);
    for (i, id) in options.iter().enumerate() {
        let node = tree.node(*id);
        assert_eq!(tree.name(*id), format!("Row {i}"));
        assert_eq!(
            node.is_selected(),
            Some(i == 2),
            "every row says whether it is selected"
        );
        assert_eq!(
            (node.position_in_set(), node.size_of_set()),
            (Some(i), Some(6))
        );
        assert_eq!(node.is_disabled(), i == 4);
        assert_eq!(node.supports_action(Action::Click), i != 4);
        // The text the row draws is inside the option.
        let text = children(&list.harness, *id, Role::Label);
        assert_eq!(text.len(), 1);
        assert_eq!(
            tree.node(text[0]).value(),
            Some(format!("Row {i}").as_str())
        );
    }
    assert_eq!(list.pass(), None, "an idle pass publishes nothing");
    assert!(list.events.is_empty());
}

#[test]
fn headers_are_text_separators_are_absent_and_positions_count_items() {
    let rows = vec![
        (0, "Recent".to_owned(), Kind::Header),
        (1, "One".to_owned(), Kind::Item),
        (2, "Two".to_owned(), Kind::Item),
        (3, String::new(), Kind::Separator),
        (4, "Older".to_owned(), Kind::Header),
        (5, "Three".to_owned(), Kind::Item),
    ];
    let mut list = List::new(rows, ListMode::Multiple);
    list.pass();
    let (tree, root) = (&list.harness.tree, list.list());
    assert!(tree.node(root).is_multiselectable());
    let kinds: Vec<_> = tree
        .node(root)
        .children()
        .iter()
        .map(|id| tree.node(*id).role())
        .collect();
    let (option, label) = (Role::ListBoxOption, Role::Label);
    assert_eq!(kinds, [label, option, option, label, option]);
    let header = children(&list.harness, root, Role::Label)[1];
    assert_eq!(tree.node(header).value(), Some("Older"));
    assert!(!tree.node(header).supports_action(Action::Click));
    for (name, position) in [("One", 0), ("Two", 1), ("Three", 2)] {
        let node = tree.node(list.option(name));
        assert_eq!(
            (node.position_in_set(), node.size_of_set()),
            (Some(position), Some(3))
        );
    }
    assert_eq!(list.pass(), None);
}

#[test]
fn a_click_request_selects_like_a_pointer_click_with_one_event() {
    let mut by_request = List::numbered(8, ListMode::Single);
    by_request.pass();
    let row = by_request.option("Row 3");
    assert!(by_request.harness.act(row, Action::Click));
    by_request.settle();
    assert_eq!(
        by_request.events,
        [ListEvent::SelectionChanged],
        "one request, one event"
    );
    assert_eq!(by_request.selected(), [3]);
    assert_eq!(by_request.harness.tree.node(row).is_selected(), Some(true));

    let mut by_pointer = List::numbered(8, ListMode::Single);
    by_pointer.pass();
    let at = logical(&by_pointer.harness, by_pointer.option("Row 3")).center();
    click(&mut by_pointer.harness.context, at);
    by_pointer.settle();
    assert_eq!(by_pointer.events, by_request.events);
    assert_eq!(by_pointer.selected(), by_request.selected());
    // Both leave the keyboard on the same row.
    let active = |list: &List| list.harness.tree.node(list.list()).active_descendant();
    assert_eq!(active(&by_request), Some(row));
    assert_eq!(active(&by_pointer), Some(by_pointer.option("Row 3")));

    // Selecting the selected row again changes nothing and reports nothing.
    by_request.events.clear();
    assert!(by_request.harness.act(row, Action::Click));
    by_request.settle();
    assert!(by_request.events.is_empty());
    // Another row replaces the selection in single mode: again one event.
    let other = by_request.option("Row 5");
    assert!(by_request.harness.act(other, Action::Click));
    by_request.settle();
    assert_eq!(by_request.events, [ListEvent::SelectionChanged]);
    assert_eq!(by_request.selected(), [5]);
    assert_eq!(by_request.harness.tree.node(row).is_selected(), Some(false));
    assert_eq!(by_request.pass(), None);
}

#[test]
fn a_check_list_toggles_rows_on_click_requests() {
    let mut list = List::numbered(5, ListMode::Checks);
    list.pass();
    assert!(list.harness.tree.node(list.list()).is_multiselectable());
    for name in ["Row 1", "Row 3"] {
        let row = list.option(name);
        assert!(list.harness.act(row, Action::Click));
        list.settle();
    }
    assert_eq!(list.selected(), [1, 3]);
    assert_eq!(list.events.len(), 2);
    let row = list.option("Row 1");
    assert!(list.harness.act(row, Action::Click));
    list.settle();
    assert_eq!(list.selected(), [3]);
    assert_eq!(list.harness.tree.node(row).is_selected(), Some(false));
}

#[test]
fn disabled_rows_and_disabled_lists_refuse_requests() {
    let mut list = List::numbered(5, ListMode::Single);
    list.rows[2].2 = Kind::Disabled;
    list.pass();
    let row = list.option("Row 2");
    assert!(!list.harness.act(row, Action::Click));
    list.settle();
    assert!(list.selection.is_empty() && list.events.is_empty());

    list.enabled = false;
    list.settle();
    let (root, row) = (list.list(), list.option("Row 0"));
    assert!(list.harness.tree.node(root).is_disabled());
    assert!(list.harness.tree.node(row).is_disabled());
    assert!(!list.harness.act(row, Action::Click));
    assert!(!list.harness.act(root, Action::ScrollDown));
    assert!(!list.harness.act(root, Action::Focus));
    list.settle();
    assert!(list.selection.is_empty() && list.events.is_empty());
}

#[test]
fn focus_is_the_list_and_the_cursor_row_is_its_active_descendant() {
    let mut list = List::numbered(8, ListMode::Single);
    list.pass();
    let root = list.list();
    assert!(list.harness.tree.node(root).supports_action(Action::Focus));
    assert_ne!(list.harness.tree.focus(), root);
    assert!(list.harness.act(root, Action::Focus));
    list.settle();
    assert_eq!(list.harness.tree.focus(), root, "rows are not Tab stops");
    let active = |list: &List| list.harness.tree.node(list.list()).active_descendant();
    assert_eq!(active(&list), Some(list.option("Row 0")));
    for _ in 0..2 {
        press(&mut list.harness.context, KeyCode::ArrowDown);
        list.pass();
    }
    list.settle();
    assert_eq!(list.harness.tree.focus(), root);
    assert_eq!(active(&list), Some(list.option("Row 2")));
    assert_eq!(
        list.selected(),
        [2],
        "selection follows the keys in single mode"
    );
    assert_eq!(list.pass(), None);
}

#[test]
fn an_empty_list_says_so() {
    let mut list = List::numbered(0, ListMode::Single);
    list.empty = "No matches".into();
    list.pass();
    let root = list.list();
    let text = children(&list.harness, root, Role::Label);
    assert_eq!(text.len(), 1);
    assert_eq!(list.harness.tree.node(text[0]).value(), Some("No matches"));
    assert_eq!(list.pass(), None);
}

#[test]
fn a_far_row_is_reached_by_scrolling_and_keeps_its_place_in_the_whole_list() {
    let mut list = List::numbered(5000, ListMode::Single);
    list.pass();
    let root = list.list();
    let built = children(&list.harness, root, Role::ListBoxOption).len();
    assert!(
        built < 20,
        "only rows near the viewport have nodes: {built}"
    );
    assert_eq!(built, list.built);
    assert!(list
        .harness
        .tree
        .find(Role::ListBoxOption, "Row 4000")
        .is_none());
    let max = list.harness.tree.node(root).scroll_y_max().unwrap();
    assert_eq!(max, f64::from(5000.0 * ROW - 240.0));

    let target = ActionData::SetScrollOffset(Point::new(0.0, f64::from(4000.0 * ROW)));
    assert!(list.harness.act_with(root, Action::SetScrollOffset, target));
    list.settle();
    assert_eq!(
        list.harness.tree.node(root).scroll_y(),
        Some(f64::from(4000.0 * ROW))
    );
    let row = list.option("Row 4000");
    let node = list.harness.tree.node(row);
    assert_eq!(
        (node.position_in_set(), node.size_of_set()),
        (Some(4000), Some(5000))
    );
    assert!(children(&list.harness, root, Role::ListBoxOption).len() < 20);
    assert!(list
        .harness
        .tree
        .find(Role::ListBoxOption, "Row 0")
        .is_none());
    // The row at the top of the viewport is the one that was asked for.
    let (top, port) = (logical(&list.harness, row), logical(&list.harness, root));
    assert!((top.min.y - port.min.y).abs() < 0.5, "{top:?} in {port:?}");

    assert!(list.harness.act(row, Action::Click));
    list.settle();
    assert_eq!(list.selected(), [4000]);
    assert_eq!(list.events, [ListEvent::SelectionChanged]);

    // A row keeps its node while it scrolls: ids follow the key, not the slot.
    let neighbour = list.option("Row 4003");
    assert!(list.harness.act(root, Action::ScrollDown));
    list.settle();
    assert_eq!(
        list.harness.tree.node(root).scroll_y(),
        Some(f64::from(4000.0 * ROW + 48.0))
    );
    assert_eq!(list.option("Row 4003"), neighbour);
    assert!(
        list.harness.tree.get(row).is_none(),
        "a row that left the window has no node"
    );
    // A row just below the viewport is built; revealing it moves the list by what is missing.
    let below = list.option("Row 4012");
    assert!(list.harness.act(below, Action::ScrollIntoView));
    list.settle();
    let (rect, port) = (logical(&list.harness, below), logical(&list.harness, root));
    assert!(rect.max.y <= port.max.y + 0.5 && rect.min.y >= port.min.y - 0.5);
    assert_eq!(list.pass(), None);
}
