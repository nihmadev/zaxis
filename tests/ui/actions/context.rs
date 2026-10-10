use super::support::*;
use crate::prelude::*;
use winit::keyboard::ModifiersState;
use zaxis::{Action, Actions, KeyBox, Mods, TextEdit};

const WATCHED: [Cmd; 4] = [Cmd::Find, Cmd::FindNext, Cmd::Save, Cmd::Delete];

fn in_context(c: &mut Context, path: &str) {
    pass(c, |ui| ui.actions().set_context(path));
}

#[test]
fn a_scoped_binding_applies_only_inside_its_context() {
    let mut c = setup();
    idle(&mut c);
    assert!(!press(&mut c, CTRL, KeyCode::KeyF), "no editor, no Find");
    in_context(&mut c, "editor");
    assert!(press(&mut c, CTRL, KeyCode::KeyF));
    assert_eq!(taken(&mut c, &WATCHED), [Cmd::Find]);
}

#[test]
fn the_narrowest_context_wins_and_the_wider_ones_stay_available() {
    let mut c = setup();
    in_context(&mut c, "editor/find");
    press(&mut c, CTRL, KeyCode::KeyF);
    assert_eq!(
        taken(&mut c, &WATCHED),
        [Cmd::FindNext],
        "inner binding first"
    );
    // Unscoped bindings are the last level.
    in_context(&mut c, "editor/find");
    press(&mut c, CTRL, KeyCode::KeyS);
    assert_eq!(taken(&mut c, &WATCHED), [Cmd::Save]);
}

#[test]
fn a_context_lasts_one_pass() {
    let mut c = setup();
    in_context(&mut c, "editor");
    idle(&mut c);
    assert!(!press(&mut c, CTRL, KeyCode::KeyF));
}

#[test]
fn a_sibling_context_does_not_apply() {
    let mut c = setup();
    in_context(&mut c, "editorial");
    assert!(
        !press(&mut c, CTRL, KeyCode::KeyF),
        "`editorial` is not inside `editor`"
    );
}

#[test]
fn a_disabled_action_lets_the_wider_context_answer() {
    let mut c = setup();
    pass(&mut c, |ui| {
        ui.actions().set_context("editor/find");
        ui.actions().set_enabled(Cmd::FindNext, false);
    });
    press(&mut c, CTRL, KeyCode::KeyF);
    assert_eq!(taken(&mut c, &WATCHED), [Cmd::Find]);
}

fn text_field(c: &mut Context, text: &mut String) {
    pass(c, |ui| {
        ui.add(TextEdit::new(text).id_source("field"));
    });
}

fn focus_field(c: &mut Context, text: &mut String) {
    text_field(c, text);
    let spot = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::TextEdit)
        .expect("the field")
        .rect
        .center();
    c.move_pointer(spot);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    text_field(c, text);
    assert!(c.probe().focused_widget.is_some(), "the field has focus");
}

#[test]
fn a_text_field_keeps_printable_keys_and_editing_shortcuts() {
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(400, 200), 1.0);
    c.set_actions(
        Actions::new()
            .register(Action::new(Cmd::Delete, "Delete").shortcut(KeyCode::Delete))
            .register(Action::new(Cmd::Find, "Find").shortcut(KeyCode::KeyF))
            .register(Action::new(Cmd::Open, "Open").shortcut(Mods::PRIMARY.key(KeyCode::KeyA)))
            .register(Action::new(Cmd::Save, "Save").shortcut(Mods::PRIMARY.key(KeyCode::KeyS)))
            .register(Action::new(Cmd::Reload, "Reload").shortcut(KeyCode::F5)),
    );
    let mut text = String::from("hello");
    focus_field(&mut c, &mut text);
    press(&mut c, ModifiersState::empty(), KeyCode::Delete);
    press(&mut c, ModifiersState::empty(), KeyCode::KeyF);
    press(&mut c, CTRL, KeyCode::KeyA);
    assert_eq!(taken(&mut c, &[Cmd::Delete, Cmd::Find, Cmd::Open]), []);
    // Ctrl+S and function keys pass through to the registry.
    press(&mut c, CTRL, KeyCode::KeyS);
    press(&mut c, ModifiersState::empty(), KeyCode::F5);
    assert_eq!(
        taken(&mut c, &[Cmd::Save, Cmd::Reload]),
        [Cmd::Save, Cmd::Reload]
    );
}

#[test]
fn without_text_focus_a_bare_key_runs_its_action() {
    let mut c = setup();
    idle(&mut c);
    assert!(press(&mut c, ModifiersState::empty(), KeyCode::Delete));
    assert_eq!(taken(&mut c, &WATCHED), [Cmd::Delete]);
}

#[test]
fn a_key_box_that_listens_takes_the_keys_before_the_actions() {
    let mut c = setup();
    let mut binding = zaxis::KeyBinding::None;
    let build = |c: &mut Context, binding: &mut zaxis::KeyBinding| {
        pass(c, |ui| {
            ui.add(KeyBox::new(binding, "key"));
        })
    };
    build(&mut c, &mut binding);
    let spot = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.action != HitAction::Block)
        .unwrap()
        .rect
        .center();
    c.move_pointer(spot);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    build(&mut c, &mut binding);
    assert!(c.key_capture_active());
    press(&mut c, CTRL, KeyCode::KeyS);
    build(&mut c, &mut binding);
    assert_eq!(taken(&mut c, &WATCHED), [], "Ctrl+S was recorded, not run");
    assert_eq!(binding, zaxis::KeyBinding::Key(KeyCode::KeyS));
    assert!(press(&mut c, CTRL, KeyCode::KeyS), "listening is over");
}

#[test]
fn a_modal_blocks_actions_unless_they_allow_it() {
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(400, 300), 1.0);
    c.set_actions(
        Actions::new()
            .register(Action::new(Cmd::Save, "Save").shortcut(Mods::PRIMARY.key(KeyCode::KeyS)))
            .register(
                Action::new(Cmd::Open, "Open")
                    .shortcut(Mods::PRIMARY.key(KeyCode::KeyO))
                    .allowed_in_modal(true),
            ),
    );
    let show = |c: &mut Context| {
        pass(c, |ui| {
            let mut open = true;
            zaxis::Modal::new("dialog").show(ui, &mut open, |ui| {
                ui.label("inside");
            });
        })
    };
    show(&mut c);
    show(&mut c);
    assert!(!press(&mut c, CTRL, KeyCode::KeyS), "Save is blocked");
    assert!(press(&mut c, CTRL, KeyCode::KeyO), "Open is allowed");
    let mut got = Vec::new();
    pass(&mut c, |ui| {
        got = [Cmd::Save, Cmd::Open]
            .into_iter()
            .filter(|cmd| ui.actions().triggered(*cmd))
            .collect();
        let mut open = true;
        zaxis::Modal::new("dialog").show(ui, &mut open, |ui| {
            ui.label("inside");
        });
    });
    assert!(
        got.contains(&Cmd::Open) && !got.contains(&Cmd::Save),
        "{got:?}"
    );
}
