//! Usage diagnostics, builder value normalization and the opt-in debug overlay.
use super::*;
use crate::{
    DebugOverlay, Diagnostic, DiagnosticKind, Loader, Popup, Rect, Separator, Slider, Window,
};
use std::{cell::RefCell, rc::Rc};
use winit::{dpi::PhysicalSize, event::ElementState};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}
fn kinds(c: &Context) -> Vec<DiagnosticKind> {
    c.diagnostics().iter().map(|d| d.kind).collect()
}
fn healthy(c: &mut Context) {
    c.run(|c| {
        Window::new("Healthy").show(c, |ui| {
            ui.button("OK");
            ui.button("Cancel");
            for i in 0..3 {
                ui.push_id(i, |ui| {
                    ui.button("Row");
                });
            }
        });
    });
}

#[test]
fn a_correct_ui_reports_nothing() {
    let mut c = setup();
    healthy(&mut c);
    healthy(&mut c);
    assert!(c.diagnostics().is_empty(), "{:?}", c.diagnostics());
}

#[test]
fn two_widgets_with_one_id_are_reported_with_their_rect_and_the_report_clears_with_the_fix() {
    let mut c = setup();
    c.run(|c| {
        Window::new("Dup").show(c, |ui| {
            ui.button("Same");
            ui.button("Other");
        });
    });
    assert!(c.diagnostics().is_empty());
    c.run(|c| {
        Window::new("Dup").show(c, |ui| {
            // Two "Same" labels in one scope derive the same id... unless the id is explicit.
            ui.add(crate::Button::new("Same").id_source("x"));
            ui.add(crate::Button::new("Different").id_source("x"));
        });
    });
    let report = c
        .diagnostics()
        .iter()
        .find(|d| d.kind == DiagnosticKind::IdCollision)
        .expect("collision found");
    assert!(report.id.is_some() && report.rect.is_some_and(|r| !r.is_empty()));
    assert_eq!(c.diagnostics().len(), 1, "one report per colliding id");
    healthy(&mut c);
    assert!(
        c.diagnostics().is_empty(),
        "reports describe the last pass only"
    );
}

#[test]
fn a_control_with_no_room_is_a_layout_problem() {
    let mut c = setup();
    c.run(|c| {
        Window::new("Tight").show(c, |ui| {
            ui.with_max_width(0.0, |ui| ui.button("No room"));
        });
    });
    assert!(
        kinds(&c).contains(&DiagnosticKind::NoLayoutSpace),
        "{:?}",
        c.diagnostics()
    );
}

#[test]
fn a_scope_left_open_is_reported_and_repaired() {
    let mut c = setup();
    c.run(|c| {
        c.placements.outstanding = 1;
        c.visual_depth = 2;
    });
    assert!(kinds(&c).contains(&DiagnosticKind::UnbalancedScope));
    assert_eq!((c.placements.outstanding, c.visual_depth), (0, 0));
    healthy(&mut c);
    assert!(c.diagnostics().is_empty());
}

#[test]
fn a_popup_without_a_usable_anchor_does_not_open_and_says_why() {
    let mut c = setup();
    let mut open = true;
    c.run(|c| {
        Window::new("Pop").show(c, |ui| {
            let anchor = Rect {
                min: Vec2::new(f32::NAN, 0.0),
                max: Vec2::new(10.0, 10.0),
            };
            let shown = Popup::new("p", anchor).show(ui, &mut open, |ui| ui.label("x"));
            assert!(shown.is_none());
        });
    });
    assert!(!open);
    assert!(kinds(&c).contains(&DiagnosticKind::PopupWithoutAnchor));
    assert!(c.popup.is_none());
}

#[test]
fn invalid_builder_values_are_normalized_instead_of_panicking() {
    let mut c = setup();
    let mut value = 5.0_f32;
    let mut sized = None;
    c.run(|c| {
        Window::new("Bad")
            .offset(Vec2::new(f32::NAN, 0.0))
            .show(c, |ui| {
                sized = Some(
                    ui.add(
                        Slider::new(&mut value, 10.0..=0.0)
                            .width(f32::NAN)
                            .step(-1.0),
                    ),
                );
                ui.add(Separator::new().thickness(-3.0));
                ui.add(Loader::new().size(f32::INFINITY));
                ui.add(crate::Skeleton::new(f32::NAN));
            });
    });
    let slider = sized.unwrap();
    assert!(slider.rect.size().x.is_finite() && slider.rect.size().x > 0.0);
    assert!(
        (0.0..=10.0).contains(&value),
        "descending range was swapped: {value}"
    );
    let messages: Vec<_> = c
        .diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::InvalidValue)
        .map(|d| d.message.as_str())
        .collect();
    for needle in [
        "Slider::new",
        "Slider::width",
        "Slider::step",
        "Separator::thickness",
        "Loader::size",
        "Window::offset",
        "Skeleton::new",
    ] {
        assert!(
            messages.iter().any(|m| m.contains(needle)),
            "{needle} in {messages:?}"
        );
    }
    assert!(
        messages
            .iter()
            .all(|m| m.contains("interact") || m.contains(".rs")),
        "{messages:?}"
    );
}

