use super::*;

struct Files {
    items: Vec<ContextMenuItem>,
    picked: Vec<Id>,
    opened: u32,
    volume: f32,
    /// Attach with `Ui::context_menu` after the widget instead of wrapping it.
    direct: bool,
    enabled: bool,
}

impl Files {
    fn new() -> Self {
        Self {
            items: vec![
                ContextMenuItem::new("copy", "Copy").right_text("Ctrl+C"),
                ContextMenuItem::separator(),
                ContextMenuItem::new("export", "Export")
                    .left_text("01")
                    .enabled(false),
                ContextMenuItem::new("pin", "Pinned").checked(true),
            ],
            picked: Vec::new(),
            opened: 0,
            volume: 2.0,
            direct: false,
            enabled: true,
        }
    }

    fn build(&mut self, context: &mut Context) {
        Window::new("Files").show(context, |ui| {
            ui.add_enabled_ui(self.enabled, |ui| {
                let file = if self.direct {
                    let file = ui.button("report.txt");
                    self.picked
                        .extend(ui.context_menu(file, &self.items).selected);
                    file
                } else {
                    let file = ui.add(Button::new("report.txt").context_menu(&self.items));
                    self.picked.extend(file.menu_selected());
                    file
                };
                self.opened += u32::from(file.clicked());
                // A widget that takes the requests for its node itself.
                let slider = Slider::new(&mut self.volume, 0.0..=10.0)
                    .step(1.0)
                    .text("Volume");
                let slider = ui.add(slider.context_menu(&self.items));
                self.picked.extend(slider.menu_selected());
            });
        });
    }
}

fn open(harness: &mut Harness, files: &mut Files, role: Role, name: &str) -> NodeId {
    harness.pass(|ctx| files.build(ctx));
    let target = harness.tree.expect(role, name);
    assert!(harness.act(target, Action::ShowContextMenu));
    harness.settle(|ctx| files.build(ctx));
    target
}

#[test]
fn a_widget_with_a_context_menu_offers_to_show_it_and_stays_a_button() {
    let mut harness = still();
    let mut files = Files::new();
    harness.pass(|ctx| files.build(ctx));
    let file = harness.tree.expect(Role::Button, "report.txt");
    let node = harness.tree.node(file);
    assert!(node.supports_action(Action::ShowContextMenu));
    assert!(node.supports_action(Action::Click));
    assert_eq!(
        node.has_popup(),
        None,
        "a popup would turn the button into an expander"
    );
    assert!(harness.tree.all(Role::Menu).is_empty());
    assert_eq!(harness.pass(|ctx| files.build(ctx)), None);
    harness.act(file, Action::Click);
    harness.settle(|ctx| files.build(ctx));
    assert_eq!(files.opened, 1);
    assert!(harness.tree.all(Role::Menu).is_empty());
    assert!(harness.context.diagnostics().is_empty());
}

#[test]
fn the_menu_is_a_layer_of_rows_with_states_and_shortcuts() {
    let mut harness = still();
    let mut files = Files::new();
    let file = open(&mut harness, &mut files, Role::Button, "report.txt");
    let tree = &harness.tree;
    let menus = tree.all(Role::Menu);
    assert_eq!(menus.len(), 1);
    let menu = menus[0];
    assert!(!inside(tree, menu, tree.expect(Role::Window, "Files")));
    let rows: Vec<_> = tree
        .ids()
        .into_iter()
        .filter(|id| inside(tree, *id, menu) && *id != menu)
        .map(|id| (tree.node(id).role(), tree.name(id)))
        .collect();
    assert_eq!(
        rows,
        [
            (Role::MenuItem, "Copy".to_owned()),
            (Role::MenuItem, "01 Export".to_owned()),
            (Role::MenuItemCheckBox, "Pinned".to_owned()),
        ],
        "a separator is not a node"
    );
    let copy = tree.expect(Role::MenuItem, "Copy");
    assert_eq!(tree.node(copy).keyboard_shortcut(), Some("Ctrl+C"));
    assert!(tree.node(copy).supports_action(Action::Click));
    let export = tree.expect(Role::MenuItem, "01 Export");
    assert!(tree.node(export).is_disabled());
    assert!(!tree.act(&mut harness.context, export, Action::Click, None));
    let pinned = harness.node(Role::MenuItemCheckBox, "Pinned");
    assert_eq!(pinned.toggled(), Some(Toggled::True));
    // The menu holds focus; the highlighted row speaks through it.
    assert_eq!(tree.focus(), menu);
    assert_eq!(tree.node(menu).active_descendant(), Some(copy));
    // It opened at the widget it belongs to.
    let (button, panel) = (
        tree.node(file).bounds().unwrap(),
        tree.node(menu).bounds().unwrap(),
    );
    let at = middle(tree.node(file));
    assert!(button.x0 <= f64::from(at.x) && f64::from(at.x) <= button.x1);
    assert!((panel.x0 - f64::from(at.x)).abs() < 8.0 && (panel.y0 - f64::from(at.y)).abs() < 8.0);
    assert_eq!(harness.pass(|ctx| files.build(ctx)), None);
    assert_eq!(harness.pass(|ctx| files.build(ctx)), None);
    assert!(files.picked.is_empty());
    assert!(harness.context.diagnostics().is_empty());
}

