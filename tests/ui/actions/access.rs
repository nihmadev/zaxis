use super::{support::*, widgets::*};
use crate::prelude::*;
use zaxis::{
    accessibility::testing::AccessTree, accesskit::Role, Button, ContextMenu, ContextMenuItem,
    MenuBar, MenuItem,
};

#[test]
fn a_button_tells_assistive_technology_its_shortcut_and_description() {
    let mut c = setup();
    c.actions().registry_mut().insert(
        zaxis::Action::new("export", "Export")
            .shortcut(zaxis::Mods::PRIMARY.key(KeyCode::KeyE))
            .description("Write a copy"),
    );
    let mut tree = AccessTree::attach(&mut c);
    pass(&mut c, |ui| {
        ui.add(Button::action(Cmd::Save));
        ui.add(Button::action("export"));
        ui.add(Button::new("Plain"));
    });
    tree.sync(&mut c);
    let save = tree.expect(Role::Button, "Save");
    assert_eq!(tree.node(save).keyboard_shortcut(), Some("Ctrl+S"));
    let export = tree.expect(Role::Button, "Export");
    assert_eq!(tree.node(export).description(), Some("Write a copy"));
    let plain = tree.expect(Role::Button, "Plain");
    assert_eq!(tree.node(plain).keyboard_shortcut(), None);
}

#[test]
fn a_menu_item_tells_assistive_technology_its_shortcut_and_state() {
    let mut c = setup();
    let mut tree = AccessTree::attach(&mut c);
    let build = |c: &mut Context| {
        for _ in 0..2 {
            pass(c, |ui| {
                ui.actions().set_checked(Cmd::Open, true);
                ui.actions().set_enabled(Cmd::Delete, false);
                let items = [MenuItem::submenu(
                    "File",
                    [
                        MenuItem::action(Cmd::Save),
                        MenuItem::action(Cmd::Open),
                        MenuItem::action(Cmd::Delete),
                    ],
                )];
                MenuBar::new("bar", &items).show(ui);
            });
        }
    };
    build(&mut c);
    let title = rows_of_button(&c)[0];
    click(&mut c, title.center());
    build(&mut c);
    tree.sync(&mut c);
    let save = tree.expect(Role::MenuItem, "Save");
    assert_eq!(tree.node(save).keyboard_shortcut(), Some("Ctrl+S"));
    let open = tree.expect(Role::MenuItemCheckBox, "Open");
    assert_eq!(tree.node(open).keyboard_shortcut(), Some("Ctrl+O"));
    let delete = tree.expect(Role::MenuItem, "Delete");
    assert!(tree.node(delete).is_disabled());
}

#[test]
fn a_context_menu_item_tells_assistive_technology_its_shortcut() {
    let mut c = setup();
    let mut tree = AccessTree::attach(&mut c);
    let items = [ContextMenuItem::action(Cmd::Save)];
    let draw = |c: &mut Context, at: Option<Vec2>| {
        pass(c, |ui| {
            let target = ui.add(Button::new("Target").min_size(vec2(200.0, 60.0)));
            let mut menu = ContextMenu::new("m", &items);
            if let Some(p) = at {
                menu = menu.open_at(p);
            }
            menu.show(ui, target);
        })
    };
    draw(&mut c, Some(vec2(20.0, 20.0)));
    draw(&mut c, None);
    tree.sync(&mut c);
    let save = tree.expect(Role::MenuItem, "Save");
    assert_eq!(tree.node(save).keyboard_shortcut(), Some("Ctrl+S"));
}

#[test]
fn an_assistive_technology_click_runs_the_action_once() {
    let mut c = setup();
    let mut tree = AccessTree::attach(&mut c);
    let build = |c: &mut Context| {
        pass(c, |ui| {
            ui.add(Button::action(Cmd::Save));
        })
    };
    build(&mut c);
    tree.sync(&mut c);
    let save = tree.expect(Role::Button, "Save");
    tree.act(&mut c, save, zaxis::accesskit::Action::Click, None);
    build(&mut c);
    assert_eq!(taken(&mut c, &[Cmd::Save]), [Cmd::Save]);
    assert_eq!(taken(&mut c, &[Cmd::Save]), []);
}
