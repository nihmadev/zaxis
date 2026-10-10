use zaxis::winit::{
    event::{ElementState, MouseButton},
    keyboard::ModifiersState,
};
use zaxis::{vec2, Context, InputEvent, PanZoomState, Rect, WheelDelta};

pub fn input(c: &mut Context, viewport: Rect, stage: usize) {
    let point = viewport.min + vec2(12.0, 12.0);
    let physical = point * c.scale_factor();
    c.on_input(InputEvent::PointerMoved {
        x: physical.x as f64,
        y: physical.y as f64,
    });
    if stage == 1 {
        c.on_input(InputEvent::Modifiers(ModifiersState::CONTROL));
        assert!(
            c.on_input(InputEvent::Wheel(WheelDelta::Pixels(
                vec2(0.0, -60.0) * c.scale_factor()
            )))
            .consumed
        );
    } else if stage == 2 {
        c.on_input(InputEvent::Modifiers(ModifiersState::empty()));
        assert!(
            c.on_input(InputEvent::Button {
                button: MouseButton::Left,
                state: ElementState::Pressed
            })
            .consumed
        );
        assert!(
            c.on_input(InputEvent::PointerMoved {
                x: (physical.x + 20.0 * c.scale_factor()) as f64,
                y: (physical.y + 10.0 * c.scale_factor()) as f64
            })
            .consumed
        );
        assert!(
            c.on_input(InputEvent::Button {
                button: MouseButton::Left,
                state: ElementState::Released
            })
            .consumed
        );
    }
}
pub fn verify(camera: &PanZoomState, before: PanZoomState, stage: usize) {
    assert!(camera.scale.is_finite() && camera.scale > 0.0 && camera.translation.is_finite());
    if stage > 0 {
        let (scale, translation) = match stage {
            1 => {
                let factor = (-0.12_f32).exp();
                let anchor = vec2(12.0, 12.0);
                (
                    before.scale * factor,
                    anchor + (before.translation - anchor) * factor,
                )
            }
            2 => (before.scale, before.translation + vec2(20.0, 10.0)),
            _ => (before.scale, before.translation),
        };
        assert!((camera.scale - scale).abs() < 0.0001);
        assert!((camera.translation - translation).abs().max_element() < 0.001);
    }
}
