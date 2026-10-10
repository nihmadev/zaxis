use super::support::*;
use crate::prelude::*;
use zaxis::{Action, Actions, DiagnosticKind, Mods};

fn invalid(c: &Context) -> Vec<String> {
    c.diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::InvalidValue)
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn actions_are_found_by_the_value_they_were_declared_with() {
    let actions = registry();
    assert_eq!(actions.len(), 7);
    assert_eq!(actions.get(Cmd::Save).unwrap().title(), "Save");
    assert_eq!(actions.get(Cmd::Save).unwrap().group_name(), "File");
    assert!(actions.get("missing").is_none());
    let titles: Vec<&str> = actions.iter().map(|a| a.title()).collect();
    assert_eq!(titles[0], "Save", "declaration order");
}

#[test]
fn the_id_comes_from_the_value_not_from_the_title() {
    let a = Action::new("file.save", "Save");
    let b = Action::new("file.save", "Enregistrer");
    assert_eq!(a.id(), b.id());
    assert_ne!(a.id(), Action::new("file.open", "Save").id());
}

#[test]
fn a_duplicate_id_keeps_the_first_and_is_reported() {
    let mut c = Context::new();
    c.set_actions(
        Actions::new()
            .register(Action::new(Cmd::Save, "Save"))
            .register(Action::new(Cmd::Save, "Save again")),
    );
    idle(&mut c);
    assert_eq!(c.actions().registry().len(), 1);
    assert_eq!(
        c.actions().registry().get(Cmd::Save).unwrap().title(),
        "Save"
    );
    assert!(invalid(&c)
        .iter()
        .any(|m| m.contains("duplicate action id")));
}

#[test]
fn an_empty_title_falls_back_to_the_name_and_is_reported() {
    let mut c = Context::new();
    c.set_actions(Actions::new().register(Action::new("file.save", "  ")));
    idle(&mut c);
    assert_eq!(
        c.actions().registry().get("file.save").unwrap().title(),
        "file.save"
    );
    assert!(invalid(&c).iter().any(|m| m.contains("empty title")));
}

#[test]
fn an_empty_registry_is_accepted_and_reported() {
    let mut c = Context::new();
    c.set_actions(Actions::new());
    idle(&mut c);
    assert!(invalid(&c)
        .iter()
        .any(|m| m.contains("no actions registered")));
    assert!(!c.actions().trigger(Cmd::Save));
    assert!(!press(&mut c, CTRL, KeyCode::KeyS));
}

#[test]
fn an_empty_shortcut_is_ignored_and_reported() {
    let mut c = Context::new();
    let action = Action::new(Cmd::Save, "Save").shortcut(zaxis::KeyBinding::None);
    c.set_actions(Actions::new().register(action));
    idle(&mut c);
    assert!(c.actions().keymap().bindings().next().is_none());
    assert!(invalid(&c).iter().any(|m| m.contains("at least one key")));
}

#[test]
fn colliding_default_shortcuts_are_reported_when_registered() {
    let mut c = Context::new();
    c.set_actions(
        Actions::new()
            .register(Action::new(Cmd::Save, "Save").shortcut(Mods::PRIMARY.key(KeyCode::KeyS)))
            .register(Action::new(Cmd::Open, "Open").shortcut(Mods::PRIMARY.key(KeyCode::KeyS))),
    );
    idle(&mut c);
    assert!(invalid(&c).iter().any(|m| m.contains("collides")));
    assert_eq!(c.actions().keymap().conflicts().len(), 1);
}

#[test]
fn an_unknown_action_asked_for_by_a_widget_is_reported_as_misuse() {
    let mut c = setup();
    pass(&mut c, |ui| {
        ui.add(zaxis::Button::action("never-declared"));
    });
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidUsage));
}

#[test]
fn the_id_an_action_reports_names_it_as_well_as_its_value() {
    let mut c = setup();
    idle(&mut c);
    let id = c.action_registry().get(Cmd::Save).unwrap().id();
    assert_eq!(id, Id::new(Cmd::Save));
    assert!(c.action_registry().get(id).is_some());
    assert!(c.actions().trigger(id));
    pass(&mut c, |ui| {
        assert!(
            ui.actions().triggered(Cmd::Save),
            "one event, either spelling"
        );
        assert!(!ui.actions().triggered(id));
        ui.actions().set_enabled(id, false);
        assert!(!ui.actions().is_enabled(Cmd::Save));
    });
    let mut handle = c.actions();
    let keymap = handle.keymap_mut();
    assert_eq!(keymap.bindings_of(id).len(), 1);
    assert!(keymap.rebind(id, None).is_empty());
    assert!(keymap.bindings_of(Cmd::Save).is_empty());
    // A plain number is the id of that number.
    let mut numbered = Actions::new();
    numbered.insert(Action::new(7_u64, "Seven"));
    assert!(numbered.get(7_u64).is_some());
    assert_eq!(numbered.get(7_u64).unwrap().id().value(), 7);
}
