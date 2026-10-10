use super::support::*;
use crate::prelude::*;
use zaxis::{Action, Actions, Chord, ChordError, ConflictKind, Kbd, Mods, Platform, Stroke};

fn chord(text: &str) -> Chord {
    text.parse().unwrap()
}

#[test]
fn chord_text_round_trips() {
    for text in [
        "mod+s",
        "ctrl+k ctrl+c",
        "ctrl+alt+shift+meta+f5",
        "shift+bracketleft",
        "escape",
        "ctrl+k ctrl+k ctrl+k ctrl+k",
        "numpad5",
    ] {
        let parsed = chord(text);
        assert_eq!(parsed.to_string(), text);
        assert_eq!(chord(&parsed.to_string()), parsed);
    }
    // Spellings that are read but never written.
    assert_eq!(chord("Ctrl+Esc"), chord("ctrl+escape"));
    assert_eq!(chord("cmd+DEL"), chord("meta+delete"));
}

#[test]
fn bad_chord_text_is_an_error_not_a_panic() {
    assert_eq!("".parse::<Chord>(), Err(ChordError::Empty));
    assert_eq!("   ".parse::<Chord>(), Err(ChordError::Empty));
    assert_eq!("ctrl+".parse::<Chord>(), Err(ChordError::Empty));
    assert_eq!(
        "hyper+s".parse::<Chord>(),
        Err(ChordError::UnknownModifier("hyper".into()))
    );
    assert_eq!(
        "ctrl+nokey".parse::<Chord>(),
        Err(ChordError::UnknownKey("nokey".into()))
    );
    assert_eq!("a b c d e".parse::<Chord>(), Err(ChordError::TooLong));
}

#[test]
fn the_shortcut_is_shown_for_the_platform() {
    let save: Chord = Mods::PRIMARY.key(KeyCode::KeyS).into();
    let shifted: Chord = (Mods::PRIMARY | Mods::SHIFT).key(KeyCode::KeyS).into();
    let show = |chord: &Chord, platform| Kbd::new(chord).platform(platform).to_string();
    assert_eq!(show(&save, Platform::Linux), "Ctrl+S");
    assert_eq!(show(&save, Platform::Windows), "Ctrl+S");
    assert_eq!(show(&save, Platform::Mac), "⌘S");
    assert_eq!(show(&shifted, Platform::Linux), "Ctrl+Shift+S");
    assert_eq!(show(&shifted, Platform::Mac), "⇧⌘S");
    let sequence = Mods::PRIMARY
        .key(KeyCode::KeyK)
        .then(Mods::PRIMARY.key(KeyCode::KeyC));
    assert_eq!(show(&sequence, Platform::Linux), "Ctrl+K Ctrl+C");
    assert_eq!(show(&sequence, Platform::Mac), "⌘K ⌘C");
    let keys: Chord = (Mods::ALT | Mods::META).key(KeyCode::ArrowLeft).into();
    assert_eq!(show(&keys, Platform::Linux), "Alt+Super+Left");
    assert_eq!(show(&keys, Platform::Windows), "Alt+Win+Left");
    assert_eq!(show(&keys, Platform::Mac), "⌥⌘←");
}

#[test]
fn the_primary_modifier_is_control_or_command_by_platform() {
    let mut c = setup();
    c.actions().keymap_mut().set_platform(Platform::Mac);
    idle(&mut c);
    assert!(
        !press(&mut c, CTRL, KeyCode::KeyS),
        "Ctrl is not Mod on a Mac"
    );
    assert!(press(
        &mut c,
        winit::keyboard::ModifiersState::SUPER,
        KeyCode::KeyS
    ));
    assert_eq!(taken(&mut c, &[Cmd::Save]), [Cmd::Save]);
}

#[test]
fn extra_modifiers_do_not_match() {
    let mut c = setup();
    idle(&mut c);
    let both = CTRL | winit::keyboard::ModifiersState::SHIFT;
    assert!(
        !press(&mut c, both, KeyCode::KeyS),
        "Ctrl+Shift+S is not Ctrl+S"
    );
    assert!(!press(
        &mut c,
        winit::keyboard::ModifiersState::empty(),
        KeyCode::KeyS
    ));
}

#[test]
fn several_bindings_can_run_one_action() {
    let mut c = Context::new();
    c.set_actions(
        Actions::new().register(
            Action::new(Cmd::Save, "Save")
                .shortcut(Mods::PRIMARY.key(KeyCode::KeyS))
                .shortcut(KeyCode::F2),
        ),
    );
    idle(&mut c);
    assert!(press(&mut c, CTRL, KeyCode::KeyS));
    assert!(press(
        &mut c,
        winit::keyboard::ModifiersState::empty(),
        KeyCode::F2
    ));
    assert_eq!(c.actions().keymap().bindings_of(Cmd::Save).len(), 2);
}

fn clashing() -> Actions {
    Actions::new()
        .register(Action::new("a", "A").shortcut(Mods::PRIMARY.key(KeyCode::KeyS)))
        .register(Action::new("b", "B").shortcut(Mods::PRIMARY.key(KeyCode::KeyS)))
        .register(Action::new("c", "C").shortcut_in("editor", Mods::PRIMARY.key(KeyCode::KeyS)))
        .register(Action::new("d", "D").shortcut(Mods::PRIMARY.key(KeyCode::KeyD)))
        .register(
            Action::new("e", "E").shortcut(
                Mods::PRIMARY
                    .key(KeyCode::KeyD)
                    .then(Mods::PRIMARY.key(KeyCode::KeyE)),
            ),
        )
}

