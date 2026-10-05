use crate::prelude::*;
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState};
use zaxis::Instant;
use zaxis::{vec2, ComboBox, ComboBoxOption, Popup, Response, ScrollArea, Window};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(700, 500), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
fn options(count: usize) -> Vec<ComboBoxOption<usize>> {
    (0..count)
        .map(|i| ComboBoxOption::new(i, i, format!("Option {i}")))
        .collect()
}
fn draw(
    c: &mut Context,
    selected: &mut Option<usize>,
    options: &[ComboBoxOption<usize>],
    filter: bool,
    disabled: bool,
) -> Response {
    let mut result = None;
    c.run(|c| {
        Window::new("test")
            .default_size(vec2(500.0, 350.0))
            .show(c, |ui| {
                result = Some(
                    ui.add(
                        ComboBox::new(selected, options)
                            .id_source("combo")
                            .label("Label")
                            .filterable(filter)
                            .enabled(!disabled),
                    ),
                );
                ui.button("Under popup");
            });
    });
    result.unwrap()
}
fn key(c: &mut Context, code: KeyCode) {
    assert!(c.on_key_event(code, ElementState::Pressed, false).consumed);
    c.on_key_event(code, ElementState::Released, false);
}
fn click(c: &mut Context, point: Vec2) {
    c.move_pointer(point);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}
fn row_hit(c: &Context, combo: Id, option: Id) -> HitRegion {
    *c.probe()
        .previous_hits
        .iter()
        .find(|h| h.id == combo.with(("option", option)))
        .unwrap()
}

mod cases_1;

mod cases_2;
