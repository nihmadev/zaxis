use super::*;
use zaxis::accesskit::HasPopup;

struct App {
    items: Vec<MenuItem>,
    compact: bool,
    picked: Vec<Id>,
    open: bool,
}

impl App {
    fn new(compact: bool) -> Self {
        Self {
            items: vec![
                MenuItem::submenu(
                    "File",
                    [
                        MenuItem::new("new", "New").shortcut("Ctrl+N"),
                        MenuItem::separator(),
                        MenuItem::submenu(
                            "Recent",
                            [
                                MenuItem::new("r1", "one"),
                                MenuItem::new("r2", "two").enabled(false),
                                MenuItem::new("r3", "three"),
                            ],
                        ),
                        MenuItem::new("auto", "Autosave").checked(true),
                    ],
                ),
                MenuItem::submenu("Edit", [MenuItem::new("undo", "Undo")]),
                MenuItem::new("about", "About"),
                MenuItem::submenu("Empty", []).enabled(false),
            ],
            compact,
            picked: Vec::new(),
            open: false,
        }
    }

    fn build(&mut self, context: &mut Context) {
        Root::new().show(context, |ui| {
            let output = MenuBar::new("menu", &self.items)
                .compact(self.compact)
                .show(ui);
            self.picked.extend(output.selected);
            self.open = output.open;
            ui.button("Below");
        });
    }

    fn act(&mut self, harness: &mut Harness, role: Role, name: &str, action: Action) {
        let node = harness.tree.expect(role, name);
        assert!(harness.act(node, action), "{action:?} on {name}");
        harness.settle(|ctx| self.build(ctx));
    }
}

fn started(compact: bool) -> (Harness, App) {
    let mut harness = still();
    let mut app = App::new(compact);
    harness.pass(|ctx| app.build(ctx));
    (harness, app)
}

#[test]
fn the_bar_lists_its_titles_as_menu_items() {
    let (mut harness, mut app) = started(false);
    let tree = &harness.tree;
    let bars = tree.all(Role::MenuBar);
    assert_eq!(bars.len(), 1);
    let titles: Vec<_> = tree.node(bars[0]).children().to_vec();
    let described: Vec<_> = titles
        .iter()
        .map(|id| (tree.node(*id).role(), tree.name(*id)))
        .collect();
    assert_eq!(
        described,
        ["File", "Edit", "About", "Empty"].map(|name| (Role::MenuItem, name.to_owned()))
    );
    let file = harness.node(Role::MenuItem, "File");
    assert_eq!(file.is_expanded(), Some(false));
    assert_eq!(file.has_popup(), Some(HasPopup::Menu));
    assert_eq!(file.toggled(), None, "a title is not a toggle button");
    for action in [Action::Click, Action::Expand, Action::Collapse] {
        assert!(file.supports_action(action), "{action:?}");
    }
    let about = harness.node(Role::MenuItem, "About");
    assert_eq!((about.is_expanded(), about.has_popup()), (None, None));
    assert!(about.supports_action(Action::Click) && !about.supports_action(Action::Expand));
    assert!(harness.node(Role::MenuItem, "Empty").is_disabled());
    assert!(tree.all(Role::Menu).is_empty());
    assert!(tree
        .all(Role::Button)
        .iter()
        .all(|id| tree.name(*id) == "Below"));
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
    assert!(harness.context.diagnostics().is_empty());
}

#[test]
fn a_title_opens_its_panel_as_a_menu_layer() {
    let (mut harness, mut app) = started(false);
    app.act(&mut harness, Role::MenuItem, "File", Action::Click);
    assert!(app.open);
    let tree = &harness.tree;
    let file = tree.expect(Role::MenuItem, "File");
    let menu = tree.expect(Role::Menu, "File");
    assert_eq!(tree.node(file).is_expanded(), Some(true));
    assert_eq!(tree.node(file).controls(), [menu]);
    assert!(!inside(tree, menu, tree.all(Role::MenuBar)[0]));
    let rows: Vec<_> = tree
        .ids()
        .into_iter()
        .filter(|id| inside(tree, *id, menu) && *id != menu)
        .map(|id| (tree.node(id).role(), tree.name(id)))
        .collect();
    assert_eq!(
        rows,
        [
            (Role::MenuItem, "New".to_owned()),
            (Role::MenuItem, "Recent".to_owned()),
            (Role::MenuItemCheckBox, "Autosave".to_owned()),
        ]
    );
    assert_eq!(
        harness.node(Role::MenuItem, "New").keyboard_shortcut(),
        Some("Ctrl+N")
    );
    let recent = harness.node(Role::MenuItem, "Recent");
    assert_eq!(recent.is_expanded(), Some(false));
    assert_eq!(recent.has_popup(), Some(HasPopup::Menu));
    assert_eq!(
        harness.node(Role::MenuItemCheckBox, "Autosave").toggled(),
        Some(Toggled::True)
    );
    assert_eq!(tree.focus(), menu, "the open menu holds focus");
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
    assert!(app.picked.is_empty());
}

