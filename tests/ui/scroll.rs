use crate::prelude::*;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::KeyCode,
};
use zaxis::{vec2, Rect, ScrollArea, ScrollAreaOutput, Shape, Window};

fn setup(scale: f64) -> Context {
    let mut c = Context::new();
    c.set_viewport(
        PhysicalSize::new((700.0 * scale) as u32, (600.0 * scale) as u32),
        scale,
    );
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
fn area(
    c: &mut Context,
    height: f32,
    count: usize,
    offset: Option<Vec2>,
) -> ScrollAreaOutput<Vec<zaxis::Response>> {
    let mut out = None;
    c.run(|c| {
        Window::new("Scroll test")
            .default_size(vec2(600.0, 500.0))
            .show(c, |ui| {
                let mut scroll = ScrollArea::vertical().id_source("list").max_height(height);
                if let Some(offset) = offset {
                    scroll = scroll.scroll_offset(offset);
                }
                out = Some(scroll.show(ui, |ui| {
                    (0..count)
                        .map(|i| ui.push_id(i, |ui| ui.button(format!("Row {i}"))))
                        .collect()
                }));
            });
    });
    out.unwrap()
}
fn wheel(c: &mut Context, pointer: Vec2, delta: Vec2) -> EventResponse {
    c.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(
            (pointer.x * c.probe().scale) as f64,
            (pointer.y * c.probe().scale) as f64,
        ),
    });
    c.on_window_event(&WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            (-delta.x * c.probe().scale) as f64,
            (-delta.y * c.probe().scale) as f64,
        )),
        phase: TouchPhase::Moved,
    })
}

fn nested(
    c: &mut Context,
    inner_offset: Option<f32>,
) -> (ScrollAreaOutput<()>, ScrollAreaOutput<()>) {
    let mut outer = None;
    let mut inner = None;
    c.run(|c| {
        Window::new("Nested")
            .default_size(vec2(600.0, 500.0))
            .show(c, |ui| {
                outer = Some(
                    ScrollArea::vertical()
                        .id_source("outer")
                        .max_height(250.0)
                        .show(ui, |ui| {
                            let mut area =
                                ScrollArea::vertical().id_source("inner").max_height(100.0);
                            if let Some(y) = inner_offset {
                                area = area.scroll_offset(vec2(0.0, y));
                            }
                            inner = Some(area.show(ui, |ui| {
                                ui.allocate_space(vec2(250.0, 400.0));
                            }));
                            ui.allocate_space(vec2(300.0, 900.0));
                        }),
                );
            })
    });
    (outer.unwrap(), inner.unwrap())
}

mod cases_1;

mod cases_2;
