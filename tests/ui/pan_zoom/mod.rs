//! Public input → published owner → camera → paint/hit geometry.
use crate::prelude::*;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, MouseButton},
    keyboard::ModifiersState,
};

mod camera;
mod integration;
mod robustness;
mod routing;
mod text;

fn setup(dpi: f64) -> Context {
    let mut c = Context::new();
    c.set_viewport(
        PhysicalSize::new((800.0 * dpi) as u32, (600.0 * dpi) as u32),
        dpi,
    );
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    style.text_edit_blink_interval = std::time::Duration::ZERO;
    c.set_style(style);
    c
}
fn pointer(c: &mut Context, p: Vec2) -> EventResponse {
    let p = p * c.scale_factor();
    c.on_input(InputEvent::PointerMoved {
        x: p.x as f64,
        y: p.y as f64,
    })
}
fn button(c: &mut Context, state: ElementState) -> EventResponse {
    c.on_input(InputEvent::Button {
        button: MouseButton::Left,
        state,
    })
}
fn wheel(c: &mut Context, p: Vec2, pixels: f32, mods: ModifiersState) -> bool {
    pointer(c, p);
    c.on_input(InputEvent::Modifiers(mods));
    c.on_input(InputEvent::Wheel(WheelDelta::Pixels(vec2(
        0.0,
        pixels * c.scale_factor(),
    ))))
    .consumed
}
fn draw<R>(
    c: &mut Context,
    state: &mut PanZoomState,
    build: impl FnOnce(&mut Ui<'_>, Rect) -> R,
) -> PanZoomOutput<R> {
    let mut out = None;
    c.run(|c| {
        Root::new().padding(Padding::all(0.0)).show(c, |ui| {
            out = Some(PanZoom::new("scene", vec2(400.0, 300.0)).show(ui, state, build));
        })
    });
    out.unwrap()
}
fn empty(c: &mut Context, state: &mut PanZoomState) -> PanZoomOutput<()> {
    draw(c, state, |_, _| {})
}
fn near(a: Vec2, b: Vec2) {
    assert!((a - b).abs().max_element() < 0.01, "{a:?} != {b:?}");
}
