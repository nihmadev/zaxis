use crate::{vec2, Context, Rect, Root, TextEdit};
use winit::{
    dpi::PhysicalSize,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(900, 700), 1.0);
    c
}

fn draw(c: &mut Context, text: &mut String) -> (Rect, bool) {
    let mut out = (Rect::default(), false);
    c.run(|c| {
        Root::new().show(c, |ui| {
            let r = ui.add(
                TextEdit::new(text)
                    .id_source("area")
                    .multiline()
                    .width(400.0)
                    .rows(8.0),
            );
            out = (r.rect, r.changed());
        });
    });
    out
}

fn click(c: &mut Context, p: crate::Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    let _ = (
        DeviceId::dummy(),
        MouseButton::Left,
        WindowEvent::Focused(true),
        vec2(0.0, 0.0),
    );
}

#[test]
fn typing_shapes_only_the_edited_paragraph() {
    let mut c = setup();
    let mut text = (0..300).map(|n| format!("Line {n}: the quick brown fox jumps over the lazy dog and more words to wrap around")).collect::<Vec<_>>().join("\n");
    let (rect, _) = draw(&mut c, &mut text);
    draw(&mut c, &mut text);
    click(&mut c, rect.center());
    draw(&mut c, &mut text);
    c.on_text_event("a");
    draw(&mut c, &mut text);
    for _ in 0..5 {
        let before = c.cache_stats().text_layouts_built;
        c.on_text_event("a");
        let (_, changed) = draw(&mut c, &mut text);
        assert!(changed);
        let built = c.cache_stats().text_layouts_built - before;
        assert!(built <= 2, "typing shaped {built} paragraphs");
    }
}

#[test]
fn probe_newline_cost() {
    let mut c = setup();
    let mut text = (0..100).map(|n| format!("Line {n}: Привет, мир — the quick brown fox jumps over the lazy dog, again {n} and again")).collect::<Vec<_>>().join("\n");
    let (rect, _) = draw(&mut c, &mut text);
    draw(&mut c, &mut text);
    click(&mut c, rect.center());
    c.input.modifiers = winit::keyboard::ModifiersState::CONTROL;
    c.on_key_event(winit::keyboard::KeyCode::End, ElementState::Pressed, false);
    c.input.modifiers = winit::keyboard::ModifiersState::empty();
    for _ in 0..3 {
        draw(&mut c, &mut text);
    }
    for i in 0..30 {
        let before = c.cache_stats().text_layouts_built;
        c.on_text_event(if i % 10 == 9 { "\n" } else { "a" });
        draw(&mut c, &mut text);
        println!(
            "step {i}: built {}",
            c.cache_stats().text_layouts_built - before
        );
    }
}
