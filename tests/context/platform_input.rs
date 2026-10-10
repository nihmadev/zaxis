//! Input without a window: `InputEvent` takes the path winit events take.

use super::*;
use zaxis::winit::{
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey},
};
use zaxis::{ImeEvent, InputEvent, KeyInput, TextEdit, WheelDelta};

fn pointer(context: &mut Context, at: zaxis::Vec2) -> zaxis::EventResponse {
    let scale = context.scale_factor();
    context.on_input(InputEvent::PointerMoved {
        x: f64::from(at.x * scale),
        y: f64::from(at.y * scale),
    })
}

fn button(context: &mut Context, state: ElementState) -> zaxis::EventResponse {
    context.on_input(InputEvent::Button {
        button: MouseButton::Left,
        state,
    })
}

fn key(code: KeyCode, state: ElementState, text: Option<&str>) -> InputEvent {
    InputEvent::Key(KeyInput {
        physical: PhysicalKey::Code(code),
        logical: Key::Character(text.unwrap_or("").into()),
        state,
        repeat: false,
        text: text.map(Into::into),
    })
}

fn click_center(context: &mut Context, rect: Rect) {
    pointer(context, rect.center());
    button(context, ElementState::Pressed);
    button(context, ElementState::Released);
}

#[test]
fn a_click_through_input_events_activates_the_button() {
    let mut context = context();
    let response = build(&mut context, "Label");
    pointer(&mut context, response.rect.center());
    let press = button(&mut context, ElementState::Pressed);
    assert!(press.consumed && press.repaint);
    button(&mut context, ElementState::Released);
    assert!(build(&mut context, "Label").clicked());
}

#[test]
fn input_events_and_window_events_give_the_same_results() {
    let (mut by_input, mut by_window) = (context(), context());
    let (mut left, mut right) = (false, false);
    let a = build_checkbox(&mut by_input, &mut left, true);
    let b = build_checkbox(&mut by_window, &mut right, true);
    assert_eq!(a.rect, b.rect);
    let center = a.rect.center();
    let consumed_input = pointer(&mut by_input, center).consumed;
    let consumed_window = by_window
        .on_window_event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(f64::from(center.x), f64::from(center.y)),
        })
        .consumed;
    assert_eq!(consumed_input, consumed_window);
    button(&mut by_input, ElementState::Pressed);
    button(&mut by_input, ElementState::Released);
    mouse(&mut by_window, ElementState::Pressed);
    mouse(&mut by_window, ElementState::Released);
    build_checkbox(&mut by_input, &mut left, true);
    build_checkbox(&mut by_window, &mut right, true);
    assert!(left && right, "both paths toggle the checkbox");
}

#[test]
fn empty_space_is_not_consumed_and_a_control_is() {
    let mut context = context();
    let response = build(&mut context, "Label");
    assert!(!pointer(&mut context, vec2(790.0, 590.0)).consumed);
    assert!(pointer(&mut context, response.rect.center()).consumed);
}

#[test]
fn typing_reaches_a_focused_text_field_and_ctrl_text_does_not() {
    let mut context = context();
    let mut text = String::new();
    let edit = |context: &mut Context, text: &mut String| {
        let mut rect = None;
        context.run(|context| {
            Window::new("Edit").show(context, |ui| {
                rect = Some(ui.add(TextEdit::new(text).id_source("edit")).rect);
            });
        });
        rect.unwrap()
    };
    let rect = edit(&mut context, &mut text);
    click_center(&mut context, rect);
    edit(&mut context, &mut text);
    let typed = context.on_input(key(KeyCode::KeyA, ElementState::Pressed, Some("a")));
    assert!(typed.repaint);
    context.on_input(key(KeyCode::KeyA, ElementState::Released, None));
    edit(&mut context, &mut text);
    assert_eq!(text, "a");
    context.on_input(InputEvent::Modifiers(ModifiersState::CONTROL));
    context.on_input(key(KeyCode::KeyB, ElementState::Pressed, Some("b")));
    edit(&mut context, &mut text);
    assert_eq!(text, "a", "Ctrl+B is a shortcut, not text");
    context.on_input(InputEvent::Modifiers(ModifiersState::empty()));
    context.on_input(InputEvent::Ime(ImeEvent::Commit("é".into())));
    edit(&mut context, &mut text);
    assert_eq!(text, "aé");
}