#[test]
fn clicking_the_open_title_closes_and_another_title_switches() {
    let (mut harness, mut app) = started(false);
    app.act(&mut harness, Role::MenuItem, "File", Action::Click);
    app.act(&mut harness, Role::MenuItem, "Edit", Action::Click);
    assert!(harness.tree.find(Role::Menu, "File").is_none());
    assert!(harness.tree.find(Role::Menu, "Edit").is_some());
    assert_eq!(
        harness.node(Role::MenuItem, "File").is_expanded(),
        Some(false)
    );
    assert_eq!(
        harness.node(Role::MenuItem, "Edit").is_expanded(),
        Some(true)
    );
    let edit = harness.tree.expect(Role::MenuItem, "Edit");
    assert!(harness.act(edit, Action::Click));
    harness.pass(|ctx| app.build(ctx));
    assert!(
        harness.tree.all(Role::Menu).is_empty(),
        "gone with the pass that closed it"
    );
    harness.settle(|ctx| app.build(ctx));
    assert!(!app.open);
    assert_eq!(
        harness.node(Role::MenuItem, "Edit").is_expanded(),
        Some(false)
    );
    assert!(app.picked.is_empty());
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
}

#[test]
fn expand_and_collapse_open_and_close_a_title() {
    let (mut harness, mut app) = started(false);
    app.act(&mut harness, Role::MenuItem, "File", Action::Expand);
    assert!(app.open && harness.tree.find(Role::Menu, "File").is_some());
    app.act(&mut harness, Role::MenuItem, "File", Action::Expand);
    assert!(app.open, "expanding twice keeps it open");
    app.act(&mut harness, Role::MenuItem, "Edit", Action::Collapse);
    assert!(app.open, "collapsing a closed title does nothing");
    app.act(&mut harness, Role::MenuItem, "File", Action::Collapse);
    assert!(!app.open && harness.tree.all(Role::Menu).is_empty());
    assert!(app.picked.is_empty());
}

#[test]
fn a_submenu_is_another_menu_layer() {
    let (mut harness, mut app) = started(false);
    app.act(&mut harness, Role::MenuItem, "File", Action::Click);
    app.act(&mut harness, Role::MenuItem, "Recent", Action::Click);
    let tree = &harness.tree;
    let (file, recent) = (
        tree.expect(Role::Menu, "File"),
        tree.expect(Role::Menu, "Recent"),
    );
    assert!(!inside(tree, recent, file));
    assert_eq!(
        harness.node(Role::MenuItem, "Recent").is_expanded(),
        Some(true)
    );
    let rows: Vec<_> = tree
        .all(Role::MenuItem)
        .into_iter()
        .filter(|id| inside(tree, *id, recent))
        .map(|id| (tree.name(id), tree.node(id).is_disabled()))
        .collect();
    assert_eq!(
        rows,
        [("one", false), ("two", true), ("three", false)].map(|(n, d)| (n.to_owned(), d))
    );
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
    assert!(harness.context.diagnostics().is_empty());
    // Collapse closes the submenu and leaves its parent open; expand reopens it.
    app.act(&mut harness, Role::MenuItem, "Recent", Action::Collapse);
    assert!(harness.tree.find(Role::Menu, "Recent").is_none());
    assert!(harness.tree.find(Role::Menu, "File").is_some());
    assert_eq!(
        harness.node(Role::MenuItem, "Recent").is_expanded(),
        Some(false)
    );
    app.act(&mut harness, Role::MenuItem, "Recent", Action::Expand);
    assert!(harness.tree.find(Role::Menu, "Recent").is_some());
    assert!(app.picked.is_empty());
}

#[test]
fn a_leaf_fires_once_and_every_panel_closes_in_that_pass() {
    let (mut harness, mut app) = started(false);
    app.act(&mut harness, Role::MenuItem, "File", Action::Click);
    app.act(&mut harness, Role::MenuItem, "Recent", Action::Click);
    let two = harness.tree.expect(Role::MenuItem, "two");
    assert!(!harness.act(two, Action::Click), "a disabled row");
    let three = harness.tree.expect(Role::MenuItem, "three");
    assert!(harness.act(three, Action::Click));
    harness.pass(|ctx| app.build(ctx));
    assert_eq!(app.picked, [Id::new("r3")]);
    assert!(
        harness.tree.all(Role::Menu).is_empty(),
        "gone with the pass that closed them"
    );
    assert_eq!(
        names(&harness.tree, Role::MenuItem),
        ["File", "Edit", "About", "Empty"]
    );
    harness.settle(|ctx| app.build(ctx));
    assert_eq!(app.picked.len(), 1);
    assert!(!app.open);
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);

    // A title without a panel is an action of its own.
    app.act(&mut harness, Role::MenuItem, "About", Action::Click);
    assert_eq!(app.picked, [Id::new("r3"), Id::new("about")]);
    // A checked row fires like any other.
    app.act(&mut harness, Role::MenuItem, "File", Action::Click);
    app.act(
        &mut harness,
        Role::MenuItemCheckBox,
        "Autosave",
        Action::Click,
    );
    assert_eq!(app.picked.last(), Some(&Id::new("auto")));
    assert_eq!(app.picked.len(), 3);
}

