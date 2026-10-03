use std::time::Duration;

use zaxis::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
};
use zaxis::{
    vec2, Button, Checkbox, Color, Context, Id, Rect, Response, Slider, SliderStatus, Window,
};

fn context() -> Context {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    context
}

fn build(context: &mut Context, label: &str) -> Response {
    let mut response = None;
    context.run(|context| {
        Window::new("Test").show(context, |ui| {
            ui.label(label);
            response = Some(ui.button("Click"));
        });
    });
    response.expect("the UI pass must run")
}

fn move_to(context: &mut Context, position: zaxis::Vec2) {
    let scale = context.scale_factor();
    context.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(
            f64::from(position.x * scale),
            f64::from(position.y * scale),
        ),
    });
}

fn build_checkbox(context: &mut Context, checked: &mut bool, enabled: bool) -> Response {
    let mut response = None;
    context.run(|context| {
        Window::new("Test").show(context, |ui| {
            response = Some(ui.add(Checkbox::new(checked, "Checked").enabled(enabled)));
        });
    });
    response.expect("the checkbox UI pass must run")
}

fn mouse(context: &mut Context, state: ElementState) {
    context.on_window_event(&WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state,
        button: MouseButton::Left,
    });
}

fn build_slider(context: &mut Context, value: &mut f32, enabled: bool) -> Response {
    let mut response = None;
    context.run(|context| {
        Window::new("Test").show(context, |ui| {
            response = Some(ui.add(Slider::new(value, -10.0..=10.0).step(2.0).enabled(enabled)));
        });
    });
    response.expect("the slider UI pass must run")
}

#[path = "context/cases_1.rs"]
mod cases_1;

#[path = "context/cases_2.rs"]
mod cases_2;

#[path = "context/cases_3.rs"]
mod cases_3;
