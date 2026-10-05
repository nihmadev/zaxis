use super::*;
use std::time::Duration;
use zaxis::accesskit::HasPopup;

struct Shop {
    selected: Option<u32>,
    changes: u32,
    options: Vec<ComboBoxOption<u32>>,
    enabled: bool,
    filterable: bool,
}

impl Shop {
    fn new() -> Self {
        Self {
            selected: Some(1),
            changes: 0,
            options: vec![
                ComboBoxOption::new("apple", 1, "Apple"),
                ComboBoxOption::new("banana", 2, "Banana"),
                ComboBoxOption::new("durian", 3, "Durian").enabled(false),
                ComboBoxOption::new("cherry", 4, "Cherry"),
            ],
            enabled: true,
            filterable: false,
        }
    }

    fn build(&mut self, context: &mut Context) {
        Window::new("Shop").show(context, |ui| {
            let combo = ComboBox::new(&mut self.selected, &self.options)
                .id_source("fruit")
                .label("Fruit")
                .enabled(self.enabled)
                .filterable(self.filterable);
            if ui.add(combo).changed() {
                self.changes += 1;
            }
            ui.button("After");
        });
    }
}

fn open(harness: &mut Harness, shop: &mut Shop) -> NodeId {
    harness.pass(|ctx| shop.build(ctx));
    let combo = harness.tree.expect(Role::ComboBox, "Fruit");
    assert!(harness.act(combo, Action::Click));
    harness.settle(|ctx| shop.build(ctx));
    combo
}

#[test]
fn a_closed_combo_box_says_its_name_value_and_state() {
    let mut harness = still();
    let mut shop = Shop::new();
    harness.pass(|ctx| shop.build(ctx));
    let combo = harness.node(Role::ComboBox, "Fruit");
    assert_eq!(combo.value(), Some("Apple"));
    assert_eq!(combo.is_expanded(), Some(false));
    assert_eq!(combo.has_popup(), Some(HasPopup::Listbox));
    assert!(combo.controls().is_empty());
    for action in [
        Action::Click,
        Action::Expand,
        Action::Collapse,
        Action::SetValue,
    ] {
        assert!(combo.supports_action(action), "{action:?}");
    }
    assert!(harness.tree.all(Role::ListBox).is_empty());
    assert!(harness.tree.all(Role::ListBoxOption).is_empty());
    assert_eq!(harness.pass(|ctx| shop.build(ctx)), None);
    assert!(harness.context.diagnostics().is_empty());

    shop.selected = None;
    harness.pass(|ctx| shop.build(ctx));
    let combo = harness.node(Role::ComboBox, "Fruit");
    assert_eq!(combo.value(), None);
    assert_eq!(combo.placeholder(), Some("Select…"));
}

#[test]
fn an_open_combo_box_publishes_its_list_and_stays_quiet_when_idle() {
    let mut harness = still();
    let mut shop = Shop::new();
    let combo = open(&mut harness, &mut shop);
    let tree = &harness.tree;
    assert_eq!(tree.node(combo).is_expanded(), Some(true));
    let list = tree.expect(Role::ListBox, "Fruit");
    assert_eq!(tree.node(combo).controls(), [list]);
    // The list is a layer of its own, after the window that opened it.
    let window = tree.expect(Role::Window, "Shop");
    assert!(!inside(tree, list, window));
    let layers = tree.node(tree.root()).children();
    let position = |of: NodeId| layers.iter().position(|layer| inside(tree, of, *layer));
    assert!(position(list) > position(window));
    assert_eq!(
        names(tree, Role::ListBoxOption),
        ["Apple", "Banana", "Durian", "Cherry"]
    );
    for (index, id) in tree.all(Role::ListBoxOption).into_iter().enumerate() {
        let option = tree.node(id);
        assert!(inside(tree, id, list));
        assert_eq!(option.is_selected(), Some(index == 0), "{index}");
        assert_eq!(option.position_in_set(), Some(index));
        assert_eq!(option.size_of_set(), Some(4));
        assert_eq!(option.is_disabled(), index == 2);
        assert_eq!(option.supports_action(Action::Click), index != 2);
    }
    // Focus stays on the trigger; the highlighted option speaks through it.
    assert_eq!(tree.focus(), combo);
    let apple = tree.expect(Role::ListBoxOption, "Apple");
    assert_eq!(tree.node(combo).active_descendant(), Some(apple));
    assert_eq!(harness.pass(|ctx| shop.build(ctx)), None);
    assert_eq!(harness.pass(|ctx| shop.build(ctx)), None);
    assert_eq!(shop.changes, 0);
    assert!(harness.context.diagnostics().is_empty());
}

