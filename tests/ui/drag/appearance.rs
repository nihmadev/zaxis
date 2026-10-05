//! Preview, cursor, dimming, return animation, reduced motion, styling and layers.
use super::*;
use zaxis::{DragSource, DragStyle, Popup, Theme};

fn started(s: &mut Scene) {
    s.begin();
    assert!(s
        .context
        .probe()
        .drag
        .session
        .as_ref()
        .is_some_and(|d| d.started));
}

#[test]
fn snapshot_preview_follows_the_pointer_keeping_the_grab_point_and_dims_the_source() {
    let mut s = scene(1.0);
    started(&mut s);
    let session = s.context.probe().drag.session.as_ref().unwrap();
    let snapshot = session
        .snapshot
        .as_ref()
        .expect("snapshot captured on the begin pass");
    assert!(!snapshot.items.is_empty());
    let grab = session.grab;
    let layer = drag_preview_layer();
    let preview = |c: &Context| {
        let mut bounds: Option<Rect> = None;
        for e in c
            .probe()
            .elements
            .iter()
            .filter(|e| e.layer == layer && e.id != layer.with("card"))
        {
            for v in &e.mesh.vertices {
                let p = Vec2::from_array(v.position);
                bounds = Some(bounds.map_or(Rect::from_min_max(p, p), |b| {
                    Rect::from_min_max(b.min.min(p), b.max.max(p))
                }));
            }
        }
        bounds.unwrap()
    };
    for target in [Vec2::new(300.0, 200.0), Vec2::new(500.0, 420.0)] {
        s.move_to(target);
        s.frame();
        let at = preview(&s.context);
        // The button is the whole snapshot, so its box starts at pointer - grab.
        assert!(
            (at.min - (target - grab)).abs().max_element() < 1.5,
            "{at:?} {target:?}"
        );
    }
    // The source is dimmed through the shared visual path, and restored after.
    assert!(!s.context.probe().visual_meshes.is_empty());
    s.release();
    for _ in 0..40 {
        s.frame();
    }
    assert!(s.context.probe().visual_meshes.is_empty());
    assert!(!s.context.probe().elements.iter().any(|e| e.layer == layer));
}

#[test]
fn cursor_reports_whether_a_drop_is_possible() {
    let mut s = scene(1.0);
    started(&mut s);
    let icon = |s: &Scene| s.context.cursor_icon();
    assert_eq!(icon(&s), winit::window::CursorIcon::Grabbing);
    s.hover_at(s.seen.target_rect.center());
    assert_eq!(icon(&s), winit::window::CursorIcon::Move);
    s.hover_at(s.seen.foreign_rect.center());
    assert_eq!(icon(&s), winit::window::CursorIcon::NoDrop);
    s.hover_at(Vec2::new(700.0, 500.0));
    assert_eq!(icon(&s), winit::window::CursorIcon::Grabbing);
    s.release();
    assert_ne!(icon(&s), winit::window::CursorIcon::Grabbing);
}

#[test]
fn cancel_returns_the_preview_then_nothing_keeps_requesting_frames() {
    let mut s = scene(1.0);
    started(&mut s);
    s.key(KeyCode::Escape);
    s.frame();
    assert!(s.context.probe().drag.returning.is_some());
    assert!(s.context.wants_animation_frame());
    for _ in 0..40 {
        s.frame();
    }
    assert!(s.context.probe().drag.returning.is_none());
    assert!(!s.context.wants_animation_frame());
    assert!(s.context.next_repaint().is_none());
}

#[test]
fn reduced_motion_skips_the_return_and_fades_instantly() {
    let mut s = scene(1.0);
    let mut style = s.context.style().clone();
    style.motion.reduced_motion = true;
    s.context.set_style(style);
    s.frame();
    started(&mut s);
    s.hover_at(s.seen.target_rect.center());
    s.frame();
    assert!(
        !s.context.wants_animation_frame(),
        "indicator settles immediately"
    );
    s.key(KeyCode::Escape);
    s.frame();
    assert!(s.context.probe().drag.returning.is_none());
}

#[test]
fn settled_indicator_fade_requests_no_more_frames() {
    let mut s = scene(1.0);
    started(&mut s);
    s.hover_at(s.seen.target_rect.center());
    for _ in 0..30 {
        s.frame();
    }
    assert!(s.seen.acceptable);
    assert!(!s.context.wants_animation_frame());
    assert!(s.context.next_repaint().is_none());
    let revision = s.context.draw_data().revision;
    s.frame();
    assert_eq!(
        s.context.draw_data().revision,
        revision,
        "stationary drag is cached"
    );
}

#[test]
fn custom_preview_content_is_drawn_in_the_overlay_and_never_takes_input() {
    let mut s = scene(1.0);
    let mut shown = 0;
    let center = s.seen.source_rect.center();
    s.move_to(center);
    s.press();
    s.move_to(center + Vec2::new(0.0, 12.0));
    for _ in 0..3 {
        s.step += 1;
        let now = s.start + Duration::from_millis(s.step * 16);
        s.context.run_at(now, |c| {
            Root::new().show(c, |ui| {
                DragSource::new(Id::new("source"), Item(7))
                    .preview(|ui| {
                        shown += 1;
                        ui.button("Custom preview");
                    })
                    .show(ui, |ui| ui.button("Drag me"));
            });
        });
    }
    assert!(shown >= 2);
    let layer = drag_preview_layer();
    assert!(s.context.probe().popup_layers.contains(&layer));
    assert!(s
        .context
        .probe()
        .previous_hits
        .iter()
        .all(|h| h.window != layer || h.action == HitAction::Block));
    assert_eq!(
        s.context.probe().drag.custom_preview,
        s.context.probe().frame
    );
}

