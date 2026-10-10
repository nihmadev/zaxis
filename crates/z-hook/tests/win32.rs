//! Window messages to input events, without a window.

use z_hook::win32::{
    cursor_resource, ime_composition, logical_key, scan_code_to_key, wm, Class, Translator,
};
use zaxis::{
    vec2,
    winit::{
        event::{ElementState, MouseButton},
        keyboard::{Key, KeyCode, ModifiersState, NamedKey, NativeKey, PhysicalKey},
        window::CursorIcon,
    },
    ImeEvent, InputEvent, WheelDelta,
};

fn key_lparam(scan: u32, extended: bool, repeat: bool, released: bool) -> isize {
    (1 | scan << 16
        | u32::from(extended) << 24
        | u32::from(repeat) << 30
        | u32::from(released) << 31) as i32 as isize
}

fn events(t: &mut Translator, msg: u32, wparam: usize, lparam: isize) -> Vec<InputEvent> {
    t.translate(msg, wparam, lparam).events
}

fn key_event(event: &InputEvent) -> &zaxis::KeyInput {
    match event {
        InputEvent::Key(key) => key,
        other => panic!("not a key: {other:?}"),
    }
}

#[test]
fn scan_codes_name_physical_keys_and_the_extended_bit_changes_them() {
    assert_eq!(scan_code_to_key(0x1E, false), Some(KeyCode::KeyA));
    assert_eq!(scan_code_to_key(0x1D, false), Some(KeyCode::ControlLeft));
    assert_eq!(scan_code_to_key(0x1D, true), Some(KeyCode::ControlRight));
    assert_eq!(scan_code_to_key(0x1C, true), Some(KeyCode::NumpadEnter));
    assert_eq!(scan_code_to_key(0x4B, false), Some(KeyCode::Numpad4));
    assert_eq!(scan_code_to_key(0x4B, true), Some(KeyCode::ArrowLeft));
    assert_eq!(scan_code_to_key(0x44, false), Some(KeyCode::F10));
    assert_eq!(scan_code_to_key(0x7F, false), None);
}

#[test]
fn a_key_press_carries_the_physical_and_logical_key_and_repeat() {
    let mut t = Translator::new();
    let down = events(
        &mut t,
        wm::KEYDOWN,
        0x41,
        key_lparam(0x1E, false, false, false),
    );
    let key = key_event(&down[0]);
    assert_eq!(key.physical, PhysicalKey::Code(KeyCode::KeyA));
    assert_eq!(key.logical, Key::Character("a".into()));
    assert_eq!((key.state, key.repeat), (ElementState::Pressed, false));
    let repeat = events(
        &mut t,
        wm::KEYDOWN,
        0x41,
        key_lparam(0x1E, false, true, false),
    );
    assert!(key_event(&repeat[0]).repeat);
    let up = events(&mut t, wm::KEYUP, 0x41, key_lparam(0x1E, false, true, true));
    assert_eq!(key_event(&up[0]).state, ElementState::Released);
    assert!(!key_event(&up[0]).repeat, "a release is never a repeat");
}

#[test]
fn numpad_with_numlock_off_is_a_numeric_scan_code_with_an_arrow_meaning() {
    let mut t = Translator::new();
    // NumPad4 with NumLock off: scan 0x4B not extended, virtual key VK_LEFT.
    let left = events(
        &mut t,
        wm::KEYDOWN,
        0x25,
        key_lparam(0x4B, false, false, false),
    );
    let key = key_event(&left[0]);
    assert_eq!(key.physical, PhysicalKey::Code(KeyCode::Numpad4));
    assert_eq!(key.logical, Key::Named(NamedKey::ArrowLeft));
}

#[test]
fn modifiers_are_announced_before_the_key_that_changes_them() {
    let mut t = Translator::new();
    let shift = events(
        &mut t,
        wm::KEYDOWN,
        0x10,
        key_lparam(0x2A, false, false, false),
    );
    assert_eq!(shift[0], InputEvent::Modifiers(ModifiersState::SHIFT));
    assert_eq!(
        key_event(&shift[1]).physical,
        PhysicalKey::Code(KeyCode::ShiftLeft)
    );
    let ctrl = events(
        &mut t,
        wm::SYSKEYDOWN,
        0x11,
        key_lparam(0x1D, true, false, false),
    );
    assert_eq!(
        ctrl[0],
        InputEvent::Modifiers(ModifiersState::SHIFT | ModifiersState::CONTROL)
    );
    let release = events(&mut t, wm::KEYUP, 0x10, key_lparam(0x2A, false, true, true));
    assert_eq!(release[0], InputEvent::Modifiers(ModifiersState::CONTROL));
    let focus = t.translate(wm::KILLFOCUS, 0, 0);
    assert_eq!(
        focus.events[0],
        InputEvent::Modifiers(ModifiersState::empty())
    );
    assert_eq!(focus.events[1], InputEvent::Focus(false));
    assert_eq!(t.modifiers(), ModifiersState::empty());
}

