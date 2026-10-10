//! Who wins when several want a key: the controls that own keys outright, a widget's
//! claim, the keymap, then the rest. Claims never get around a modal, a popup, a chord in
//! progress or a key being captured, and never take keys from text editing.

use super::support::*;
use crate::prelude::*;
use winit::{
    event::ElementState,
    keyboard::{Key, ModifiersState},
};
use zaxis::{
    Action, Actions, Button, DiagnosticKind, KeyBox, KeyInterest, Modal, Mods, Platform, Popup,
    Slider, TextEdit,
};

#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug)]
enum Cmd {
    Save,
    Open,
    Chord,
}

fn with_actions() -> Context {
    let mut c = setup();
    let mut actions = Actions::new()
        .register(Action::new(Cmd::Save, "Save").shortcut(Mods::PRIMARY.key(KeyCode::KeyS)))
        .register(Action::new(Cmd::Open, "Open").shortcut(Mods::PRIMARY.key(KeyCode::KeyO)))
        .register(
            Action::new(Cmd::Chord, "Chord").shortcut(
                Mods::PRIMARY
                    .key(KeyCode::KeyK)
                    .then(Mods::PRIMARY.key(KeyCode::KeyB)),
            ),
        );
    actions.keymap_mut().set_platform(Platform::Linux);
    c.set_actions(actions);
    c
}

fn taken(c: &mut Context) -> Vec<Cmd> {
    let mut seen = Vec::new();
    pass(c, |ui| {
        for cmd in [Cmd::Save, Cmd::Open, Cmd::Chord] {
            if ui.actions().triggered(cmd) {
                seen.push(cmd);
            }
        }
    });
    seen
}

fn ctrl(c: &mut Context, code: KeyCode) -> bool {
    mods(c, ModifiersState::CONTROL);
    let consumed = tap(c, code);
    mods(c, ModifiersState::empty());
    consumed
}

/// A claiming pad that has focus.
fn focused(c: &mut Context, interest: KeyInterest<'_>, log: &mut Vec<KeyEvent>) -> Id {
    let mut id = None;
    pass(c, |ui| id = Some(pad(ui, "p", interest, log).id));
    c.request_focus(id.unwrap());
    pass(c, |ui| {
        pad(ui, "p", interest, log);
    });
    id.unwrap()
}

#[test]
fn an_explicit_claim_beats_the_action_on_the_same_key_and_the_rest_still_run() {
    let interest = KeyInterest::keys(&[KeyCode::KeyS]).with_mods(Mods::PRIMARY);
    let mut c = with_actions();
    let mut log = Vec::new();
    focused(&mut c, interest, &mut log);
    assert!(ctrl(&mut c, KeyCode::KeyS));
    pass(&mut c, |ui| {
        pad(ui, "p", interest, &mut log);
        assert!(
            !ui.actions().triggered(Cmd::Save),
            "Save did not run: the widget owns Ctrl+S"
        );
    });
    assert_eq!(log.len(), 1);
    assert!(ctrl(&mut c, KeyCode::KeyO));
    assert_eq!(
        taken(&mut c),
        [Cmd::Open],
        "a key nobody claimed still runs its action"
    );
}

#[test]
fn the_action_runs_when_the_widget_is_not_focused() {
    let interest = KeyInterest::keys(&[KeyCode::KeyS]).with_mods(Mods::PRIMARY);
    let mut c = with_actions();
    let mut log = Vec::new();
    pass(&mut c, |ui| {
        pad(ui, "p", interest, &mut log);
    });
    assert!(ctrl(&mut c, KeyCode::KeyS));
    assert_eq!(taken(&mut c), [Cmd::Save]);
}

