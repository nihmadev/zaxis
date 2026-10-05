use crate::prelude::*;
use zaxis::winit::{
    dpi::PhysicalSize,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
};
use zaxis::{vec2, Window};
use zaxis::{Padding, Root};

#[test]
fn root_tracks_viewport_and_reuses_unchanged_geometry() {
    let mut context = Context::new();
    for (size, scale) in [
        (PhysicalSize::new(640, 480), 1.0),
        (PhysicalSize::new(1600, 1200), 2.0),
    ] {
        context.set_viewport(size, scale);
        let mut clip = context.viewport();
        context.run(|context| {
            let result = Root::new().padding(Padding::all(0.0)).show(context, |ui| {
                clip = ui.clip_rect();
                ui.label("Root content");
                42
            });
            assert_eq!(result, 42);
        });
        assert_eq!(clip, context.viewport());
        assert_eq!(
            context.probe().windows[&Id::new("zaxis-root")].displayed_rect,
            clip
        );
        let revision = context.draw_data().revision;
        context.run(|context| {
            Root::new().padding(Padding::all(0.0)).show(context, |ui| {
                ui.label("Root content");
            });
        });
        assert_eq!(context.draw_data().revision, revision);
    }
}

#[test]
fn clicking_root_keeps_floating_windows_above_it() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(640, 480), 1.0);
    let panel = Id::new("floating");
    // Build the root last to also verify that callback order cannot cover panels.
    context.run(|context| {
        Window::new("Floating").id(panel).show(context, |ui| {
            ui.button("Panel");
        });
        Root::new().show(context, |ui| {
            ui.button("Root");
        });
    });
    assert_eq!(context.top_window(vec2(100.0, 100.0)), Some(panel));
    context.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: zaxis::winit::dpi::PhysicalPosition::new(600.0, 400.0),
    });
    context.on_window_event(&WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state: ElementState::Pressed,
        button: MouseButton::Left,
    });
    assert_eq!(context.top_window(vec2(100.0, 100.0)), Some(panel));
}
