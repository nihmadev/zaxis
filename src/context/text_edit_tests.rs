use super::*;
use crate::{vec2, Response, TextEdit, Window};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Ime, WindowEvent},
    keyboard::ModifiersState,
};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(700, 500), 1.0);
    c
}
fn draw(
    c: &mut Context,
    a: &mut String,
    b: &mut String,
    enabled: bool,
    read_only: bool,
) -> [Response; 2] {
    let mut result = None;
    c.run(|c| {
        Window::new("Edit test")
            .default_size(vec2(500.0, 350.0))
            .show(c, |ui| {
                result = Some([
                    ui.add(
                        TextEdit::new(a)
                            .id_source("a")
                            .width(120.0)
                            .disabled(!enabled)
                            .read_only(read_only),
                    ),
                    ui.add(TextEdit::new(b).id_source("b")),
                ]);
            });
    });
    result.unwrap()
}
fn key(c: &mut Context, key: KeyCode, mods: ModifiersState) {
    c.input.modifiers = mods;
    c.on_key_event(key, ElementState::Pressed, false);
    c.on_key_event(key, ElementState::Released, false);
}
fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}

mod cases_1;

mod cases_2;
