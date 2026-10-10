//! Order, repeats, releases and snapshots: the events of a gesture reach the owner as they
//! arrived, none is merged with another, and the shared `InputState` stays compatible.

use super::support::*;
use crate::prelude::*;
use winit::{
    event::ElementState,
    keyboard::{Key, ModifiersState, NamedKey},
};
use zaxis::{KeyInterest, Mods};

/// One pad named "p" that has focus, built each time with `interest`.
fn focused_pad(c: &mut Context, interest: KeyInterest<'_>, log: &mut Vec<KeyEvent>) -> Response {
    let mut out = None;
    pass(c, |ui| out = Some(pad(ui, "p", interest, log)));
    out.unwrap()
}

fn ready(interest: KeyInterest<'_>) -> (Context, Vec<KeyEvent>) {
    let mut c = setup();
    let mut log = Vec::new();
    let r = focused_pad(&mut c, interest, &mut log);
    c.request_focus(r.id);
    focused_pad(&mut c, interest, &mut log);
    (c, log)
}

#[test]
fn events_between_two_passes_arrive_in_the_order_they_came() {
    let (mut c, mut log) = ready(KeyInterest::arrows());
    for code in [
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
    ] {
        tap(&mut c, code);
    }
    focused_pad(&mut c, KeyInterest::arrows(), &mut log);
    assert_eq!(
        codes(&log),
        [
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowDown,
            KeyCode::ArrowLeft
        ]
    );
}

#[test]
fn identical_presses_are_not_merged() {
    let (mut c, mut log) = ready(KeyInterest::arrows());
    for _ in 0..5 {
        tap(&mut c, KeyCode::ArrowRight);
    }
    focused_pad(&mut c, KeyInterest::arrows(), &mut log);
    assert_eq!(log.len(), 5, "a set would have collapsed them to one");
    assert!(log.iter().all(KeyEvent::is_press));
}

#[test]
fn repeats_are_owned_but_delivered_only_when_asked() {
    let (mut c, mut log) = ready(KeyInterest::arrows());
    assert!(down(&mut c, KeyCode::ArrowRight));
    assert!(again(&mut c, KeyCode::ArrowRight));
    assert!(again(&mut c, KeyCode::ArrowRight));
    assert!(up(&mut c, KeyCode::ArrowRight));
    focused_pad(&mut c, KeyInterest::arrows(), &mut log);
    assert_eq!(
        log.len(),
        1,
        "the repeats were consumed without being delivered"
    );

    let (mut c, mut log) = ready(KeyInterest::arrows().repeats());
    down(&mut c, KeyCode::ArrowRight);
    again(&mut c, KeyCode::ArrowRight);
    again(&mut c, KeyCode::ArrowRight);
    up(&mut c, KeyCode::ArrowRight);
    focused_pad(&mut c, KeyInterest::arrows().repeats(), &mut log);
    let flags: Vec<_> = log.iter().map(|e| (e.is_press(), e.is_repeat())).collect();
    assert_eq!(flags, [(true, false), (false, true), (false, true)]);
}

#[test]
fn releases_are_delivered_in_order_with_the_presses_on_request() {
    let interest = KeyInterest::keys(&[KeyCode::KeyA, KeyCode::KeyB]).releases();
    let (mut c, mut log) = ready(interest);
    down(&mut c, KeyCode::KeyA);
    down(&mut c, KeyCode::KeyB);
    up(&mut c, KeyCode::KeyA);
    up(&mut c, KeyCode::KeyB);
    focused_pad(&mut c, interest, &mut log);
    let seq: Vec<_> = log.iter().map(|e| (e.code, e.is_release())).collect();
    assert_eq!(
        seq,
        [
            (KeyCode::KeyA, false),
            (KeyCode::KeyB, false),
            (KeyCode::KeyA, true),
            (KeyCode::KeyB, true)
        ]
    );
}

#[test]
fn a_claimed_key_is_one_event_not_a_second_click() {
    // Enter and Space click a focused region; claimed, they are the widget's keys only.
    let mut c = setup();
    let mut log = Vec::new();
    let clicks = std::cell::Cell::new(0);
    let build = |c: &mut Context, claim: bool, log: &mut Vec<KeyEvent>| {
        let mut out = None;
        pass(c, |ui| {
            let rect = ui.allocate_space(vec2(90.0, 30.0));
            let r = ui.interact(rect, "p", Sense::CLICK | Sense::FOCUS);
            if claim {
                log.extend(ui.keys(&r, KeyInterest::activation()));
            }
            clicks.set(clicks.get() + u32::from(r.clicked()));
            out = Some(r);
        });
        out.unwrap()
    };
    let r = build(&mut c, true, &mut log);
    c.request_focus(r.id);
    build(&mut c, true, &mut log);
    tap(&mut c, KeyCode::Enter);
    tap(&mut c, KeyCode::Space);
    build(&mut c, true, &mut log);
    build(&mut c, true, &mut log);
    assert_eq!(codes(&log), [KeyCode::Enter, KeyCode::Space]);
    assert_eq!(clicks.get(), 0, "no click on top of the events");
    build(&mut c, false, &mut log);
    tap(&mut c, KeyCode::Enter);
    build(&mut c, false, &mut log);
    build(&mut c, false, &mut log);
    assert_eq!(clicks.get(), 1, "unclaimed, Enter clicks as before");
}

