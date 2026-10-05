use crate::prelude::*;
use winit::{
    dpi::PhysicalSize,
    event::ElementState,
    keyboard::{KeyCode, ModifiersState},
};
use zaxis::{Slider, Window};

fn draw(context: &mut Context, value: &mut f32, enabled: bool) -> zaxis::Response {
    context.request_repaint();
    let mut response = None;
    context.run(|context| {
        Window::new("Test").show(context, |ui| {
            ui.button("Before");
            response = Some(ui.add(Slider::new(value, 0.0..=10.0).step(3.0).enabled(enabled)));
            ui.button("After");
        });
    });
    response.unwrap()
}

#[test]
fn tab_arrows_repeat_pages_and_endpoints_preserve_event_order() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut value = 0.0;
    draw(&mut context, &mut value, true);
    assert!(context.key(KeyCode::Tab, ElementState::Pressed, false));
    assert!(context.key(KeyCode::Tab, ElementState::Pressed, false));
    assert!(draw(&mut context, &mut value, true).has_focus);
    for repeat in [false, true, true] {
        assert!(context.key(KeyCode::ArrowRight, ElementState::Pressed, repeat));
    }
    assert!(context.key(KeyCode::ArrowRight, ElementState::Released, false));
    assert!(draw(&mut context, &mut value, true).changed());
    assert_eq!(value, 9.0);
    context.key(KeyCode::End, ElementState::Pressed, false);
    context.key(KeyCode::ArrowLeft, ElementState::Pressed, false);
    draw(&mut context, &mut value, true);
    assert_eq!(value, 9.0);
    for (key, expected) in [
        (KeyCode::Home, 0.0),
        (KeyCode::PageUp, 10.0),
        (KeyCode::PageDown, 0.0),
        (KeyCode::ArrowUp, 3.0),
        (KeyCode::ArrowDown, 0.0),
    ] {
        context.key(key, ElementState::Pressed, false);
        draw(&mut context, &mut value, true);
        assert_eq!(value, expected);
    }
    assert!(!context.key(KeyCode::Enter, ElementState::Pressed, false));
    assert!(!context.key(KeyCode::Space, ElementState::Pressed, false));
    context.key(KeyCode::Tab, ElementState::Pressed, false);
    assert!(!draw(&mut context, &mut value, true).has_focus);
    context.set_modifiers(ModifiersState::SHIFT);
    context.key(KeyCode::Tab, ElementState::Pressed, false);
    assert!(draw(&mut context, &mut value, true).has_focus);
    context.key(KeyCode::ArrowRight, ElementState::Pressed, false);
    assert!(!draw(&mut context, &mut value, false).changed());
    assert_eq!(value, 0.0);
    assert!(!context.key(KeyCode::ArrowRight, ElementState::Pressed, false));
}

#[test]
fn continuous_and_zero_span_ranges_handle_keyboard_and_extreme_values() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut value = 0.0;
    let draw = |context: &mut Context, value: &mut f32, range| {
        context.request_repaint();
        context.run(|context| {
            Window::new("Test").show(context, |ui| {
                ui.slider(value, range);
            });
        });
    };
    draw(&mut context, &mut value, 0.0..=1.0);
    context.key(KeyCode::Tab, ElementState::Pressed, false);
    context.key(KeyCode::ArrowRight, ElementState::Pressed, false);
    draw(&mut context, &mut value, 0.0..=1.0);
    assert_eq!(value, 0.01);
    draw(&mut context, &mut value, 5.0..=5.0);
    context.key(KeyCode::PageUp, ElementState::Pressed, false);
    draw(&mut context, &mut value, 5.0..=5.0);
    assert_eq!(value, 5.0);
    draw(&mut context, &mut value, -f32::MAX..=f32::MAX);
    context.key(KeyCode::End, ElementState::Pressed, false);
    draw(&mut context, &mut value, -f32::MAX..=f32::MAX);
    assert_eq!(value, f32::MAX);
    assert!(context
        .draw_data()
        .vertices
        .iter()
        .all(|vertex| { vertex.position.iter().all(|v| v.is_finite()) }));
}

#[test]
fn labeled_slider_formats_the_updated_value_and_preserves_disabled_values() {
    let mut context = Context::new();
    let mut style = context.style().clone();
    style.motion.reduced_motion = true;
    context.set_style(style);
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut value = 12.0;
    let draw = |context: &mut Context, value: &mut f32, enabled| {
        let mut response = None;
        context.run(|context| {
            Window::new("Test").show(context, |ui| {
                response = Some(
                    ui.add(
                        Slider::new(value, 0.0..=10.0)
                            .text("Volume##id")
                            .precision(0)
                            .suffix("%")
                            .enabled(enabled),
                    ),
                );
            });
        });
        response.unwrap()
    };
    let response = draw(&mut context, &mut value, false);
    assert!(!response.changed());
    match &context.probe().cache[&response.id.with("label")].paint[0] {
        Paint::Text { text, color, .. } => {
            assert_eq!(text, "Volume: 12%");
            assert_eq!(*color, context.style().disabled_text);
        }
        _ => panic!("expected a slider caption"),
    }
    let response = draw(&mut context, &mut value, true);
    assert!(response.changed());
    assert_eq!(value, 10.0);
    assert!(
        matches!(&context.probe().cache[&response.id.with("label")].paint[0], Paint::Text { text, .. } if text == "Volume: 10%")
    );
    assert!(context.needs_repaint());
    let revision = context.draw_data().revision;
    draw(&mut context, &mut value, true);
    assert_eq!(context.draw_data().revision, revision);
    assert!(!context.needs_repaint());
}

#[test]
fn labeled_sliders_keep_captions_below_tracks_in_horizontal_rows() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut values = [0.0, 1.0];
    let mut responses = Vec::new();
    context.run(|context| {
        Window::new("Test")
            .default_size(Vec2::new(600.0, 300.0))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    for value in &mut values {
                        responses
                            .push(ui.add(Slider::new(value, 0.0..=1.0).text("Gain").width(180.0)));
                    }
                });
            });
    });
    assert_eq!(responses[0].rect.min.y, responses[1].rect.min.y);
    assert!(responses[1].rect.min.x >= responses[0].rect.max.x);
    for response in responses {
        match &context.probe().cache[&response.id.with("label")].paint[0] {
            Paint::Text {
                position,
                wrap_width,
                ..
            } => {
                assert_eq!(position.x, response.rect.min.x);
                assert!(position.y > response.rect.max.y);
                assert_eq!(*wrap_width, response.rect.size().x);
            }
            _ => panic!("expected a caption below the track"),
        }
    }
}
