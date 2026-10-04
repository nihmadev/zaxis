//! Interactions that were in flight under a modal end when it opens.
use super::*;
use crate::{SplitPane, SplitPanel, Window};

fn splits(c: &mut Context, open: &mut bool) -> Vec<f32> {
    let mut sizes = Vec::new();
    c.run(|c| {
        Root::new().padding(crate::Padding::all(0.0)).show(c, |ui| {
            let out = SplitPane::horizontal("split")
                .panels(
                    (0..3)
                        .map(|i| SplitPanel::new(i).min_size(30.0))
                        .collect::<Vec<_>>(),
                )
                .show(ui, |_| {});
            sizes = out.panels.iter().map(|p| p.size).collect();
            Modal::new("m").show(ui, open, |ui| {
                ui.label("modal");
            });
        });
    });
    sizes
}

#[test]
fn split_resize_ends_when_a_modal_opens() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 300), 1.0);
    let mut open = false;
    let before = splits(&mut c, &mut open);
    let handle = c
        .previous_hits
        .iter()
        .find(|h| matches!(h.action, HitAction::SplitResize { .. }))
        .map(|h| center(h.rect))
        .unwrap();
    c.move_pointer(handle);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(handle + vec2(30.0, 0.0));
    let dragged = splits(&mut c, &mut open);
    assert!(dragged[0] > before[0]);
    open = true;
    splits(&mut c, &mut open);
    assert!(c.capture.is_none());
    c.move_pointer(handle + vec2(90.0, 0.0));
    let after = splits(&mut c, &mut open);
    assert!(
        (after[0] - dragged[0]).abs() < 0.5,
        "the divider stays where it ended"
    );
    c.primary_button(ElementState::Released);
    let released = splits(&mut c, &mut open);
    assert!((released[0] - dragged[0]).abs() < 0.5);
}

fn panel(c: &mut Context, open: &mut bool) -> Rect {
    let mut rect = Rect::default();
    c.run(|c| {
        Window::new("Panel").show(c, |ui| {
            rect = ui.label("content").rect;
        });
        Root::new().show(c, |ui| {
            Modal::new("m").show(ui, open, |ui| {
                ui.label("modal");
            });
        });
    });
    rect
}

#[test]
fn window_move_ends_when_a_modal_opens() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 400), 1.0);
    let mut open = false;
    let start = panel(&mut c, &mut open);
    let grab = vec2(start.min.x + 20.0, start.min.y - 40.0);
    c.move_pointer(grab);
    c.primary_button(ElementState::Pressed);
    panel(&mut c, &mut open);
    c.move_pointer(grab + vec2(20.0, 10.0));
    let moved = panel(&mut c, &mut open);
    assert_ne!(moved.min, start.min, "the title bar drags the panel");
    open = true;
    panel(&mut c, &mut open);
    assert!(c.capture.is_none());
    c.move_pointer(grab + vec2(120.0, 90.0));
    assert_eq!(
        panel(&mut c, &mut open).min,
        moved.min,
        "the panel no longer follows"
    );
    c.primary_button(ElementState::Released);
    assert_eq!(panel(&mut c, &mut open).min, moved.min);
}
