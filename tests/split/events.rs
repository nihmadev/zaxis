use super::*;
use crate::{
    Color, CornerRadius, Padding, Rect, Root, ScrollArea, Shape, SplitHandle, SplitHandleStyle,
    SplitOutput, SplitPane, SplitPanel, SplitSize, SplitStyle, SplitSurface, TextEdit, Transform,
};
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};
fn draw(c: &mut Context, panels: Vec<SplitPanel>) -> SplitOutput<()> {
    let mut output = None;
    c.run(|c| {
        Root::new().padding(Padding::all(0.0)).show(c, |ui| {
            output = Some(
                SplitPane::horizontal("test")
                    .panels(panels)
                    .show(ui, |_| {}),
            );
        })
    });
    output.unwrap()
}
fn specs() -> Vec<SplitPanel> {
    (0..3).map(|i| SplitPanel::new(i).min_size(30.0)).collect()
}
#[test]
fn captured_drag_release_and_idle_cache() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 300), 1.0);
    let before = draw(&mut c, specs());
    let p = before.boundaries[0].bounds.center();
    c.move_pointer(p);
    assert!(c.primary_button(ElementState::Pressed));
    c.move_pointer(p + Vec2::new(60.0, 900.0));
    let after = draw(&mut c, specs());
    assert!(after.resize_started && after.changed);
    assert_eq!(after.panels[2].size, before.panels[2].size);
    assert!((after.panels[0].size - before.panels[0].size - 60.0).abs() < 0.01);
    assert_eq!(c.cursor_icon(), winit::window::CursorIcon::EwResize);
    c.primary_button(ElementState::Released);
    let released = draw(&mut c, specs());
    assert!(released.resize_ended);
    assert!((released.panels[0].size - after.panels[0].size).abs() < 0.01);
    draw(&mut c, specs());
    let stats = c.cache_stats();
    let revision = c.draw_data().revision;
    draw(&mut c, specs());
    assert_eq!(
        stats.tessellated_elements,
        c.cache_stats().tessellated_elements
    );
    assert_eq!(revision, c.draw_data().revision);
    assert!(!c.needs_repaint());
}

#[test]
fn batched_events_keyboard_focus_reset_and_nested_text() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 300), 1.0);
    let out = draw(&mut c, specs());
    let p = out.boundaries[0].bounds.center();
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(p + Vec2::new(50.0, 0.0));
    c.primary_button(ElementState::Released);
    let out = draw(&mut c, specs());
    assert!(out.resize_started && out.resize_ended);
    let before = out.panels[0].size;
    assert!(c.key(KeyCode::ArrowRight, ElementState::Pressed, false));
    assert_eq!(draw(&mut c, specs()).panels[0].size, before + 4.0);
    c.input.modifiers = winit::keyboard::ModifiersState::SHIFT;
    assert!(c.key(KeyCode::ArrowLeft, ElementState::Pressed, false));
    assert_eq!(draw(&mut c, specs()).panels[0].size, before - 20.0);
    c.key(KeyCode::Home, ElementState::Pressed, false);
    let reset = draw(&mut c, specs());
    assert!((reset.panels[0].size - reset.panels[1].size).abs() < 0.01);
    c.input.modifiers = Default::default();
    c.set_focus(None);
    c.key(KeyCode::Tab, ElementState::Pressed, false);
    let focused = draw(&mut c, specs());
    assert!(focused.boundaries[0].focused);
    assert!(c.focus_visible(focused.boundaries[0].id));
    let mut text = String::from("abc");
    let mut edit = None;
    c.run(|c| {
        Root::new().show(c, |ui| {
            ui.split_horizontal("nested", specs(), |split| {
                split.panel(0, |ui| {
                    edit = Some(ui.add(TextEdit::new(&mut text)));
                });
            });
        })
    });
    c.request_focus(edit.unwrap().id);
    assert!(c.key(KeyCode::ArrowRight, ElementState::Pressed, false));
    assert!(c.split_input.is_empty());
}