#[test]
fn a_chord_in_progress_keeps_its_next_key_and_a_claim_on_its_first_stroke_wins() {
    let second = KeyInterest::keys(&[KeyCode::KeyB]).with_mods(Mods::PRIMARY);
    let mut c = with_actions();
    let mut log = Vec::new();
    focused(&mut c, second, &mut log);
    assert!(
        ctrl(&mut c, KeyCode::KeyK),
        "the chord starts: K is nobody's claim"
    );
    assert!(ctrl(&mut c, KeyCode::KeyB));
    assert_eq!(
        taken(&mut c),
        [Cmd::Chord],
        "the second stroke went to the chord, not the claim"
    );
    pass(&mut c, |ui| {
        pad(ui, "p", second, &mut log);
    });
    assert!(log.is_empty());

    let first = KeyInterest::keys(&[KeyCode::KeyK]).with_mods(Mods::PRIMARY);
    let mut c = with_actions();
    focused(&mut c, first, &mut log);
    assert!(ctrl(&mut c, KeyCode::KeyK));
    assert!(
        !ctrl(&mut c, KeyCode::KeyB),
        "no chord was begun, so B is not its second stroke"
    );
    assert_eq!(taken(&mut c), []);
}

#[test]
fn a_key_box_that_listens_gets_the_key_before_any_claim() {
    let interest = KeyInterest::keys(&[KeyCode::KeyS]).with_mods(Mods::PRIMARY);
    let mut c = with_actions();
    let mut binding = zaxis::KeyBinding::None;
    let mut log = Vec::new();
    let mut spot = vec2(0.0, 0.0);
    let build = |c: &mut Context,
                 binding: &mut zaxis::KeyBinding,
                 log: &mut Vec<KeyEvent>,
                 spot: &mut Vec2| {
        pass(c, |ui| {
            pad(ui, "p", interest, log);
            *spot = ui.add(KeyBox::new(binding, "key")).rect.center();
        });
    };
    build(&mut c, &mut binding, &mut log, &mut spot);
    click(&mut c, spot);
    build(&mut c, &mut binding, &mut log, &mut spot);
    assert!(c.key_capture_active());
    let pad_id = id_of_pad(&mut c, interest);
    c.request_focus(pad_id);
    build(&mut c, &mut binding, &mut log, &mut spot);
    assert!(c.key_capture_active());
    ctrl(&mut c, KeyCode::KeyS);
    build(&mut c, &mut binding, &mut log, &mut spot);
    assert_eq!(
        binding,
        zaxis::KeyBinding::Key(KeyCode::KeyS),
        "recorded by the box"
    );
    assert!(log.is_empty(), "the claim did not take it");
}

fn id_of_pad(c: &mut Context, interest: KeyInterest<'_>) -> Id {
    let mut id = None;
    let mut sink = Vec::new();
    pass(c, |ui| id = Some(pad(ui, "p", interest, &mut sink).id));
    id.unwrap()
}

#[test]
fn a_text_field_keeps_its_keys_and_a_claim_on_it_is_reported_and_ignored() {
    let mut c = with_actions();
    let mut text = String::from("ab");
    let mut got = Vec::new();
    let build = |c: &mut Context, text: &mut String, got: &mut Vec<KeyEvent>| {
        let mut id = None;
        pass(c, |ui| {
            let r = ui.add(TextEdit::new(text).id_source("t"));
            got.extend(ui.keys(&r, KeyInterest::keys(&[KeyCode::ArrowLeft, KeyCode::KeyA])));
            id = Some(r.id);
        });
        id.unwrap()
    };
    let id = build(&mut c, &mut text, &mut got);
    c.request_focus(id);
    build(&mut c, &mut text, &mut got);
    assert!(
        c.diagnostics()
            .iter()
            .any(|d| d.kind == DiagnosticKind::InvalidUsage && d.id == Some(id)),
        "{:?}",
        c.diagnostics()
    );
    tap(&mut c, KeyCode::ArrowLeft);
    let typed = key_as(
        KeyCode::KeyA,
        Key::Character("a".into()),
        ElementState::Pressed,
        false,
        Some("a"),
    );
    c.on_input(typed);
    build(&mut c, &mut text, &mut got);
    assert!(got.is_empty(), "the field's own keys are not claimable");
    assert_eq!(
        text, "aab",
        "the caret moved left and the typed 'a' went before 'b'"
    );
}