#[test]
fn clicking_a_row_selects_once_and_closes_in_that_pass() {
    let mut harness = still();
    let mut files = Files::new();
    let file = open(&mut harness, &mut files, Role::Button, "report.txt");
    let pinned = harness.tree.expect(Role::MenuItemCheckBox, "Pinned");
    assert!(harness.act(pinned, Action::Click));
    harness.pass(|ctx| files.build(ctx));
    assert_eq!(files.picked, [Id::new("pin")]);
    assert!(
        harness.tree.all(Role::Menu).is_empty(),
        "gone with the pass that closed it"
    );
    assert!(harness.tree.all(Role::MenuItem).is_empty());
    harness.settle(|ctx| files.build(ctx));
    assert_eq!(files.picked.len(), 1);
    assert_eq!(harness.tree.focus(), file, "focus returns to the widget");
    assert_eq!(harness.pass(|ctx| files.build(ctx)), None);
}

#[test]
fn a_request_and_a_secondary_click_open_the_same_menu() {
    let mut harness = still();
    let mut files = Files::new();
    harness.pass(|ctx| files.build(ctx));
    let file = harness.tree.expect(Role::Button, "report.txt");
    let at = middle(harness.tree.node(file));
    harness.context.move_pointer(at);
    harness.context.secondary_button(ElementState::Pressed);
    harness.context.secondary_button(ElementState::Released);
    harness.settle(|ctx| files.build(ctx));
    let by_pointer = names(&harness.tree, Role::MenuItem);
    let copy = harness.tree.expect(Role::MenuItem, "Copy");
    press(&mut harness.context, middle(harness.tree.node(copy)));
    harness.settle(|ctx| files.build(ctx));
    assert_eq!(files.picked, [Id::new("copy")]);
    assert!(harness.tree.all(Role::Menu).is_empty());

    open(&mut harness, &mut files, Role::Button, "report.txt");
    assert_eq!(names(&harness.tree, Role::MenuItem), by_pointer);
    let copy = harness.tree.expect(Role::MenuItem, "Copy");
    harness.act(copy, Action::Click);
    harness.settle(|ctx| files.build(ctx));
    assert_eq!(files.picked, [Id::new("copy"), Id::new("copy")]);
    assert_eq!(files.opened, 0, "the menu never clicked the button");
}

#[test]
fn escape_and_an_outside_press_remove_the_menu() {
    let mut harness = still();
    let mut files = Files::new();
    open(&mut harness, &mut files, Role::Button, "report.txt");
    key(&mut harness.context, KeyCode::Escape);
    harness.pass(|ctx| files.build(ctx));
    assert!(
        harness.tree.all(Role::Menu).is_empty(),
        "gone with the pass that closed it"
    );
    open(&mut harness, &mut files, Role::Button, "report.txt");
    assert_eq!(harness.tree.all(Role::Menu).len(), 1);
    press(&mut harness.context, vec2(700.0, 500.0));
    harness.pass(|ctx| files.build(ctx));
    assert!(harness.tree.all(Role::Menu).is_empty());
    assert!(files.picked.is_empty());
}

#[test]
fn the_keyboard_moves_the_active_row() {
    let mut harness = still();
    let mut files = Files::new();
    open(&mut harness, &mut files, Role::Button, "report.txt");
    let menu = harness.tree.all(Role::Menu)[0];
    key(&mut harness.context, KeyCode::ArrowDown);
    harness.settle(|ctx| files.build(ctx));
    let pinned = harness.tree.expect(Role::MenuItemCheckBox, "Pinned");
    assert_eq!(harness.tree.node(menu).active_descendant(), Some(pinned));
    key(&mut harness.context, KeyCode::Enter);
    harness.settle(|ctx| files.build(ctx));
    assert_eq!(files.picked, [Id::new("pin")]);
}

#[test]
fn a_menu_attached_after_the_widget_works_the_same() {
    let mut harness = still();
    let mut files = Files::new();
    files.direct = true;
    open(&mut harness, &mut files, Role::Button, "report.txt");
    assert_eq!(harness.tree.all(Role::Menu).len(), 1);
    let copy = harness.tree.expect(Role::MenuItem, "Copy");
    harness.act(copy, Action::Click);
    harness.settle(|ctx| files.build(ctx));
    assert_eq!(files.picked, [Id::new("copy")]);
}

#[test]
fn a_widget_that_takes_its_own_requests_still_opens_its_menu() {
    let mut harness = still();
    let mut files = Files::new();
    let slider = open(&mut harness, &mut files, Role::Slider, "Volume");
    assert_eq!(harness.tree.all(Role::Menu).len(), 1);
    assert_eq!(files.volume, 2.0);
    key(&mut harness.context, KeyCode::Escape);
    harness.settle(|ctx| files.build(ctx));
    // Its own requests are not disturbed by the menu's.
    assert!(harness.act(slider, Action::Increment));
    harness.settle(|ctx| files.build(ctx));
    assert_eq!(files.volume, 3.0);
    assert!(harness.tree.all(Role::Menu).is_empty());
}

#[test]
fn a_disabled_target_refuses_the_request() {
    let mut harness = still();
    let mut files = Files::new();
    files.enabled = false;
    harness.pass(|ctx| files.build(ctx));
    let file = harness.tree.expect(Role::Button, "report.txt");
    assert!(!harness
        .tree
        .node(file)
        .supports_action(Action::ShowContextMenu));
    assert!(!harness.act(file, Action::ShowContextMenu));
    harness.settle(|ctx| files.build(ctx));
    assert!(harness.tree.all(Role::Menu).is_empty());
}