#[test]
fn clicking_an_option_selects_once_and_closes_in_that_pass() {
    let mut harness = still();
    let mut shop = Shop::new();
    open(&mut harness, &mut shop);
    let cherry = harness.tree.expect(Role::ListBoxOption, "Cherry");
    assert!(harness.act(cherry, Action::Click));
    harness.pass(|ctx| shop.build(ctx));
    assert_eq!((shop.selected, shop.changes), (Some(4), 1));
    assert!(
        harness.tree.all(Role::ListBox).is_empty(),
        "gone with the pass that closed it"
    );
    assert!(harness.tree.all(Role::ListBoxOption).is_empty());
    harness.settle(|ctx| shop.build(ctx));
    assert_eq!(shop.changes, 1);
    let combo = harness.node(Role::ComboBox, "Fruit");
    assert_eq!(combo.value(), Some("Cherry"));
    assert_eq!(combo.is_expanded(), Some(false));
    assert_eq!(combo.active_descendant(), None);
    assert_eq!(
        harness.tree.focus(),
        harness.tree.expect(Role::ComboBox, "Fruit")
    );
    assert_eq!(harness.pass(|ctx| shop.build(ctx)), None);
}

#[test]
fn a_request_and_the_pointer_choose_the_same_way() {
    let mut by_request = Shop::new();
    let mut harness = still();
    open(&mut harness, &mut by_request);
    let banana = harness.tree.expect(Role::ListBoxOption, "Banana");
    let at = middle(harness.tree.node(banana));
    harness.act(banana, Action::Click);
    harness.settle(|ctx| by_request.build(ctx));

    let mut by_pointer = Shop::new();
    let mut harness = still();
    open(&mut harness, &mut by_pointer);
    press(&mut harness.context, at);
    harness.settle(|ctx| by_pointer.build(ctx));
    assert_eq!((by_pointer.selected, by_pointer.changes), (Some(2), 1));
    assert_eq!(
        (by_request.selected, by_request.changes),
        (by_pointer.selected, by_pointer.changes)
    );
    assert!(harness.tree.all(Role::ListBox).is_empty());
}

#[test]
fn expand_and_collapse_only_act_when_they_change_something() {
    let mut harness = still();
    let mut shop = Shop::new();
    harness.pass(|ctx| shop.build(ctx));
    let combo = harness.tree.expect(Role::ComboBox, "Fruit");
    harness.act(combo, Action::Collapse);
    harness.settle(|ctx| shop.build(ctx));
    assert!(harness.tree.all(Role::ListBox).is_empty());
    harness.act(combo, Action::Expand);
    harness.settle(|ctx| shop.build(ctx));
    assert_eq!(harness.tree.all(Role::ListBox).len(), 1);
    harness.act(combo, Action::Expand);
    harness.settle(|ctx| shop.build(ctx));
    assert_eq!(
        harness.tree.all(Role::ListBox).len(),
        1,
        "expanding twice keeps it open"
    );
    harness.act(combo, Action::Collapse);
    harness.pass(|ctx| shop.build(ctx));
    assert!(
        harness.tree.all(Role::ListBox).is_empty(),
        "gone with the pass that closed it"
    );
    assert_eq!(
        harness.node(Role::ComboBox, "Fruit").is_expanded(),
        Some(false)
    );
    // A click toggles, as the pointer does on the trigger.
    harness.act(combo, Action::Click);
    harness.settle(|ctx| shop.build(ctx));
    harness.act(combo, Action::Click);
    harness.settle(|ctx| shop.build(ctx));
    assert!(harness.tree.all(Role::ListBox).is_empty());
    assert_eq!((shop.selected, shop.changes), (Some(1), 0));
}