#[test]
fn altgr_text_and_the_select_all_shortcut_of_a_text_field_are_untouched() {
    let mut c = with_actions();
    let mut text = String::new();
    let mut got = Vec::new();
    let mut id = None;
    let interest = KeyInterest::keys(&[KeyCode::KeyQ, KeyCode::KeyV, KeyCode::KeyA]).any_mods();
    let build =
        |c: &mut Context, text: &mut String, got: &mut Vec<KeyEvent>, id: &mut Option<Id>| {
            pass(c, |ui| {
                let r = ui.add(TextEdit::new(text).id_source("t"));
                got.extend(ui.keys(&r, interest));
                *id = Some(r.id);
            });
        };
    build(&mut c, &mut text, &mut got, &mut id);
    c.request_focus(id.unwrap());
    build(&mut c, &mut text, &mut got, &mut id);
    mods(&mut c, ModifiersState::ALT);
    let altgr = key_as(
        KeyCode::KeyQ,
        Key::Character("@".into()),
        ElementState::Pressed,
        false,
        Some("@"),
    );
    c.on_input(altgr);
    mods(&mut c, ModifiersState::empty());
    build(&mut c, &mut text, &mut got, &mut id);
    assert_eq!(text, "@", "AltGr text still types");
    assert!(
        ctrl(&mut c, KeyCode::KeyA),
        "select all belongs to the field"
    );
    assert!(got.is_empty());
}

#[test]
fn a_slider_keeps_its_adjusting_keys_and_a_claim_on_it_changes_nothing() {
    let mut c = setup();
    let mut value = 0.5_f32;
    let mut got = Vec::new();
    let build = |c: &mut Context, value: &mut f32, got: &mut Vec<KeyEvent>| {
        let mut id = None;
        pass(c, |ui| {
            let r = ui.add(Slider::new(value, 0.0..=1.0).step(0.1));
            got.extend(ui.keys(&r, KeyInterest::arrows()));
            id = Some(r.id);
        });
        id.unwrap()
    };
    let id = build(&mut c, &mut value, &mut got);
    c.request_focus(id);
    build(&mut c, &mut value, &mut got);
    tap(&mut c, KeyCode::ArrowRight);
    build(&mut c, &mut value, &mut got);
    assert!(value > 0.5 && got.is_empty());
}

#[test]
fn a_widget_below_an_open_popup_gets_nothing() {
    let interest = KeyInterest::arrows();
    let mut c = setup();
    let mut log = Vec::new();
    let mut open = false;
    let shown = std::cell::Cell::new(false);
    let build = |c: &mut Context, open: &mut bool, log: &mut Vec<KeyEvent>| {
        let mut id = None;
        shown.set(false);
        c.run(|c| {
            zaxis::Window::new("w").show(c, |ui| {
                let r = pad(ui, "p", interest, log);
                let anchor = ui.button("anchor").rect;
                Popup::new("pop", anchor).show(ui, open, |ui| {
                    shown.set(true);
                    ui.add(Button::new("inside"));
                });
                id = Some(r.id);
            });
        });
        id.unwrap()
    };
    let id = build(&mut c, &mut open, &mut log);
    c.request_focus(id);
    build(&mut c, &mut open, &mut log);
    assert!(
        tap(&mut c, KeyCode::ArrowRight),
        "with no popup the claim works"
    );
    open = true;
    build(&mut c, &mut open, &mut log);
    build(&mut c, &mut open, &mut log);
    assert!(shown.get(), "the popup is open");
    assert!(
        !tap(&mut c, KeyCode::ArrowRight),
        "the popup is in front: the widget under it gets no key"
    );
    open = false;
    build(&mut c, &mut open, &mut log);
}

