use crate::prelude::*;
use winit::dpi::PhysicalSize;
#[cfg(not(target_os = "macos"))]
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    window::ResizeDirection,
};
#[cfg(not(target_os = "macos"))]
use zaxis::{context::native_chrome::resize_direction, vec2, Padding};
use zaxis::{Root, TitleBar};

#[test]
#[cfg(not(target_os = "macos"))]
fn full_caption_has_no_padding_gaps_and_buttons_keep_press_release_pairs() {
    for scale in [1.0, 2.0] {
        let mut context = Context::new();
        context.set_viewport(
            PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
            scale,
        );
        context.run(|context| {
            Root::new().padding(Padding::all(0.0)).show(context, |_| {});
            TitleBar::new("Test").show(context);
        });
        let chrome = context.probe().native_chrome.as_ref().unwrap();
        for point in [vec2(10.0, 8.0), vec2(300.0, 44.0), vec2(660.0, 23.0)] {
            assert!(chrome.drag.contains(point));
        }
        assert!(!chrome.drag.contains(vec2(680.0, 23.0)));
        assert_eq!(
            resize_direction(context.viewport(), vec2(2.0, 2.0), 5.0),
            Some(ResizeDirection::NorthWest)
        );
        for (index, x) in [(0, 685.0), (1, 731.0), (2, 777.0)] {
            context.on_window_event(&WindowEvent::CursorMoved {
                device_id: DeviceId::dummy(),
                position: PhysicalPosition::new(x * scale, 23.0 * scale),
            });
            // No redraw between press and release, matching rapid OS event delivery.
            for state in [ElementState::Pressed, ElementState::Released] {
                context.on_window_event(&WindowEvent::MouseInput {
                    device_id: DeviceId::dummy(),
                    state,
                    button: MouseButton::Left,
                });
            }
            context.run(|context| {
                Root::new().show(context, |_| {});
                let response = TitleBar::new("Test").show(context);
                assert_eq!(
                    [response.minimize, response.maximize, response.close],
                    [index == 0, index == 1, index == 2]
                );
            });
        }
        context.run(|context| {
            Root::new().show(context, |_| {});
        });
        assert!(context.probe().native_chrome.is_none());
    }
}

#[test]
#[cfg(target_os = "macos")]
fn system_caption_has_no_custom_controls_or_reserved_client_area() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 2.0);
    assert_eq!(TitleBar::HEIGHT, 0.0);
    context.run(|context| {
        Root::new().show(context, |_| {});
        let hits = context.probe().hits.len();
        let elements = context.probe().elements.len();
        let response = TitleBar::new("Test").show(context);
        assert!(!response.close && !response.minimize && !response.maximize);
        assert!(context.probe().native_chrome.is_none());
        assert_eq!(context.probe().hits.len(), hits);
        assert_eq!(context.probe().elements.len(), elements);
    });
}