#[test]
fn a_value_selects_the_option_of_that_name() {
    let mut harness = still();
    let mut shop = Shop::new();
    harness.pass(|ctx| shop.build(ctx));
    let combo = harness.tree.expect(Role::ComboBox, "Fruit");
    let set = |harness: &mut Harness, shop: &mut Shop, text: &str| {
        assert!(harness.act_with(combo, Action::SetValue, ActionData::Value(text.into())));
        harness.settle(|ctx| shop.build(ctx));
    };
    set(&mut harness, &mut shop, "Banana");
    assert_eq!((shop.selected, shop.changes), (Some(2), 1));
    assert_eq!(
        harness.node(Role::ComboBox, "Fruit").value(),
        Some("Banana")
    );
    set(&mut harness, &mut shop, "Banana");
    assert_eq!(shop.changes, 1, "the same value is not a change");
    set(&mut harness, &mut shop, "Durian");
    assert_eq!(
        (shop.selected, shop.changes),
        (Some(2), 1),
        "a disabled option"
    );
    set(&mut harness, &mut shop, "Papaya");
    assert_eq!(
        (shop.selected, shop.changes),
        (Some(2), 1),
        "no such option"
    );
    // With the list open the value closes it, like picking the row.
    harness.act(combo, Action::Expand);
    harness.settle(|ctx| shop.build(ctx));
    set(&mut harness, &mut shop, "Cherry");
    assert_eq!((shop.selected, shop.changes), (Some(4), 2));
    assert!(harness.tree.all(Role::ListBox).is_empty());
}

#[test]
fn a_disabled_combo_box_refuses_requests() {
    let mut harness = still();
    let mut shop = Shop::new();
    shop.enabled = false;
    harness.pass(|ctx| shop.build(ctx));
    let combo = harness.tree.expect(Role::ComboBox, "Fruit");
    assert!(harness.tree.node(combo).is_disabled());
    assert!(!harness.act(combo, Action::Click));
    assert!(!harness.act(combo, Action::Expand));
    assert!(!harness.act_with(combo, Action::SetValue, ActionData::Value("Banana".into())));
    harness.settle(|ctx| shop.build(ctx));
    assert!(harness.tree.all(Role::ListBox).is_empty());
    assert_eq!((shop.selected, shop.changes), (Some(1), 0));
}

#[test]
fn the_keyboard_moves_the_active_option_and_escape_closes() {
    let mut harness = still();
    let mut shop = Shop::new();
    let combo = open(&mut harness, &mut shop);
    key(&mut harness.context, KeyCode::ArrowDown);
    harness.settle(|ctx| shop.build(ctx));
    let banana = harness.tree.expect(Role::ListBoxOption, "Banana");
    assert_eq!(harness.tree.node(combo).active_descendant(), Some(banana));
    assert_eq!(harness.tree.focus(), combo);
    key(&mut harness.context, KeyCode::ArrowDown);
    harness.settle(|ctx| shop.build(ctx));
    let cherry = harness.tree.expect(Role::ListBoxOption, "Cherry");
    assert_eq!(
        harness.tree.node(combo).active_descendant(),
        Some(cherry),
        "the disabled option is skipped"
    );
    key(&mut harness.context, KeyCode::Escape);
    harness.pass(|ctx| shop.build(ctx));
    assert!(
        harness.tree.all(Role::ListBox).is_empty(),
        "gone with the pass that closed it"
    );
    assert_eq!(
        harness.node(Role::ComboBox, "Fruit").is_expanded(),
        Some(false)
    );
    assert_eq!((shop.selected, shop.changes), (Some(1), 0));
}

#[test]
fn an_outside_press_removes_the_list() {
    let mut harness = still();
    let mut shop = Shop::new();
    open(&mut harness, &mut shop);
    press(&mut harness.context, vec2(700.0, 500.0));
    harness.pass(|ctx| shop.build(ctx));
    assert!(harness.tree.all(Role::ListBox).is_empty());
    assert!(harness.tree.all(Role::ListBoxOption).is_empty());
    assert_eq!(shop.changes, 0);
}