#[test]
fn style_priority_is_builder_then_style_then_default_and_themes_carry_it() {
    let mut s = scene(1.0);
    let start = s.seen.source_rect.center();
    // Default 5px: 9px of travel begins a drag. With a 20px style it does not.
    let mut style = s.context.style().clone();
    style.drag = DragStyle::default().threshold(20.0);
    s.context.set_style(style);
    s.frame();
    s.move_to(start);
    s.press();
    s.move_to(start + Vec2::new(0.0, 12.0));
    assert!(s.context.probe().drag.session.is_none());
    s.move_to(start + Vec2::new(0.0, 22.0));
    assert!(s.context.probe().drag.session.is_some());
    s.release();
    // A builder value beats the style.
    s.context.cancel_drag();
    let mut theme = Theme::dark();
    theme.overrides.drag = DragStyle::default().threshold(3.0);
    s.context.set_theme(theme);
    assert_eq!(s.context.style().drag.threshold, Some(3.0));
    assert_eq!(
        DragStyle::default()
            .threshold(2.0)
            .resolve(s.context.style())
            .threshold,
        2.0
    );
    assert_eq!(
        DragStyle::default().resolve(s.context.style()).threshold,
        3.0
    );
}

#[test]
fn popup_layers_win_and_lower_targets_stay_unreachable() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let start = Instant::now();
    let mut step = 0u64;
    let mut open = true;
    let mut rects = [Rect::default(); 3];
    let mut hovered = [false; 3];
    let mut frame =
        |c: &mut Context, open: &mut bool, rects: &mut [Rect; 3], hovered: &mut [bool; 3]| {
            step += 1;
            c.run_at(start + Duration::from_millis(step * 16), |c| {
                Root::new().padding(Padding::all(10.0)).show(c, |ui| {
                    let root = ui.drop_target(
                        Id::new("root"),
                        |_: &Item| true,
                        |ui| {
                            ui.allocate_space(Vec2::new(400.0, 200.0));
                        },
                    );
                    rects[0] = root.response.rect;
                    hovered[0] = root.hovering;
                    Popup::new(
                        "p",
                        Rect::from_min_size(Vec2::new(500.0, 300.0), Vec2::new(150.0, 20.0)),
                    )
                    .size(Vec2::new(200.0, 120.0))
                    .show(ui, open, |ui| {
                        let source = ui.drag_source(Id::new("s"), Item(1), |ui| ui.label("Source"));
                        rects[1] = source.response.rect;
                        let inner = ui.drop_target(
                            Id::new("inner"),
                            |_: &Item| true,
                            |ui| {
                                ui.allocate_space(Vec2::new(150.0, 30.0));
                            },
                        );
                        rects[2] = inner.response.rect;
                        hovered[2] = inner.hovering;
                    });
                });
            });
        };
    for _ in 0..3 {
        frame(&mut context, &mut open, &mut rects, &mut hovered);
    }
    let from = rects[1].center();
    context.move_pointer(from);
    context.primary_button(ElementState::Pressed);
    context.move_pointer(from + Vec2::new(0.0, 12.0));
    for _ in 0..3 {
        frame(&mut context, &mut open, &mut rects, &mut hovered);
    }
    assert!(context
        .probe()
        .drag
        .session
        .as_ref()
        .is_some_and(|d| d.started));
    context.move_pointer(rects[0].center());
    frame(&mut context, &mut open, &mut rects, &mut hovered);
    assert!(
        !hovered[0],
        "the popup owns the layer; the root target is out of reach"
    );
    context.move_pointer(rects[2].center());
    frame(&mut context, &mut open, &mut rects, &mut hovered);
    assert!(hovered[2]);
    context.primary_button(ElementState::Released);
    frame(&mut context, &mut open, &mut rects, &mut hovered);
    assert!(open, "a drag inside a popup does not dismiss it");
}

#[test]
fn passive_source_reports_clicks_and_hover_does_not_leak_into_a_drag() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let start = Instant::now();
    let mut step = 0u64;
    let mut rect = Rect::default();
    let (mut clicks, mut hovered_other) = (0, false);
    let mut frame = |c: &mut Context, clicks: &mut u32, other: &mut bool, rect: &mut Rect| {
        step += 1;
        c.run_at(start + Duration::from_millis(step * 16), |c| {
            Root::new().show(c, |ui| {
                let out = ui.drag_source(Id::new("label"), Item(1), |ui| ui.label("Plain label"));
                *rect = out.response.rect;
                *clicks += u32::from(out.response.clicked());
                *other = ui.button("Other").hovered;
            });
        });
    };
    for _ in 0..2 {
        frame(&mut context, &mut clicks, &mut hovered_other, &mut rect);
    }
    context.move_pointer(rect.center());
    context.primary_button(ElementState::Pressed);
    context.primary_button(ElementState::Released);
    frame(&mut context, &mut clicks, &mut hovered_other, &mut rect);
    assert_eq!(clicks, 1);
    context.move_pointer(rect.center());
    context.primary_button(ElementState::Pressed);
    context.move_pointer(rect.center() + Vec2::new(0.0, 14.0));
    frame(&mut context, &mut clicks, &mut hovered_other, &mut rect);
    frame(&mut context, &mut clicks, &mut hovered_other, &mut rect);
    let other = context
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::Activate)
        .unwrap()
        .rect;
    context.move_pointer(other.center());
    frame(&mut context, &mut clicks, &mut hovered_other, &mut rect);
    assert!(
        !hovered_other,
        "no hover feedback for other widgets while dragging"
    );
    context.primary_button(ElementState::Released);
    assert_eq!(clicks, 1, "a drag is not a click");
}
