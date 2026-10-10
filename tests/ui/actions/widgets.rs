//! Menus, context menus and buttons bound to actions.

use super::support::*;
use crate::prelude::*;
use zaxis::{Button, ContextMenu, ContextMenuItem, MenuBar, MenuItem, Rect};

pub fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}

/// Rectangles of the clickable controls outside popups, left to right.
pub fn rows_of_button(c: &Context) -> Vec<Rect> {
    let popup = c.probe().popup.as_ref().map(|p| p.id);
    let mut rects: Vec<Rect> = c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| h.action == HitAction::Activate && Some(h.window) != popup)
        .map(|h| h.rect)
        .collect();
    rects.sort_by(|a, b| {
        a.min
            .y
            .total_cmp(&b.min.y)
            .then(a.min.x.total_cmp(&b.min.x))
    });
    rects
}

/// Clickable rows of the open popup, top to bottom.
pub fn popup_rows(c: &Context) -> Vec<Rect> {
    let popup = c.probe().popup.as_ref().expect("a popup is open").id;
    let mut rows: Vec<Rect> = c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| {
            h.window == popup
                && matches!(h.action, HitAction::Activate | HitAction::Block)
                && h.rect.size().y <= 26.0
        })
        .map(|h| h.rect)
        .collect();
    rows.sort_by(|a, b| a.min.y.total_cmp(&b.min.y));
    rows
}

pub fn menu(c: &mut Context) {
    menu_passes(c, 2);
}

pub fn menu_passes(c: &mut Context, passes: usize) {
    for _ in 0..passes {
        pass(c, |ui| {
            let items = [
                MenuItem::submenu(
                    "File",
                    [MenuItem::action(Cmd::Save), MenuItem::action(Cmd::Open)],
                ),
                MenuItem::submenu("Edit", [MenuItem::new("undo", "Undo")]),
            ];
            let out = MenuBar::new("bar", &items).show(ui);
            assert_eq!(
                out.selected, None,
                "action items are not reported as selected"
            );
        });
    }
}

#[test]
fn a_menu_item_runs_its_action_once() {
    let mut c = setup();
    menu(&mut c);
    let title = rows_of_button(&c)[0];
    click(&mut c, title.center());
    menu(&mut c);
    let rows = popup_rows(&c);
    assert_eq!(rows.len(), 2);
    click(&mut c, rows[1].center());
    menu_passes(&mut c, 1);
    assert_eq!(taken(&mut c, &[Cmd::Save, Cmd::Open]), [Cmd::Open]);
    assert_eq!(taken(&mut c, &[Cmd::Save, Cmd::Open]), []);
    assert!(c.probe().popup.is_none(), "the menu closed");
}

#[test]
fn a_disabled_menu_item_cannot_be_chosen() {
    let mut c = setup();
    let build = |c: &mut Context| {
        for _ in 0..2 {
            pass(c, |ui| {
                ui.actions().set_enabled(Cmd::Open, false);
                let items = [MenuItem::submenu(
                    "File",
                    [MenuItem::action(Cmd::Save), MenuItem::action(Cmd::Open)],
                )];
                MenuBar::new("bar", &items).show(ui);
            });
        }
    };
    build(&mut c);
    let title = rows_of_button(&c)[0];
    click(&mut c, title.center());
    build(&mut c);
    let rows = popup_rows(&c);
    click(&mut c, rows[1].center());
    build(&mut c);
    assert_eq!(taken(&mut c, &[Cmd::Open]), []);
}

#[test]
fn a_context_menu_item_runs_its_action_once() {
    let mut c = setup();
    let items = [
        ContextMenuItem::action(Cmd::Save),
        ContextMenuItem::new("plain", "Plain"),
    ];
    let draw = |c: &mut Context, at: Option<Vec2>| {
        pass(c, |ui| {
            let target = ui.add(Button::new("Target").min_size(vec2(200.0, 60.0)));
            let mut menu = ContextMenu::new("m", &items);
            if let Some(p) = at {
                menu = menu.open_at(p);
            }
            let out = menu.show(ui, target);
            assert_eq!(out.selected, None);
        });
    };
    draw(&mut c, Some(vec2(20.0, 20.0)));
    draw(&mut c, None);
    let rows = popup_rows(&c);
    click(&mut c, rows[0].center());
    draw(&mut c, None);
    assert_eq!(taken(&mut c, &[Cmd::Save]), [Cmd::Save]);
}

#[test]
fn a_button_follows_the_state_of_its_action() {
    let mut c = setup();
    let build = |c: &mut Context, enabled: bool| {
        pass(c, |ui| {
            ui.actions().set_enabled(Cmd::Save, enabled);
            ui.add(Button::action(Cmd::Save));
        })
    };
    build(&mut c, false);
    let rect = rows_of_button(&c);
    assert!(rect.is_empty(), "a disabled button is not clickable");
    build(&mut c, true);
    let rect = rows_of_button(&c)[0];
    click(&mut c, rect.center());
    build(&mut c, true);
    assert_eq!(taken(&mut c, &[Cmd::Save]), [Cmd::Save]);
}

#[test]
fn a_disabled_button_does_not_run_its_action() {
    let mut c = setup();
    let build = |c: &mut Context| {
        pass(c, |ui| {
            ui.actions().set_enabled(Cmd::Save, false);
            ui.add(Button::action(Cmd::Save));
        })
    };
    build(&mut c);
    build(&mut c);
    click(&mut c, vec2(30.0, 20.0));
    build(&mut c);
    assert_eq!(taken(&mut c, &[Cmd::Save]), []);
}

#[test]
fn a_key_an_open_menu_uses_is_not_used_again_by_the_registry() {
    use zaxis::{Action, Actions};
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(640, 420), 1.0);
    c.set_actions(
        Actions::new()
            .register(Action::new(Cmd::Save, "Save").shortcut(KeyCode::ArrowDown))
            .register(Action::new(Cmd::Open, "Open").shortcut(KeyCode::KeyX)),
    );
    menu(&mut c);
    let title = rows_of_button(&c)[0];
    click(&mut c, title.center());
    menu(&mut c);
    assert!(c.probe().popup.is_some());
    assert!(press(
        &mut c,
        winit::keyboard::ModifiersState::empty(),
        KeyCode::ArrowDown
    ));
    let mut ran = true;
    pass(&mut c, |ui| {
        let items = [MenuItem::submenu("File", [MenuItem::action(Cmd::Save)])];
        MenuBar::new("bar", &items).show(ui);
        ran = ui.actions().triggered(Cmd::Save);
    });
    assert!(!ran, "the menu took the arrow");
    assert!(c.probe().popup.is_some(), "and the menu is still open");
    // While a menu is open it owns the keyboard: other shortcuts wait.
    assert!(!press(
        &mut c,
        winit::keyboard::ModifiersState::empty(),
        KeyCode::KeyX
    ));
}