#[test]
fn a_widget_outside_an_open_modal_gets_nothing_and_one_inside_does() {
    let interest = KeyInterest::arrows();
    let mut c = setup();
    let (mut under, mut inside) = (Vec::new(), Vec::new());
    let mut open = false;
    let build = |c: &mut Context,
                 open: &mut bool,
                 under: &mut Vec<KeyEvent>,
                 inside: &mut Vec<KeyEvent>| {
        let mut ids = (None, None);
        pass(c, |ui| {
            ids.0 = Some(pad(ui, "under", interest, under).id);
            Modal::new("m").show(ui, open, |ui| {
                ids.1 = Some(pad(ui, "inside", interest, inside).id);
            });
        });
        ids
    };
    let (under_id, _) = build(&mut c, &mut open, &mut under, &mut inside);
    c.request_focus(under_id.unwrap());
    build(&mut c, &mut open, &mut under, &mut inside);
    open = true;
    for _ in 0..4 {
        build(&mut c, &mut open, &mut under, &mut inside);
    }
    c.request_focus(under_id.unwrap());
    tap(&mut c, KeyCode::ArrowLeft);
    build(&mut c, &mut open, &mut under, &mut inside);
    assert!(under.is_empty(), "the modal is the only target");
    let (_, inside_id) = build(&mut c, &mut open, &mut under, &mut inside);
    c.request_focus(inside_id.unwrap());
    build(&mut c, &mut open, &mut under, &mut inside);
    assert!(tap(&mut c, KeyCode::ArrowLeft));
    build(&mut c, &mut open, &mut under, &mut inside);
    assert_eq!(inside.len(), 1);
}

#[test]
fn a_claim_on_a_region_that_cannot_take_focus_is_reported_once_per_cause() {
    let mut c = setup();
    let mut log = Vec::new();
    pass(&mut c, |ui| {
        let rect = ui.allocate_space(vec2(80.0, 20.0));
        let r = ui.interact(rect, "no-focus", Sense::CLICK);
        log.extend(ui.keys(&r, KeyInterest::arrows()));
    });
    pass(&mut c, |ui| {
        let rect = ui.allocate_space(vec2(80.0, 20.0));
        let r = ui.interact(rect, "no-focus", Sense::CLICK);
        log.extend(ui.keys(&r, KeyInterest::arrows()));
    });
    let n = c
        .diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::InvalidUsage)
        .count();
    assert_eq!(n, 1, "{:?}", c.diagnostics());
}

#[test]
fn ime_composition_in_a_text_field_is_left_to_the_field_while_a_claim_waits_next_to_it() {
    use zaxis::ImeEvent;
    let mut c = setup();
    let mut text = String::new();
    let mut got = Vec::new();
    let build = |c: &mut Context, text: &mut String, got: &mut Vec<KeyEvent>| {
        let mut ids = (None, None);
        pass(c, |ui| {
            let field = ui.add(TextEdit::new(text).id_source("t"));
            let other = pad(ui, "p", KeyInterest::arrows(), got);
            ids = (Some(field.id), Some(other.id));
        });
        ids
    };
    let (field, _) = build(&mut c, &mut text, &mut got);
    c.request_focus(field.unwrap());
    build(&mut c, &mut text, &mut got);
    assert!(
        c.on_input(InputEvent::Ime(ImeEvent::Preedit("ni".into(), None)))
            .consumed
    );
    assert!(
        tap(&mut c, KeyCode::ArrowLeft),
        "the composing field keeps the arrow"
    );
    c.on_input(InputEvent::Ime(ImeEvent::Commit("你".into())));
    build(&mut c, &mut text, &mut got);
    assert_eq!(text, "你", "the committed text went to the field");
    assert!(
        got.is_empty(),
        "nothing was addressed to the claiming neighbour"
    );
}