#[test]
fn input_state_keeps_its_key_sets_for_a_claimed_key() {
    let (mut c, _log) = ready(KeyInterest::arrows());
    down(&mut c, KeyCode::ArrowRight);
    assert!(
        c.input().keys_down.contains(&KeyCode::ArrowRight),
        "held while down"
    );
    assert!(c.input().keys_pressed.contains(&KeyCode::ArrowRight));
    up(&mut c, KeyCode::ArrowRight);
    assert!(
        !c.input().keys_down.contains(&KeyCode::ArrowRight),
        "and not after the release"
    );
    assert!(c.input().keys_released.contains(&KeyCode::ArrowRight));
}

#[test]
fn the_modifiers_of_the_moment_travel_with_the_event() {
    let interest = KeyInterest::keys(&[KeyCode::KeyJ]).with_mods(Mods::SHIFT);
    let (mut c, mut log) = ready(interest);
    mods(&mut c, ModifiersState::SHIFT);
    down(&mut c, KeyCode::KeyJ);
    up(&mut c, KeyCode::KeyJ);
    mods(&mut c, ModifiersState::empty());
    focused_pad(&mut c, interest, &mut log);
    assert_eq!(log.len(), 1);
    assert_eq!(
        log[0].mods(),
        Mods::SHIFT,
        "Shift was let go before the pass"
    );
    assert!(log[0].modifiers.shift_key());
    assert!(
        !tap(&mut c, KeyCode::KeyJ),
        "without Shift the key is not claimed"
    );
}

#[test]
fn exact_modifiers_never_match_extra_ones_so_altgr_and_shortcuts_are_left_alone() {
    let (mut c, mut log) = ready(KeyInterest::keys(&[KeyCode::KeyQ]));
    mods(&mut c, ModifiersState::CONTROL | ModifiersState::ALT);
    assert!(
        !tap(&mut c, KeyCode::KeyQ),
        "AltGr+Q (Ctrl+Alt on Windows) is text, not the Q claim"
    );
    mods(&mut c, ModifiersState::empty());
    assert!(tap(&mut c, KeyCode::KeyQ));
    focused_pad(&mut c, KeyInterest::keys(&[KeyCode::KeyQ]), &mut log);
    assert_eq!(log.len(), 1);
}

#[test]
fn any_modifiers_matches_them_all() {
    let interest = KeyInterest::arrows().any_mods();
    let (mut c, mut log) = ready(interest);
    for state in [
        ModifiersState::empty(),
        ModifiersState::SHIFT,
        ModifiersState::CONTROL,
    ] {
        mods(&mut c, state);
        tap(&mut c, KeyCode::ArrowUp);
    }
    mods(&mut c, ModifiersState::empty());
    focused_pad(&mut c, interest, &mut log);
    assert_eq!(log.len(), 3);
}

#[test]
fn letters_follow_the_printed_letter_on_other_layouts_unless_matched_by_position() {
    let z = KeyInterest::keys(&[KeyCode::KeyZ]).with_mods(Mods::CTRL);
    let (mut c, mut log) = ready(z);
    mods(&mut c, ModifiersState::CONTROL);
    // Russian layout: the key that carries "я" sits where Z sits on a US keyboard.
    let russian = |state| {
        key_as(
            KeyCode::KeyZ,
            Key::Character("я".into()),
            state,
            false,
            None,
        )
    };
    assert!(
        c.on_input(russian(ElementState::Pressed)).consumed,
        "falls back to position"
    );
    c.on_input(russian(ElementState::Released));
    // AZERTY: the physical Q position prints "a".
    let azerty_a = |state| {
        key_as(
            KeyCode::KeyQ,
            Key::Character("a".into()),
            state,
            false,
            None,
        )
    };
    assert!(
        !c.on_input(azerty_a(ElementState::Pressed)).consumed,
        "that is A, not Z"
    );
    c.on_input(azerty_a(ElementState::Released));
    focused_pad(&mut c, z, &mut log);
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].layout, KeyCode::KeyZ);
    assert_eq!(log[0].logical, Key::Character("я".into()));

    let wasd = KeyInterest::keys(&[KeyCode::KeyQ]).by_position();
    let (mut c, mut log) = ready(wasd);
    assert!(
        c.on_input(azerty_a(ElementState::Pressed)).consumed,
        "by position it is Q"
    );
    c.on_input(azerty_a(ElementState::Released));
    focused_pad(&mut c, wasd, &mut log);
    assert_eq!(log[0].code, KeyCode::KeyQ);
}

#[test]
fn a_claimed_printable_key_types_no_text() {
    let (mut c, _log) = ready(KeyInterest::keys(&[KeyCode::KeyA]));
    let typed = key_as(
        KeyCode::KeyA,
        Key::Character("a".into()),
        ElementState::Pressed,
        false,
        Some("a"),
    );
    assert!(c.on_input(typed).consumed);
    assert!(c.input().text.is_empty(), "a command key is not text");
    let other = key_as(
        KeyCode::KeyB,
        Key::Character("b".into()),
        ElementState::Pressed,
        false,
        Some("b"),
    );
    c.on_input(other);
    assert_eq!(c.input().text, "b", "an unclaimed key still types");
    let _ = NamedKey::Enter;
}
