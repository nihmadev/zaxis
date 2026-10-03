use super::*;
use crate::{vec2, DragValue, NumberInput, Numeric, Response, Window};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Ime, WindowEvent},
    keyboard::{KeyCode, ModifiersState},
};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(700, 500), 1.0);
    c
}
fn draw<T: Numeric>(
    c: &mut Context,
    value: &mut T,
    step: T,
    drag: bool,
    enabled: bool,
) -> Response {
    let mut result = None;
    c.run(|c| {
        Window::new("Numbers").show(c, |ui| {
            result = Some(if drag {
                ui.add(
                    DragValue::new(value)
                        .id_source("value")
                        .step(step)
                        .enabled(enabled)
                        .sensitivity(1.0),
                )
            } else {
                ui.add(
                    NumberInput::new(value)
                        .id_source("value")
                        .step(step)
                        .enabled(enabled),
                )
            });
            ui.button("Next");
        });
    });
    result.unwrap()
}
fn key(c: &mut Context, key: KeyCode, mods: ModifiersState) {
    c.input.modifiers = mods;
    c.on_key_event(key, ElementState::Pressed, false);
    c.on_key_event(key, ElementState::Released, false);
}
fn replace(c: &mut Context, text: &str) {
    key(c, KeyCode::KeyA, ModifiersState::CONTROL);
    c.on_text_event(text);
}
fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}

#[test]
fn number_draft_enter_escape_focus_loss_and_invalid_are_ordered() {
    let mut c = setup();
    let mut v = -3.5_f64;
    let r = draw(&mut c, &mut v, 0.1, false, true);
    click(&mut c, r.rect.center());
    replace(&mut c, "-");
    assert!(!draw(&mut c, &mut v, 0.1, false, true).changed());
    assert_eq!(v, -3.5);
    c.on_text_event(".");
    draw(&mut c, &mut v, 0.1, false, true);
    assert_eq!(v, -3.5);
    c.on_text_event("25");
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    let r = draw(&mut c, &mut v, 0.1, false, true);
    assert!(r.changed() && r.submitted());
    assert_eq!(v, -0.25);
    replace(&mut c, "42");
    key(&mut c, KeyCode::Escape, ModifiersState::empty());
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    assert!(!draw(&mut c, &mut v, 0.1, false, true).changed());
    assert_eq!(v, -0.25);
    click(&mut c, r.rect.center());
    replace(&mut c, "-12.75");
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    assert!(draw(&mut c, &mut v, 0.1, false, true).changed());
    assert_eq!(v, -12.75);
    for text in ["no", "-", ".", "NaN", "inf", "-inf", "1e999"] {
        click(&mut c, r.rect.center());
        replace(&mut c, text);
        key(&mut c, KeyCode::Enter, ModifiersState::empty());
        let next = draw(&mut c, &mut v, 0.1, false, true);
        assert!(!next.changed() && !next.submitted(), "{text}");
        c.on_window_event(&WindowEvent::Focused(false));
        assert!(!draw(&mut c, &mut v, 0.1, false, true).changed());
        assert_eq!(v, -12.75);
    }
}

#[test]
fn fractional_steps_modifiers_and_escape_restore_session() {
    let mut c = setup();
    let mut v = -1.0_f64;
    let r = draw(&mut c, &mut v, 0.1, false, true);
    c.request_focus(r.id);
    draw(&mut c, &mut v, 0.1, false, true);
    for _ in 0..20 {
        key(&mut c, KeyCode::ArrowUp, ModifiersState::empty());
        draw(&mut c, &mut v, 0.1, false, true);
    }
    assert_eq!(v, 1.0);
    key(&mut c, KeyCode::Escape, ModifiersState::empty());
    assert!(draw(&mut c, &mut v, 0.1, false, true).changed());
    assert_eq!(v, -1.0);
    key(&mut c, KeyCode::ArrowUp, ModifiersState::SHIFT);
    draw(&mut c, &mut v, 0.1, false, true);
    assert!((v + 0.99).abs() < 1e-14);
    key(&mut c, KeyCode::ArrowUp, ModifiersState::CONTROL);
    draw(&mut c, &mut v, 0.1, false, true);
    assert!((v - 0.01).abs() < 1e-14);
    replace(&mut c, "bad");
    key(&mut c, KeyCode::ArrowDown, ModifiersState::empty());
    assert!(!draw(&mut c, &mut v, 0.1, false, true).changed());
}

