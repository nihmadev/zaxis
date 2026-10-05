use crate::prelude::*;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
};
use zaxis::{vec2, Rect, ScrollArea, Window};

const LABELS: [&str; 10] = [
    "Easing", "Curves", "Tweens", "Timeline", "Springs", "Physics", "Compose", "Effects", "Layout",
    "Reorder",
];

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(1600, 600), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
/// One pass with a window `width` wide; returns every tab's rectangle.
fn strip(c: &mut Context, width: f32, selected: &mut usize) -> Vec<Rect> {
    labeled(c, width, selected, &LABELS)
}
fn labeled(c: &mut Context, width: f32, selected: &mut usize, labels: &[&str]) -> Vec<Rect> {
    let mut rects = Vec::new();
    c.run(|c| {
        Window::new("Tabs")
            .id(Id::new(("tabs", width as u32)))
            .default_position(vec2(10.0, 10.0))
            .default_size(vec2(width, 200.0))
            .resizable(false)
            .show(c, |ui| {
                let tabs = ui.tab_bar(selected, labels.iter().copied().enumerate());
                rects = tabs.iter().map(|tab| tab.rect).collect();
            });
    });
    rects
}
fn wheel(c: &mut Context, pointer: Vec2, delta: Vec2) -> EventResponse {
    c.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(pointer.x as f64, pointer.y as f64),
    });
    c.on_window_event(&WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            -delta.x as f64,
            -delta.y as f64,
        )),
        phase: TouchPhase::Moved,
    })
}

#[test]
fn rows_that_fit_stay_inside_the_window_without_scrolling() {
    let mut c = setup();
    let mut selected = 0;
    // Plenty of room: equal shares.
    let rects = strip(&mut c, 1500.0, &mut selected);
    let width = rects[0].size().x;
    assert!(rects.iter().all(|r| (r.size().x - width).abs() < 0.01));
    assert!(rects.last().unwrap().max.x <= 1510.0);
    let rects = strip(&mut c, 900.0, &mut selected);
    assert!(rects.windows(2).all(|pair| pair[0].max.x <= pair[1].min.x));
    assert!(rects.last().unwrap().max.x <= 910.0, "{:?}", rects.last());
}

#[test]
fn an_uneven_label_gets_its_natural_width_instead_of_overflowing() {
    let mut c = setup();
    let mut selected = 0;
    let labels = ["a", "b", "c", "A considerably longer tab"];
    let rects = labeled(&mut c, 400.0, &mut selected, &labels);
    let widths: Vec<f32> = rects.iter().map(|r| r.size().x).collect();
    assert!(widths[3] > widths[0] * 1.5, "{widths:?}");
    assert!(
        rects.last().unwrap().max.x <= 410.0,
        "fits without scrolling"
    );
    assert!(rects.windows(2).all(|pair| pair[0].max.x <= pair[1].min.x));
}

#[test]
fn narrow_row_scrolls_with_a_plain_wheel_and_reaches_the_last_tab() {
    let mut c = setup();
    let mut selected = 0;
    let rects = strip(&mut c, 400.0, &mut selected);
    let window_right = 410.0;
    let last = *rects.last().unwrap();
    assert!(last.max.x > window_right, "last tab starts hidden");
    // Every label keeps its own width instead of being squeezed.
    assert!(rects.iter().all(|r| r.size().x >= 40.0));

    // A vertical wheel over the strip moves it sideways.
    let over = rects[0].center();
    for _ in 0..40 {
        wheel(&mut c, over, vec2(0.0, 40.0));
        strip(&mut c, 400.0, &mut selected);
    }
    let rects = strip(&mut c, 400.0, &mut selected);
    let last = *rects.last().unwrap();
    assert!(
        last.max.x <= window_right + 0.5 && last.min.x >= 10.0,
        "last tab scrolled into view: {last:?}"
    );

    // And it can be selected there.
    c.move_pointer(last.center());
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    strip(&mut c, 400.0, &mut selected);
    assert_eq!(selected, LABELS.len() - 1);
}

#[test]
fn wheel_over_a_horizontal_area_passes_the_rest_to_the_parent() {
    let mut c = setup();
    // Returns (inner offset, outer offset, a point over the inner area).
    let frame = |c: &mut Context| {
        let mut out = (0.0, 0.0, Vec2::ZERO);
        c.run(|c| {
            Window::new("Nested")
                .default_position(vec2(10.0, 10.0))
                .default_size(vec2(400.0, 300.0))
                .resizable(false)
                .show(c, |ui| {
                    let outer = ScrollArea::vertical()
                        .id_source("outer")
                        .max_height(200.0)
                        .show(ui, |ui| {
                            let inner = ScrollArea::horizontal()
                                .id_source("inner")
                                .max_height(30.0)
                                .show(ui, |ui| {
                                    for i in 0..30 {
                                        ui.button(format!("Button {i}"));
                                    }
                                });
                            ui.add_space(400.0);
                            (inner.offset.x, inner.viewport.center())
                        });
                    out = (outer.inner.0, outer.offset.y, outer.inner.1);
                });
        });
        out
    };
    let (_, _, point) = frame(&mut c);
    // 100 px of wheel: 100 consumed horizontally, nothing left for the parent.
    wheel(&mut c, point, vec2(0.0, 100.0));
    let (inner, outer, _) = frame(&mut c);
    assert_eq!((inner, outer), (100.0, 0.0));
    // Past the end the remainder scrolls the parent.
    wheel(&mut c, point, vec2(0.0, 600.0));
    let (inner, outer, _) = frame(&mut c);
    assert!(inner > 100.0 && outer > 0.0, "{inner} {outer}");
}