#[test]
fn a_bad_value_is_reported_to_the_handler_once_not_every_frame() {
    let mut c = setup();
    let seen: Rc<RefCell<Vec<Diagnostic>>> = Rc::default();
    let sink = Rc::clone(&seen);
    c.set_diagnostic_handler(Some(Box::new(move |d| sink.borrow_mut().push(d.clone()))));
    for _ in 0..5 {
        c.run(|c| {
            Window::new("Bad").show(c, |ui| {
                ui.add(Separator::new().thickness(f32::NAN));
            });
        });
        assert_eq!(
            c.diagnostics().len(),
            1,
            "still queryable while it persists"
        );
    }
    assert_eq!(seen.borrow().len(), 1, "but the handler heard it only once");
    c.set_diagnostic_handler(None);
}

fn scene(c: &mut Context, with_issue: bool) {
    c.run(|c| {
        Window::new("Scene").show(c, |ui| {
            ui.button("A");
            if with_issue {
                ui.add(crate::Button::new("B").id_source("A"));
                ui.add(crate::Button::new("C").id_source("A"));
            }
        });
    });
}

#[test]
fn issues_with_the_overlay_off_cost_no_geometry_and_never_churn_the_cache() {
    let mut c = setup();
    scene(&mut c, true);
    scene(&mut c, true);
    let (before, vertices) = (c.cache_stats(), c.draw_data().vertices.len());
    scene(&mut c, true);
    let after = c.cache_stats();
    assert_eq!(after.tessellated_elements, before.tessellated_elements);
    assert_eq!(after.geometry_rebuilds, before.geometry_rebuilds);
    assert_eq!(c.draw_data().vertices.len(), vertices);
    assert!(!c.diagnostics().is_empty());
    assert!(
        c.draw_data().commands.len() <= 12,
        "no overlay layer exists"
    );
}

#[test]
fn the_overlay_draws_on_top_without_touching_layout_hits_or_other_elements() {
    let mut c = setup();
    scene(&mut c, true);
    scene(&mut c, true);
    let hits: Vec<_> = c
        .previous_hits
        .iter()
        .map(|h| (h.id, h.rect, h.clip))
        .collect();
    let vertices = c.draw_data().vertices.len();
    c.set_debug_overlay(DebugOverlay::ISSUES);
    scene(&mut c, true);
    let with: Vec<_> = c
        .previous_hits
        .iter()
        .map(|h| (h.id, h.rect, h.clip))
        .collect();
    assert_eq!(with, hits, "overlay does not move or add regions");
    assert!(
        c.draw_data().vertices.len() > vertices,
        "frame and message are drawn"
    );
    // Steady with the overlay on: only the overlay's own element may change.
    scene(&mut c, true);
    let stats = c.cache_stats();
    scene(&mut c, true);
    assert_eq!(
        c.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    // Turning it off restores the exact geometry.
    c.set_debug_overlay(DebugOverlay::OFF);
    scene(&mut c, true);
    scene(&mut c, true);
    assert_eq!(c.draw_data().vertices.len(), vertices);
    assert_eq!(c.debug_overlay(), DebugOverlay::OFF);
}

#[test]
fn bounds_clip_and_hit_regions_under_the_cursor_are_outlined() {
    let mut c = setup();
    scene(&mut c, false);
    let at = c
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::Activate)
        .unwrap()
        .rect
        .center();
    scene(&mut c, false);
    let plain = c.draw_data().vertices.len();
    c.move_pointer(at);
    for overlay in [
        DebugOverlay {
            bounds: true,
            ..DebugOverlay::OFF
        },
        DebugOverlay {
            clip: true,
            ..DebugOverlay::OFF
        },
        DebugOverlay {
            hits: true,
            ..DebugOverlay::OFF
        },
    ] {
        c.set_debug_overlay(overlay);
        scene(&mut c, false);
        assert!(c.draw_data().vertices.len() > plain, "{overlay:?}");
    }
    // Away from every region nothing is outlined.
    c.set_debug_overlay(DebugOverlay::ALL);
    c.move_pointer(Vec2::new(790.0, 590.0));
    scene(&mut c, false);
    assert_eq!(c.draw_data().vertices.len(), plain);
    let _ = ElementState::Pressed;
}

#[test]
fn external_failures_reach_the_diagnostics_between_passes() {
    let mut c = setup();
    healthy(&mut c);
    c.report(DiagnosticKind::External, None, None, || {
        "clipboard unavailable: test".into()
    });
    healthy(&mut c);
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::External && d.message.contains("clipboard")));
    healthy(&mut c);
    assert!(c.diagnostics().is_empty());
}