#[test]
fn large_integers_remain_exact_and_overflow_is_rejected() {
    let mut c = setup();
    let mut v = 9_007_199_254_740_993_u64;
    let r = draw(&mut c, &mut v, 1, false, true);
    c.request_focus(r.id);
    key(&mut c, KeyCode::ArrowUp, ModifiersState::empty());
    draw(&mut c, &mut v, 1, false, true);
    assert_eq!(v, 9_007_199_254_740_994);
    replace(&mut c, &u64::MAX.to_string());
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    draw(&mut c, &mut v, 1, false, true);
    assert_eq!(v, u64::MAX);
    key(&mut c, KeyCode::ArrowUp, ModifiersState::CONTROL);
    assert!(!draw(&mut c, &mut v, 1, false, true).changed());
    replace(&mut c, "18446744073709551616");
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    assert!(!draw(&mut c, &mut v, 1, false, true).changed());
    assert_eq!(v, u64::MAX);
    assert_eq!(
        i128::MIN.offset(i128::MAX, 2.0, i128::MIN, i128::MAX),
        i128::MAX - 1
    );
    assert_eq!(i128::MAX.offset(1, 1.0, i128::MIN, i128::MAX), i128::MAX);
    assert_eq!(u128::MAX.offset(1, -1.0, 0, u128::MAX), u128::MAX - 1);
    assert_eq!((-5_i8).offset(2, -100.0, i8::MIN, i8::MAX), i8::MIN);
}

#[test]
fn drag_capture_click_to_edit_keyboard_and_disabled() {
    let mut c = setup();
    let mut v = 10_i64;
    let r = draw(&mut c, &mut v, 1, true, true);
    c.move_pointer(r.rect.center());
    c.primary_button(ElementState::Pressed);
    c.move_pointer(r.rect.center() + vec2(103.0, 40.0));
    c.primary_button(ElementState::Released);
    assert!(draw(&mut c, &mut v, 1, true, true).changed());
    assert_eq!(v, 110);
    assert_eq!(
        c.previous_hits
            .iter()
            .find(|h| h.id == r.id)
            .unwrap()
            .action,
        HitAction::DragValue
    );
    click(&mut c, r.rect.center());
    draw(&mut c, &mut v, 1, true, true);
    assert!(c.text_edits.contains_key(&r.id));
    c.on_text_event("-7"); // Click-to-edit selects the exact value.
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    assert!(draw(&mut c, &mut v, 1, true, true).changed());
    assert_eq!(v, -7);
    draw(&mut c, &mut v, 1, true, true);
    key(&mut c, KeyCode::ArrowRight, ModifiersState::CONTROL);
    draw(&mut c, &mut v, 1, true, true);
    assert_eq!(v, 3);
    key(&mut c, KeyCode::F2, ModifiersState::empty());
    draw(&mut c, &mut v, 1, true, true);
    replace(&mut c, "999");
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    assert!(!draw(&mut c, &mut v, 1, true, false).changed());
    assert_eq!(v, 3);
    assert!(c.focused_widget.is_none());
    assert!(!draw(&mut c, &mut v, 1, true, true).changed());
}

#[test]
fn ranges_nan_precision_formatter_and_ime_preserve_actual_value() {
    let mut c = setup();
    let mut v = f64::NAN;
    let mut response = None;
    let draw = |c: &mut Context, v: &mut f64, response: &mut Option<Response>| {
        c.run(|c| {
            Window::new("Range").show(c, |ui| {
                *response = Some(
                    ui.add(
                        NumberInput::new(v)
                            .range(-10.0..=10.0)
                            .step(0.25)
                            .precision(1)
                            .formatter(|v| format!("{v:.1}"))
                            .prefix("$ ")
                            .suffix(" kg"),
                    ),
                );
            });
        });
    };
    draw(&mut c, &mut v, &mut response);
    assert!(!response.unwrap().changed());
    c.request_focus(response.unwrap().id);
    replace(&mut c, "100");
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    draw(&mut c, &mut v, &mut response);
    assert_eq!(v, 10.0);
    replace(&mut c, "-3.125");
    c.on_window_event(&WindowEvent::Ime(Ime::Preedit("6".into(), Some((1, 1)))));
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    draw(&mut c, &mut v, &mut response);
    assert_eq!(v, 10.0); // Enter must not submit unfinished composition.
    c.on_window_event(&WindowEvent::Ime(Ime::Disabled));
    replace(&mut c, "-3.125");
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    draw(&mut c, &mut v, &mut response);
    assert_eq!(v, -3.125); // Display precision and step do not round typed values.
    assert_eq!(f64::INFINITY.offset(1.0, -1.0, -10.0, 10.0), 9.0);
    assert_eq!(f64::NEG_INFINITY.offset(1.0, 1.0, -10.0, 10.0), -9.0);
}