#[test]
fn a_filter_field_is_part_of_the_open_popup() {
    let mut harness = still();
    let mut shop = Shop::new();
    shop.filterable = true;
    open(&mut harness, &mut shop);
    let tree = &harness.tree;
    let list = tree.expect(Role::ListBox, "Fruit");
    let filter = tree.expect(Role::TextInput, "Filter…");
    let window = tree.expect(Role::Window, "Shop");
    assert!(!inside(tree, filter, window) && !inside(tree, filter, list));
    assert_eq!(
        tree.parent(filter),
        tree.parent(list),
        "the field sits beside the list"
    );
    assert_eq!(tree.focus(), filter);
    assert_eq!(tree.node(filter).controls(), [list]);
    let apple = tree.expect(Role::ListBoxOption, "Apple");
    assert_eq!(tree.node(filter).active_descendant(), Some(apple));
    assert_eq!(harness.pass(|ctx| shop.build(ctx)), None);

    harness.context.on_text_event("ch");
    harness.settle(|ctx| shop.build(ctx));
    assert_eq!(names(&harness.tree, Role::ListBoxOption), ["Cherry"]);
    let cherry = harness.node(Role::ListBoxOption, "Cherry");
    assert_eq!(
        (cherry.position_in_set(), cherry.size_of_set()),
        (Some(0), Some(1))
    );
    harness.context.on_text_event("x");
    harness.settle(|ctx| shop.build(ctx));
    assert!(harness.tree.all(Role::ListBoxOption).is_empty());
    assert!(harness.tree.find(Role::Label, "No matches").is_some());
}

#[test]
fn a_long_list_reports_the_whole_set_and_scrolls() {
    let mut harness = still();
    let options: Vec<_> = (0..60u32)
        .map(|n| ComboBoxOption::new(n, n, format!("Item {n}")))
        .collect();
    let mut selected = Some(0);
    let mut build = |ctx: &mut Context| {
        Window::new("Shop").show(ctx, |ui| {
            ui.add(
                ComboBox::new(&mut selected, &options)
                    .id_source("long")
                    .label("Long"),
            );
        });
    };
    harness.pass(&mut build);
    let combo = harness.tree.expect(Role::ComboBox, "Long");
    harness.act(combo, Action::Expand);
    harness.settle(&mut build);
    let built = harness.tree.all(Role::ListBoxOption);
    assert!(
        !built.is_empty() && built.len() < 60,
        "rows are virtualized: {}",
        built.len()
    );
    assert_eq!(harness.tree.node(built[0]).size_of_set(), Some(60));
    let list = harness.tree.expect(Role::ListBox, "Long");
    assert!(harness.tree.node(list).supports_action(Action::ScrollDown));
    assert!(harness.act(list, Action::ScrollDown));
    harness.settle(&mut build);
    assert!(harness.tree.node(list).scroll_y().unwrap_or(0.0) > 0.0);
}

#[test]
fn the_opening_animation_ends_in_silence() {
    let mut harness = Harness::new();
    let mut shop = Shop::new();
    let mut spoken = Spoken::default();
    let start = Instant::now();
    let mut at = |harness: &mut Harness, shop: &mut Shop, ms: u64| {
        let now = start + Duration::from_millis(ms);
        pass_at(harness, now, &mut spoken, |ctx| shop.build(ctx))
    };
    at(&mut harness, &mut shop, 0);
    let combo = harness.tree.expect(Role::ComboBox, "Fruit");
    harness.act(combo, Action::Click);
    for ms in (16..2000).step_by(16) {
        at(&mut harness, &mut shop, ms);
    }
    assert_eq!(harness.tree.all(Role::ListBox).len(), 1);
    assert_eq!(at(&mut harness, &mut shop, 3000), None);
    assert_eq!(at(&mut harness, &mut shop, 3016), None);
    // Closing: the list is gone at once, the fading popup publishes nothing more.
    harness.act(combo, Action::Collapse);
    at(&mut harness, &mut shop, 3032);
    assert!(harness.tree.all(Role::ListBox).is_empty());
    for ms in (3048..5000).step_by(16) {
        at(&mut harness, &mut shop, ms);
        assert!(harness.tree.all(Role::ListBox).is_empty());
    }
    assert_eq!(at(&mut harness, &mut shop, 6000), None);
}
