use crate::grid::setup;
use crate::prelude::*;
use winit::{
    event::{DeviceId, ElementState, WindowEvent},
    window::CursorIcon,
};
use zaxis::{Vec2, Window};

fn build(context: &mut Context, resizable: bool) {
    context.run(|context| {
        Window::new("Cursor")
            .resizable(resizable)
            .show(context, |ui| {
                ui.button("Ordinary control");
            });
    });
}

#[test]
fn window_resize_uses_system_cursor_through_capture_release_and_leave() {
    let mut context = setup();
    build(&mut context, true);
    let grip = context
        .probe()
        .previous_hits
        .iter()
        .find(|hit| hit.action == HitAction::Resize)
        .unwrap()
        .rect;
    context.move_pointer(grip.center());
    assert_eq!(context.cursor_icon(), CursorIcon::NwseResize);
    context.on_window_event(&WindowEvent::CursorLeft {
        device_id: DeviceId::dummy(),
    });
    assert_eq!(context.cursor_icon(), CursorIcon::Default);
    context.move_pointer(grip.center());
    context.primary_button(ElementState::Pressed);
    context.move_pointer(Vec2::splat(1000.0));
    assert_eq!(context.cursor_icon(), CursorIcon::NwseResize);
    context.primary_button(ElementState::Released);
    assert_eq!(context.cursor_icon(), CursorIcon::Default);
}

#[test]
fn disabled_or_occluded_resize_regions_dont_change_cursor() {
    let mut context = setup();
    build(&mut context, true);
    let grip = context
        .probe()
        .previous_hits
        .iter()
        .find(|hit| hit.action == HitAction::Resize)
        .unwrap()
        .rect;
    context.move_pointer(grip.center());
    build(&mut context, false);
    assert_eq!(context.cursor_icon(), CursorIcon::Default);
    context.run(|context| {
        Window::new("Cursor").show(context, |_| {});
        Window::new("Cover")
            .default_position(grip.min - Vec2::splat(20.0))
            .default_size(Vec2::splat(200.0))
            .show(context, |_| {});
    });
    assert_eq!(context.cursor_icon(), CursorIcon::Default);
}