#[test]
fn the_same_chord_in_one_context_is_a_conflict_and_across_contexts_is_not() {
    let found = clashing().keymap().conflicts();
    assert_eq!(found.len(), 2, "{found:?}");
    let same = found.iter().find(|c| c.kind == ConflictKind::Same).unwrap();
    assert_eq!((same.first, same.second), (Id::new("a"), Id::new("b")));
    assert_eq!(same.context, None);
    assert_eq!(same.chord, chord("mod+s"));
    let prefix = found
        .iter()
        .find(|c| c.kind == ConflictKind::Prefix)
        .unwrap();
    assert_eq!((prefix.first, prefix.second), (Id::new("d"), Id::new("e")));
}

#[test]
fn on_a_conflict_the_first_action_wins() {
    let mut c = Context::new();
    c.set_actions(clashing());
    idle(&mut c);
    let mut first = None;
    press(&mut c, CTRL, KeyCode::KeyS);
    pass(&mut c, |ui| {
        first = ["a", "b"].into_iter().find(|n| ui.actions().triggered(*n));
    });
    assert_eq!(first, Some("a"));
}

#[test]
fn rebinding_replaces_the_keys_and_reports_what_it_collides_with() {
    let mut actions = registry();
    let keymap = actions.keymap_mut();
    assert!(keymap.conflicts().is_empty());
    assert!(!keymap.is_overridden(Cmd::Save));
    let clash = keymap.rebind(Cmd::Save, Some(chord("mod+o")));
    assert_eq!(clash.len(), 1);
    assert_eq!(clash[0].kind, ConflictKind::Same);
    assert!(keymap.is_overridden(Cmd::Save));
    assert_eq!(keymap.bindings_of(Cmd::Save)[0].chord, chord("mod+o"));
    assert_eq!(keymap.conflicts().len(), 1);
    assert!(keymap
        .rebind(Cmd::Save, Some(chord("mod+shift+s")))
        .is_empty());
    assert!(keymap.conflicts().is_empty());
    keymap.rebind(Cmd::Save, None);
    assert!(keymap.bindings_of(Cmd::Save).is_empty(), "unbound");
    keymap.reset(Cmd::Save);
    assert_eq!(keymap.bindings_of(Cmd::Save)[0].chord, chord("mod+s"));
    assert!(!keymap.is_overridden(Cmd::Save));
}

#[test]
fn a_rebound_action_runs_from_its_new_keys_only() {
    let mut c = setup();
    c.actions()
        .keymap_mut()
        .rebind(Cmd::Save, Some(chord("mod+shift+s")));
    idle(&mut c);
    assert!(!press(&mut c, CTRL, KeyCode::KeyS));
    assert!(press(
        &mut c,
        CTRL | winit::keyboard::ModifiersState::SHIFT,
        KeyCode::KeyS
    ));
    assert_eq!(taken(&mut c, &[Cmd::Save]), [Cmd::Save]);
}

#[test]
fn rebinding_an_unknown_action_is_ignored() {
    let mut actions = registry();
    assert!(actions
        .keymap_mut()
        .rebind("nope", Some(chord("f1")))
        .is_empty());
    assert_eq!(actions.keymap().bindings().count(), 7);
}

#[test]
fn saved_overrides_load_back_the_same() {
    let mut actions = registry();
    let keymap = actions.keymap_mut();
    keymap.rebind(Cmd::Save, Some(chord("mod+shift+s")));
    keymap.rebind(Cmd::Sidebar, Some(chord("ctrl+k ctrl+s")));
    keymap.rebind(Cmd::Delete, None);
    keymap.rebind(Cmd::Find, Some(chord("f3")));
    let text = keymap.save();
    assert!(text.starts_with("# zaxis keymap 1\n"));
    assert!(text.contains("Save = mod+shift+s\n"), "{text}");
    assert!(text.contains("Find@editor = f3\n"), "{text}");
    assert!(text.contains("Delete =\n"), "{text}");
    assert!(!text.contains("Open"), "defaults are not written: {text}");

    let mut fresh = registry();
    let issues = fresh.keymap_mut().load(&text);
    assert!(issues.is_empty(), "{issues:?}");
    let a: Vec<_> = actions.keymap().bindings().cloned().collect();
    let b: Vec<_> = fresh.keymap().bindings().cloned().collect();
    assert_eq!(a, b);
    assert_eq!(
        fresh.keymap().save(),
        text,
        "saving again gives the same text"
    );
}

#[test]
fn loading_skips_bad_lines_and_applies_the_rest() {
    let mut actions = registry();
    let issues = actions
        .keymap_mut()
        .load("# comment\n\nSave = mod+nokey, f9\nGhost = f1\nnonsense\nOpen = mod+p # trailing\n");
    assert_eq!(issues.iter().map(|i| i.line).collect::<Vec<_>>(), [3, 4, 5]);
    let keymap = actions.keymap();
    assert_eq!(keymap.bindings_of(Cmd::Save)[0].chord, chord("f9"));
    assert_eq!(keymap.bindings_of(Cmd::Open)[0].chord, chord("mod+p"));
    // Load replaces earlier overrides.
    let mut again = actions.clone();
    again.keymap_mut().load("");
    assert_eq!(
        again.keymap().bindings_of(Cmd::Save)[0].chord,
        chord("mod+s")
    );
}

#[test]
fn strokes_keep_their_parts() {
    let stroke = Stroke::new(Mods::CTRL | Mods::SHIFT, KeyCode::KeyZ);
    assert_eq!(stroke.to_string(), "ctrl+shift+z");
    assert_eq!(stroke.code(), Some(KeyCode::KeyZ));
    assert!(!Stroke::new(Mods::CTRL, zaxis::KeyBinding::None).is_valid());
    assert!(Chord::new([Stroke::new(Mods::CTRL, zaxis::KeyBinding::None)]).is_empty());
}
