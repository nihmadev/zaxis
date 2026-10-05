use crate::prelude::*;
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState};
use zaxis::context::tooltip::place;
use zaxis::{vec2, Align, Button, Padding, Root, Tooltip};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(640, 360), 1.0);
    c
}

fn draw(c: &mut Context, now: Instant, enabled: bool) -> Id {
    let mut target = Id::new("unused");
    c.run_at(now, |c| {
        Root::new().show(c, |ui| {
            ui.horizontal_aligned(Align::Center, |ui| {
                ui.spacer();
                target = ui
                    .add(
                        Tooltip::new(
                            "A useful explanation that is much wider than this tiny button.",
                        )
                        .enabled(enabled)
                        .wrap(Button::new("?")),
                    )
                    .id;
            });
        });
    });
    target
}

#[test]
fn tooltip_delay_layout_and_passive_input() {
    let mut c = setup();
    let now = Instant::now();
    let target = draw(&mut c, now, true);
    let hit = *c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.id == target)
        .unwrap();
    c.move_pointer(hit.rect.center());
    draw(&mut c, now, true);
    assert!(!c.probe().seen.contains(&target.with("tooltip")));
    assert!(c.next_repaint().is_some());
    let hits = c.probe().previous_hits.len();
    let later = now + Duration::from_millis(400);
    draw(&mut c, later, true);
    let paint = &c.probe().cache[&target.with("tooltip")].paint;
    let Paint::Shape(Shape::Rect { rect, .. }) = &paint[0] else {
        panic!("tooltip box")
    };
    assert!(rect.size().x > hit.rect.size().x * 3.0);
    assert!(rect.min.x >= c.viewport().min.x && rect.max.x <= c.viewport().max.x);
    assert_eq!(c.probe().previous_hits.len(), hits);
    assert!(c.probe().popup.is_none());
    assert!(c.probe().focused_widget.is_none());
    assert_eq!(c.hit_test(hit.rect.center()).unwrap().id, target);
    c.primary_button(ElementState::Pressed);
    draw(&mut c, later, true);
    assert!(!c.probe().seen.contains(&target.with("tooltip")));
    c.primary_button(ElementState::Released);
    draw(&mut c, later, false);
    assert!(!c.probe().seen.contains(&target.with("tooltip")));
}

#[test]
fn text_wraps_at_words_and_fits_above_bottom_anchor() {
    let mut c = setup();
    let (text, size, wrap) = c.tooltip_text(
        "Первая строка с пояснением\nSecond paragraph with words",
        14.0,
        FontWeight::REGULAR,
        180.0,
    );
    assert!(text.contains("\nSecond"));
    assert!(size.x <= 180.0 && size.y < 140.0);
    assert_eq!(c.measure_text(&text, 14.0, FontWeight::REGULAR, wrap), size);
    let anchor = Rect::from_min_size(vec2(610.0, 330.0), vec2(20.0, 20.0));
    let rect = place(anchor, size + vec2(20.0, 12.0), c.viewport());
    assert!(rect.max.y < anchor.min.y);
    assert!(rect.max.x <= 640.0 && rect.min.x >= 0.0);
}

#[test]
fn passive_text_can_have_a_tooltip_and_global_disable_is_respected() {
    let mut c = setup();
    c.probe_mut().style.tooltip.delay = Duration::ZERO;
    c.move_pointer(vec2(25.0, 25.0));
    let mut id = Id::new("unused");
    let build = |c: &mut Context, id: &mut Id| {
        Root::new().padding(Padding::all(20.0)).show(c, |ui| {
            *id = ui
                .add(Tooltip::new("Label explanation").wrap(zaxis::Text::new("Label")))
                .id;
        });
    };
    c.run(|c| build(c, &mut id));
    assert!(c.probe().seen.contains(&id.with("tooltip")));
    assert!(!c
        .probe()
        .previous_hits
        .iter()
        .any(|h| h.id == id.with("tooltip-anchor")));
    c.probe_mut().style.tooltip.enabled = false;
    c.run(|c| build(c, &mut id));
    assert!(!c.probe().seen.contains(&id.with("tooltip")));
}

#[test]
fn tooltip_follows_scrolling_and_escapes_the_scroll_clip() {
    let mut c = setup();
    c.probe_mut().style.tooltip.delay = Duration::ZERO;
    let mut target = Id::new("unused");
    let build = |c: &mut Context, target: &mut Id, offset: f32| {
        Root::new().show(c, |ui| {
            zaxis::ScrollArea::vertical()
                .max_height(60.0)
                .scroll_offset(vec2(0.0, offset))
                .show(ui, |ui| {
                    ui.add_space(45.0);
                    let response = ui.button("Target");
                    *target = response.id;
                    ui.tooltip(response, "A hint outside the small scrolling viewport.");
                    ui.add_space(150.0);
                });
        });
    };
    c.run(|c| build(c, &mut target, 30.0));
    let hit = *c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.id == target)
        .unwrap();
    c.move_pointer(hit.rect.intersect(hit.clip).center());
    c.run(|c| build(c, &mut target, 30.0));
    let Paint::Shape(Shape::Rect { rect, .. }) = c.probe().cache[&target.with("tooltip")].paint[0]
    else {
        panic!("tooltip box");
    };
    assert!(rect.min.y >= hit.rect.max.y);
    assert!(rect.max.y > hit.clip.max.y);
    c.run(|c| build(c, &mut target, 120.0));
    assert!(!c.probe().seen.contains(&target.with("tooltip")));
}

#[test]
fn response_hint_follows_a_centered_passive_label() {
    let mut c = setup();
    c.probe_mut().style.tooltip.delay = Duration::ZERO;
    let mut target = Id::new("unused");
    c.move_pointer(vec2(320.0, 25.0));
    c.run(|c| {
        Root::new().padding(Padding::all(20.0)).show(c, |ui| {
            ui.vertical_aligned(Align::Center, |ui| {
                let response = ui.label("Centered label");
                target = response.id;
                ui.tooltip(response, "Explanation");
            });
        });
    });
    assert!(c.probe().seen.contains(&target.with("tooltip")));
    let Paint::Shape(Shape::Rect { rect, .. }) = c.probe().cache[&target.with("tooltip")].paint[0]
    else {
        panic!("tooltip box");
    };
    assert!(rect.min.x > 200.0);
}