#[test]
fn topology_ids_focus_viewport_and_fixed_sizes() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 300), 1.0);
    let specs = || {
        vec![
            SplitPanel::new(0).default_size(SplitSize::Pixels(100.0)),
            SplitPanel::new(1),
            SplitPanel::new(2),
            SplitPanel::new(3),
        ]
    };
    let before = draw(&mut c, specs());
    let p = before.boundaries[1].bounds.center();
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(p + Vec2::new(50.0, 0.0));
    c.primary_button(ElementState::Released);
    let resized = draw(&mut c, specs());
    let ratio = resized.panels[1].size / resized.panels[2].size;
    let focus = resized.boundaries[1].id;
    c.set_viewport(PhysicalSize::new(900, 400), 1.0);
    let grown = draw(&mut c, specs());
    assert_eq!(grown.panels[0].size, 100.0);
    assert!((grown.panels[1].size / grown.panels[2].size - ratio).abs() < 0.0001);
    let mut reordered = specs();
    reordered.rotate_left(1);
    let reordered = draw(&mut c, reordered);
    assert_eq!(reordered.boundaries[0].id, focus);
    assert!(reordered.boundaries[0].focused);
    assert!((reordered.panels[0].size - grown.panels[1].size).abs() < 0.01);
    let mut added = specs();
    added.push(SplitPanel::new(4));
    let added = draw(&mut c, added);
    assert_eq!(added.panels.len(), 5);
    assert_eq!(added.panels[0].size, 100.0);
    let p = added.boundaries[1].bounds.center();
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    draw(&mut c, specs());
    let removed = draw(
        &mut c,
        vec![SplitPanel::new(0), SplitPanel::new(1), SplitPanel::new(3)],
    );
    assert!(removed.resize_ended && c.capture.is_none());
    assert!(!removed.boundaries.iter().any(|b| b.id == focus));
}

#[test]
fn controlled_values_rebase_drag_and_propose_without_feedback() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 300), 1.0);
    let specs = |size| {
        vec![
            SplitPanel::new(0).size(SplitSize::Pixels(size)),
            SplitPanel::new(1),
            SplitPanel::new(2),
        ]
    };
    let out = draw(&mut c, specs(100.0));
    let p = out.boundaries[0].bounds.center();
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(p + Vec2::new(30.0, 0.0));
    assert_eq!(draw(&mut c, specs(100.0)).panels[0].size, 130.0);
    // Accept the proposed value, then consume the next movement.
    c.move_pointer(p + Vec2::new(40.0, 0.0));
    assert_eq!(draw(&mut c, specs(130.0)).panels[0].size, 140.0);
    // An unrelated external edit takes authority and rebases at the previous pointer.
    assert_eq!(draw(&mut c, specs(200.0)).panels[0].size, 200.0);
    c.move_pointer(p + Vec2::new(50.0, 0.0));
    assert_eq!(draw(&mut c, specs(200.0)).panels[0].size, 210.0);
    c.primary_button(ElementState::Released);
    draw(&mut c, specs(210.0));
    assert_eq!(draw(&mut c, specs(180.0)).panels[0].size, 180.0);
}

#[test]
fn disabled_cancellation_focus_loss_and_component_removal() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 300), 1.0);
    let out = draw(&mut c, specs());
    let p = out.boundaries[0].bounds.center();
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    draw(&mut c, specs());
    c.key(KeyCode::Escape, ElementState::Pressed, false);
    let out = draw(&mut c, specs());
    assert!(out.resize_ended && out.boundaries[0].cancelled);
    c.primary_button(ElementState::Released);
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    draw(&mut c, specs());
    c.on_window_event(&winit::event::WindowEvent::Focused(false));
    assert!(draw(&mut c, specs()).resize_ended);
    assert!(c.capture.is_none());
    c.on_window_event(&winit::event::WindowEvent::Focused(true));
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    draw(&mut c, specs());
    let mut disabled = specs();
    disabled[0].resizable_after = false;
    let out = draw(&mut c, disabled);
    assert!(out.resize_ended);
    assert!(!out.boundaries[0].enabled);
    assert!(c.capture.is_none());
    c.primary_button(ElementState::Released);
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    draw(&mut c, specs());
    c.run(|c| Root::new().show(c, |_| {}));
    assert!(c.capture.is_none());
    assert!(c.splits.is_empty());
}