#[test]
fn characters_include_supplementary_planes_through_surrogate_pairs() {
    let mut t = Translator::new();
    let text = |t: &mut Translator, unit: u32| {
        t.translate(wm::CHAR, unit as usize, 0)
            .events
            .first()
            .map(|e| key_event(e).text.clone().unwrap().to_string())
    };
    assert_eq!(text(&mut t, 'é' as u32).as_deref(), Some("é"));
    // U+1F600 as UTF-16: D83D DE00.
    assert_eq!(text(&mut t, 0xD83D), None, "the first half waits");
    assert_eq!(text(&mut t, 0xDE00).as_deref(), Some("😀"));
    assert_eq!(
        text(&mut t, 0xDE00),
        None,
        "a lone low surrogate is dropped"
    );
    assert_eq!(text(&mut t, 0xD83D), None);
    assert_eq!(
        text(&mut t, 'a' as u32).as_deref(),
        Some("a"),
        "a new character discards the dangling half"
    );
}

#[test]
fn control_characters_are_keys_not_text() {
    let mut t = Translator::new();
    for unit in [0x08, 0x09, 0x0D, 0x1B, 0x7F] {
        assert!(
            t.translate(wm::CHAR, unit, 0).events.is_empty(),
            "{unit:#x}"
        );
    }
    assert_eq!(t.translate(wm::CHAR, 0x20, 0).class, Class::Text);
}

#[test]
fn unichar_carries_a_whole_code_point() {
    let mut t = Translator::new();
    let events = events(&mut t, wm::UNICHAR, 0x1F600, 0);
    assert_eq!(key_event(&events[0]).text.as_deref(), Some("😀"));
}

#[test]
fn pointer_messages_carry_signed_client_coordinates() {
    let mut t = Translator::new();
    let lparam = ((-5i16 as u16 as u32) << 16 | 300u32) as i32 as isize;
    let moved = t.translate(wm::MOUSEMOVE, 0, lparam);
    assert_eq!(moved.class, Class::Pointer);
    assert_eq!(
        moved.events,
        [InputEvent::PointerMoved { x: 300.0, y: -5.0 }]
    );
    let press = events(&mut t, wm::LBUTTONDOWN, 1, 0x0020_0040);
    assert_eq!(press[0], InputEvent::PointerMoved { x: 64.0, y: 32.0 });
    assert_eq!(
        press[1],
        InputEvent::Button {
            button: MouseButton::Left,
            state: ElementState::Pressed
        }
    );
    let extra = events(&mut t, wm::XBUTTONUP, 2 << 16, 0);
    assert_eq!(
        extra[1],
        InputEvent::Button {
            button: MouseButton::Forward,
            state: ElementState::Released
        }
    );
    assert_eq!(
        t.translate(wm::MOUSELEAVE, 0, 0).events,
        [InputEvent::PointerLeft]
    );
}

#[test]
fn the_wheel_reports_notches_in_both_directions() {
    let mut t = Translator::new();
    let up = events(&mut t, wm::MOUSEWHEEL, (120u32 << 16) as usize, 0);
    assert_eq!(up, [InputEvent::Wheel(WheelDelta::Lines(vec2(0.0, 1.0)))]);
    let down = events(&mut t, wm::MOUSEWHEEL, ((-240i16) as u16 as usize) << 16, 0);
    assert_eq!(
        down,
        [InputEvent::Wheel(WheelDelta::Lines(vec2(0.0, -2.0)))]
    );
    let right = events(&mut t, wm::MOUSEHWHEEL, (120u32 << 16) as usize, 0);
    assert_eq!(
        right,
        [InputEvent::Wheel(WheelDelta::Lines(vec2(1.0, 0.0)))]
    );
}

#[test]
fn dpi_changes_become_scale_factors() {
    let mut t = Translator::new();
    assert_eq!(
        events(&mut t, wm::DPICHANGED, 144, 0),
        [InputEvent::ScaleFactor(1.5)]
    );
}

#[test]
fn raw_input_and_cursor_messages_are_classified_for_the_router() {
    let mut t = Translator::new();
    assert_eq!(t.translate(wm::INPUT, 0, 0).class, Class::Raw);
    assert_eq!(t.translate(wm::SETCURSOR, 0, 0).class, Class::Cursor);
    assert_eq!(t.translate(0x0001, 0, 0).class, Class::Other);
}

#[test]
fn ime_results_commit_and_the_composition_follows() {
    let events = ime_composition(Some("日本"), Some(("かん", Some((0, 3)))));
    assert_eq!(
        events,
        [
            InputEvent::Ime(ImeEvent::Commit("日本".into())),
            InputEvent::Ime(ImeEvent::Preedit("かん".into(), Some((0, 3)))),
        ]
    );
    assert!(ime_composition(Some(""), None).is_empty());
    let mut t = Translator::new();
    assert_eq!(
        t.translate(wm::IME_ENDCOMPOSITION, 0, 0).events,
        [InputEvent::Ime(ImeEvent::Preedit(String::new(), None))]
    );
}

#[test]
fn unknown_virtual_keys_keep_their_native_code_and_cursors_map_to_stock_ones() {
    assert_eq!(
        logical_key(0xE7),
        Key::Unidentified(NativeKey::Windows(0xE7))
    );
    assert_eq!(logical_key(0x70), Key::Named(NamedKey::F1));
    assert_eq!(logical_key(0x87), Key::Named(NamedKey::F24));
    assert_eq!(cursor_resource(CursorIcon::Text), 32513);
    assert_eq!(cursor_resource(CursorIcon::Default), 32512);
    assert_eq!(cursor_resource(CursorIcon::EwResize), 32644);
}