#[test]
fn navigation_follows_the_logical_key() {
    let mut context = context();
    let mut value = 0.0;
    let build = |context: &mut Context, value: &mut f32| {
        let mut rect = None;
        context.run(|context| {
            Window::new("S").show(context, |ui| {
                rect = Some(ui.add(Slider::new(value, -10.0..=10.0).step(1.0)).rect);
            });
        });
        rect.unwrap()
    };
    let rect = build(&mut context, &mut value);
    click_center(&mut context, rect);
    build(&mut context, &mut value);
    let before = value;
    // NumPad4 with NumLock off: a numeric scan code that means ArrowLeft.
    context.on_input(InputEvent::Key(KeyInput {
        physical: PhysicalKey::Code(KeyCode::Numpad4),
        logical: Key::Named(NamedKey::ArrowLeft),
        state: ElementState::Pressed,
        repeat: false,
        text: None,
    }));
    build(&mut context, &mut value);
    assert!(value < before, "{value} should be below {before}");
}

#[test]
fn focus_loss_releases_a_held_button() {
    let mut context = context();
    let response = build(&mut context, "Label");
    pointer(&mut context, response.rect.center());
    button(&mut context, ElementState::Pressed);
    assert!(context.input().primary_down);
    context.on_input(InputEvent::Focus(false));
    assert!(!context.input().primary_down);
    assert!(!context.input().focused);
    context.on_input(InputEvent::Focus(true));
    assert!(context.input().focused);
}

#[test]
fn resize_and_dpi_change_the_logical_viewport() {
    let mut context = context();
    context.on_input(InputEvent::Resized {
        width: 1600,
        height: 900,
    });
    assert_eq!(context.viewport().size(), vec2(1600.0, 900.0));
    context.on_input(InputEvent::ScaleFactor(2.0));
    assert_eq!(context.scale_factor(), 2.0);
    assert_eq!(context.viewport().size(), vec2(800.0, 450.0));
    context.on_input(InputEvent::Resized {
        width: 3200,
        height: 1800,
    });
    assert_eq!(context.viewport().size(), vec2(1600.0, 900.0));
}

#[test]
fn wheel_units_scale_by_line_height_and_dpi() {
    let mut context = context();
    context.on_input(InputEvent::Wheel(WheelDelta::Lines(vec2(0.0, 1.0))));
    let lines = context.input().scroll_delta;
    context.on_input(InputEvent::ScaleFactor(2.0));
    context.run(|_| {});
    context.on_input(InputEvent::Wheel(WheelDelta::Pixels(vec2(0.0, 40.0))));
    assert_eq!(context.input().scroll_delta, vec2(0.0, 20.0));
    assert!(lines.y > 0.0 && lines.x == 0.0);
}

#[test]
fn window_events_convert_to_input_events() {
    let converted = |event: WindowEvent| InputEvent::from_window_event(&event);
    assert_eq!(
        converted(WindowEvent::Focused(true)),
        Some(InputEvent::Focus(true))
    );
    assert_eq!(
        converted(WindowEvent::Resized(PhysicalSize::new(10, 20))),
        Some(InputEvent::Resized {
            width: 10,
            height: 20
        })
    );
    assert_eq!(
        converted(WindowEvent::MouseWheel {
            device_id: DeviceId::dummy(),
            delta: MouseScrollDelta::LineDelta(1.0, -2.0),
            phase: zaxis::winit::event::TouchPhase::Moved,
        }),
        Some(InputEvent::Wheel(WheelDelta::Lines(vec2(1.0, -2.0))))
    );
    assert_eq!(
        converted(WindowEvent::CursorLeft {
            device_id: DeviceId::dummy()
        }),
        Some(InputEvent::PointerLeft)
    );
    assert_eq!(converted(WindowEvent::RedrawRequested), None);
}

#[test]
fn unknown_window_events_do_nothing() {
    let mut context = context();
    let response = context.on_window_event(&WindowEvent::RedrawRequested);
    assert!(!response.consumed && !response.repaint);
}