#[test]
fn disabled_number_discards_draft_external_updates_reset_and_ime_blur_commits() {
    let mut c = setup();
    let mut v = -8.0_f64;
    let r = draw(&mut c, &mut v, 0.1, false, true);
    c.request_focus(r.id);
    replace(&mut c, "12");
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    assert!(!draw(&mut c, &mut v, 0.1, false, false).changed());
    assert_eq!(v, -8.0);
    draw(&mut c, &mut v, 0.1, false, true);
    c.request_focus(r.id);
    replace(&mut c, "99");
    draw(&mut c, &mut v, 0.1, false, true);
    v = 3.0;
    assert!(!draw(&mut c, &mut v, 0.1, false, true).changed());
    key(&mut c, KeyCode::Enter, ModifiersState::empty());
    draw(&mut c, &mut v, 0.1, false, true);
    assert_eq!(v, 3.0);
    replace(&mut c, "4.5");
    c.on_window_event(&WindowEvent::Ime(Ime::Preedit("9".into(), Some((1, 1)))));
    c.on_window_event(&WindowEvent::Focused(false));
    assert!(draw(&mut c, &mut v, 0.1, false, true).changed());
    assert_eq!(v, 4.5); // Preedit is discarded; the last committed draft is confirmed.
}

#[test]
fn fractional_drag_modifiers_reversal_cancel_and_idle_cache() {
    let mut c = setup();
    let mut v = -1.0_f64;
    let r = draw(&mut c, &mut v, 0.1, true, true);
    let revision = c.draw_data().revision;
    assert!(!draw(&mut c, &mut v, 0.1, true, true).changed());
    assert_eq!(revision, c.draw_data().revision);
    c.move_pointer(r.rect.center());
    c.primary_button(ElementState::Pressed);
    c.move_pointer(r.rect.center() + vec2(13.0, 90.0));
    draw(&mut c, &mut v, 0.1, true, true);
    assert_eq!(v, 0.0);
    c.input.modifiers = ModifiersState::SHIFT;
    c.move_pointer(r.rect.center() + vec2(23.0, 90.0));
    draw(&mut c, &mut v, 0.1, true, true);
    assert_eq!(v, 0.1);
    c.input.modifiers = ModifiersState::CONTROL;
    c.move_pointer(r.rect.center() + vec2(24.0, 90.0));
    draw(&mut c, &mut v, 0.1, true, true);
    assert_eq!(v, 1.1);
    key(&mut c, KeyCode::Escape, ModifiersState::empty());
    assert!(draw(&mut c, &mut v, 0.1, true, true).changed());
    assert_eq!(v, -1.0);
    c.primary_button(ElementState::Released);
    assert!(!draw(&mut c, &mut v, 0.1, true, true).changed());
    assert_eq!(
        c.previous_hits
            .iter()
            .find(|h| h.id == r.id)
            .unwrap()
            .action,
        HitAction::DragValue
    );
}

#[test]
fn precise_integer_steps_at_limits_and_small_float_steps_are_not_lost() {
    let mut c = setup();
    let mut v = 0_u8;
    let r = draw(&mut c, &mut v, 1, true, true);
    c.request_focus(r.id);
    for _ in 0..10 {
        key(&mut c, KeyCode::ArrowUp, ModifiersState::SHIFT);
        draw(&mut c, &mut v, 1, true, true);
    }
    assert_eq!(v, 1);
    v = u8::MAX;
    draw(&mut c, &mut v, 1, true, true);
    for _ in 0..10 {
        key(&mut c, KeyCode::ArrowDown, ModifiersState::SHIFT);
        draw(&mut c, &mut v, 1, true, true);
    }
    assert_eq!(v, u8::MAX - 1);
    assert_eq!(100_i32.offset(1, -1.0, 0, 10), 9);
    assert_eq!(0_i64.offset(10, 0.1, i64::MIN, i64::MAX), 1);
    assert_eq!(0_u128.offset(u128::MAX, 0.5, 0, u128::MAX), 1_u128 << 127);
    assert_eq!(0.0_f64.offset(1e-320, 1.0, -1.0, 1.0), 1e-320);
    assert_eq!(1e-9_f64.offset(1.0, 1.0, -10.0, 10.0), 1.000000001);
    assert_eq!(0.3_f64.offset(0.1, -3.0, -10.0, 10.0), 0.0);
}