#[test]
fn tiny_empty_and_single_panels_and_edge_hit_priority() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(4, 2), 1.0);
    let out = draw(&mut c, specs());
    assert!(out.panels.iter().all(|p| p.size == 0.0));
    assert!(out
        .boundaries
        .iter()
        .all(|b| b.bounds.min.x >= 0.0 && b.bounds.max.x <= 4.0));
    assert!(draw(&mut c, vec![]).panels.is_empty());
    assert_eq!(draw(&mut c, vec![SplitPanel::new(0)]).panels[0].size, 4.0);
    c.set_viewport(PhysicalSize::new(0, 0), 1.0);
    assert!(draw(&mut c, specs())
        .panels
        .iter()
        .all(|p| p.bounds.is_empty()));
    c.set_viewport(PhysicalSize::new(600, 300), 1.0);
    let mut out = None;
    let mut button = None;
    c.run(|c| {
        Root::new().padding(Padding::all(0.0)).show(c, |ui| {
            out = Some(
                SplitPane::horizontal("edge")
                    .gap(0.0)
                    .handle(
                        SplitHandleStyle::default()
                            .kind(SplitHandle::PanelEdge)
                            .hit_width(10.0),
                    )
                    .panels(specs())
                    .show(ui, |s| {
                        s.panel(0, |ui| {
                            button = Some(ui.button("Button outside edge zone"));
                            let bounds = ui.allocate_space(Vec2::new(1000.0, 1000.0));
                            ui.context.register_hit(HitRegion {
                                id: Id::new("overflow"),
                                window: Id::new("zaxis-root"),
                                rect: bounds,
                                clip: ui.clip_rect(),
                                action: HitAction::Activate,
                            });
                        })
                    }),
            );
        })
    });
    let out = out.unwrap();
    let handle = out.boundaries[0].bounds;
    assert_eq!(handle.max.x, out.panels[0].bounds.max.x);
    assert_eq!(
        c.hit_test(handle.center()).unwrap().id,
        out.boundaries[0].id
    );
    assert_eq!(
        c.hit_test(button.unwrap().rect.center()).unwrap().action,
        HitAction::Activate
    );
    assert!(c
        .previous_hits
        .iter()
        .filter(|h| h.id == Id::new("overflow"))
        .all(|h| h.clip.max.x <= out.panels[0].content_bounds.max.x));
}

#[test]
fn dpi_transforms_scroll_clipping_and_nested_axes() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(1200, 800), 2.0);
    let surface = SplitSurface::default()
        .corner_radius(CornerRadius {
            top_left: 30.0,
            top_right: 10.0,
            bottom_left: 18.0,
            bottom_right: 2.0,
        })
        .padding(Padding::all(0.0));
    let transform = Transform::around(Vec2::ZERO, 0.8, Vec2::new(10.0, 20.0));
    let mut out = None;
    let mut nested = None;
    let draw = |c: &mut Context,
                out: &mut Option<SplitOutput<()>>,
                nested: &mut Option<SplitOutput<()>>| {
        c.run(|c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                ui.visual("scaled", transform, 1.0, |ui| {
                    *out = Some(
                        SplitPane::horizontal("visual")
                            .size(Vec2::new(500.0, 350.0))
                            .style(SplitStyle::default().panel(surface))
                            .panels(specs())
                            .show(ui, |s| {
                                s.panel(0, |ui| {
                                    ui.paint(Shape::rect(
                                        Rect::from_min_size(Vec2::ZERO, Vec2::splat(5000.0)),
                                        Color::WHITE,
                                    ));
                                    ScrollArea::vertical().show(ui, |ui| {
                                        for i in 0..20 {
                                            ui.button(format!("Button {i}"));
                                        }
                                    });
                                });
                                s.panel(1, |ui| {
                                    *nested = Some(
                                        SplitPane::vertical("nested")
                                            .panels(specs())
                                            .show(ui, |_| {}),
                                    );
                                });
                            }),
                    );
                });
            })
        });
    };
    draw(&mut c, &mut out, &mut nested);
    let bounds = transform.rect(out.as_ref().unwrap().panels[0].content_bounds);
    let white: Vec<_> = c
        .elements
        .iter()
        .filter(|e| {
            c.cache
                .get(&e.id)
                .is_some_and(|p| format!("{:?}", p.paint).contains("255, 255, 255"))
        })
        .collect();
    assert!(!white.is_empty());
    assert!(white.iter().all(|e| e.clip.intersect(bounds) == e.clip));
    let bid = nested.as_ref().unwrap().boundaries[0].id;
    let hit = *c.previous_hits.iter().find(|h| h.id == bid).unwrap();
    c.move_pointer(hit.rect.center());
    c.primary_button(ElementState::Pressed);
    c.move_pointer(hit.rect.center() + Vec2::new(0.0, 24.0));
    let old = nested.as_ref().unwrap().panels[0].size;
    draw(&mut c, &mut out, &mut nested);
    assert!((nested.as_ref().unwrap().panels[0].size - old - 30.0).abs() < 0.02);
    assert_eq!(c.cursor_icon(), winit::window::CursorIcon::NsResize);
}