#[test]
fn a_request_and_the_pointer_fire_the_same_action() {
    let (mut harness, mut app) = started(false);
    let file = middle(harness.node(Role::MenuItem, "File"));
    press(&mut harness.context, file);
    harness.settle(|ctx| app.build(ctx));
    let new = middle(harness.node(Role::MenuItem, "New"));
    press(&mut harness.context, new);
    harness.settle(|ctx| app.build(ctx));
    assert_eq!(app.picked, [Id::new("new")]);
    app.act(&mut harness, Role::MenuItem, "File", Action::Click);
    app.act(&mut harness, Role::MenuItem, "New", Action::Click);
    assert_eq!(app.picked, [Id::new("new"), Id::new("new")]);
    assert!(harness.tree.all(Role::Menu).is_empty());
}

#[test]
fn the_keyboard_row_is_the_active_descendant_of_the_menu() {
    let (mut harness, mut app) = started(false);
    app.act(&mut harness, Role::MenuItem, "File", Action::Click);
    let menu = harness.tree.expect(Role::Menu, "File");
    assert_eq!(harness.tree.node(menu).active_descendant(), None);
    key(&mut harness.context, KeyCode::ArrowDown);
    harness.settle(|ctx| app.build(ctx));
    let new = harness.tree.expect(Role::MenuItem, "New");
    assert_eq!(harness.tree.node(menu).active_descendant(), Some(new));
    key(&mut harness.context, KeyCode::ArrowDown);
    key(&mut harness.context, KeyCode::ArrowRight);
    harness.settle(|ctx| app.build(ctx));
    let one = harness.tree.expect(Role::MenuItem, "one");
    assert_eq!(harness.tree.focus(), menu);
    assert_eq!(
        harness.tree.node(menu).active_descendant(),
        Some(one),
        "a row of the submenu"
    );
    key(&mut harness.context, KeyCode::Escape);
    harness.pass(|ctx| app.build(ctx));
    assert!(
        harness.tree.all(Role::Menu).is_empty(),
        "gone with the pass that closed them"
    );
}

#[test]
fn the_compact_menu_is_a_named_button_with_a_panel() {
    let (mut harness, mut app) = started(true);
    assert!(
        harness.context.diagnostics().is_empty(),
        "the hamburger has a name"
    );
    assert!(harness.tree.all(Role::MenuBar).is_empty());
    let button = harness.tree.expect(Role::Button, "Menu");
    let node = harness.tree.node(button);
    assert_eq!(node.is_expanded(), Some(false));
    assert_eq!(node.has_popup(), Some(HasPopup::Menu));
    assert_eq!(node.toggled(), None);
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);

    app.act(&mut harness, Role::Button, "Menu", Action::Expand);
    let menu = harness.tree.expect(Role::Menu, "Menu");
    assert_eq!(harness.tree.node(button).is_expanded(), Some(true));
    assert_eq!(harness.tree.focus(), menu);
    let titles: Vec<_> = harness
        .tree
        .all(Role::MenuItem)
        .into_iter()
        .map(|id| {
            (
                harness.tree.name(id),
                harness.tree.node(id).has_popup().is_some(),
            )
        })
        .collect();
    assert_eq!(
        titles,
        [
            ("File", true),
            ("Edit", true),
            ("About", false),
            ("Empty", true)
        ]
        .map(|(name, popup)| (name.to_owned(), popup))
    );
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
    app.act(&mut harness, Role::MenuItem, "Edit", Action::Click);
    app.act(&mut harness, Role::MenuItem, "Undo", Action::Click);
    assert_eq!(app.picked, [Id::new("undo")]);
    assert!(harness.tree.all(Role::Menu).is_empty());

    app.act(&mut harness, Role::Button, "Menu", Action::Click);
    assert!(app.open);
    app.act(&mut harness, Role::Button, "Menu", Action::Click);
    assert!(!app.open, "a click on the open hamburger closes it");
    app.act(&mut harness, Role::Button, "Menu", Action::Click);
    app.act(&mut harness, Role::Button, "Menu", Action::Collapse);
    assert!(!app.open && harness.tree.all(Role::Menu).is_empty());
    assert_eq!(app.picked.len(), 1);
}
