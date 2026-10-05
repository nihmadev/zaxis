//! What is built, painted and scheduled: layers, clipping, DPI, images, reduced motion, idling.
use super::{Host, Rig};
use crate::prelude::*;
use std::time::Duration;
use zaxis::{Carousel, DiagnosticKind, Root};

#[test]
fn only_visible_layers_build_content_and_each_builds_once() {
    let mut rig = Rig::new(8);
    rig.settle();
    assert_eq!(
        rig.built,
        vec![0],
        "sheets at rest are blank: only the front card is built"
    );
    rig.focus();
    rig.page = 0;
    rig.key(KeyCode::ArrowRight);
    rig.tick(60);
    let mut built = rig.built.clone();
    built.sort_unstable();
    assert_eq!(
        built,
        vec![0, 1],
        "the leaving and the rising card, once each"
    );
    rig.settle();
    assert_eq!(rig.built, vec![1]);
}

#[test]
fn images_build_overlays_for_the_active_slide_only() {
    let mut rig = Rig::new(10);
    rig.images = true;
    rig.settle();
    assert_eq!(rig.built, vec![0]);
    rig.page = 5;
    rig.tick(40);
    assert!(rig.built.len() <= 2, "{:?}", rig.built);
    rig.settle();
    assert_eq!(rig.built, vec![5]);
}

#[test]
fn a_settled_carousel_requests_no_frames_and_reuses_geometry() {
    let mut rig = Rig::new(4);
    rig.settle();
    assert!(!rig.c.needs_repaint_at(rig.now));
    assert!(rig.c.next_repaint().is_none() && !rig.c.wants_animation_frame());
    let tessellated = rig.c.cache_stats().tessellated_elements;
    for _ in 0..5 {
        rig.tick(16);
    }
    assert_eq!(rig.c.cache_stats().tessellated_elements, tessellated);
    // A running transition asks for frames and stops asking when it ends.
    rig.page = 2;
    rig.pass();
    assert!(rig.c.next_repaint().is_some() || rig.c.wants_animation_frame());
    rig.settle();
    assert!(!rig.c.needs_repaint_at(rig.now));
}

#[test]
fn reduced_motion_jumps_to_the_page_and_stops_asking_for_frames() {
    let mut rig = Rig::new(4);
    rig.reduced();
    rig.focus();
    let out = rig.key(KeyCode::ArrowRight);
    assert!(out.changed);
    rig.tick(16);
    assert!((rig.position() - 1.0).abs() < 1e-4, "{}", rig.position());
    assert!(rig.c.next_repaint().is_none() && !rig.c.wants_animation_frame());
}

#[test]
fn everything_painted_stays_inside_the_carousel() {
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(800, 600), 1.0);
    let viewport = c.viewport();
    let mut stage = Rect::default();
    let mut page = 0;
    let mut now = Instant::now();
    for step in 0..24 {
        now += Duration::from_millis(16);
        if step == 3 {
            page = 1;
        }
        c.run_at(now, |c| {
            Root::new().show(c, |ui| {
                ui.add_space(20.0);
                stage = Carousel::new("clip")
                    .pages(4)
                    .size(Vec2::new(360.0, 240.0))
                    .show(ui, &mut page, |ui, i| {
                        ui.button(format!("Card {i}"));
                    })
                    .response
                    .rect;
            });
        });
        let eps = 0.01;
        let grown = Rect::from_min_max(stage.min - Vec2::splat(eps), stage.max + Vec2::splat(eps));
        let mut inside = 0;
        for e in c.probe().elements {
            // The root's own panel is wider than the carousel; only carousel elements remain.
            if e.clip == viewport || e.clip.size().x > stage.size().x * 1.5 {
                continue;
            }
            inside += 1;
            assert!(
                e.clip.min.cmpge(grown.min).all() && e.clip.max.cmple(grown.max).all(),
                "clip {:?} leaves the carousel {stage:?} at step {step}",
                e.clip
            );
        }
        assert!(inside > 0);
    }
}

#[test]
fn page_content_is_clipped_to_a_rectangle_inside_the_rounded_card() {
    let mut rig = Rig::new(3);
    rig.settle();
    let state = &rig.c.probe().carousels[&rig.id()];
    let card = state.layers.iter().find(|(k, _)| *k == 0).unwrap().1;
    let arc = 18.0 * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
    let content: Vec<_> = rig
        .c
        .probe()
        .elements
        .iter()
        .filter(|e| e.clip.min.x > card.min.x + 1.0 && e.clip.max.x < card.max.x - 1.0)
        .collect();
    assert!(
        !content.is_empty(),
        "the card button is clipped to the content rectangle"
    );
    for e in content {
        assert!(e.clip.min.x >= card.min.x + arc - 0.01 && e.clip.min.y >= card.min.y + arc - 0.01);
        assert!(e.clip.max.x <= card.max.x - arc + 0.01 && e.clip.max.y <= card.max.y - arc + 0.01);
    }
}

#[test]
fn gestures_and_geometry_do_not_depend_on_the_display_scale() {
    let logical = Rig::with_scale(3, 1.0).rect().size();
    for scale in [1.0, 1.5, 2.0] {
        let mut rig = Rig::with_scale(3, scale);
        assert_eq!(rig.rect().size(), logical, "logical size at {scale}x");
        let start = rig.empty();
        rig.drag(start, Vec2::new(-100.0, 0.0), 6, 16);
        rig.tick(200);
        rig.release();
        rig.tick(16);
        rig.settle();
        assert_eq!(rig.page, 1, "swipe at {scale}x");
    }
}

#[test]
fn the_carousel_works_inside_a_card_and_a_scroll_area() {
    for host in [Host::Card, Host::Scroll] {
        let mut rig = Rig::new(3);
        rig.host = host;
        rig.settle();
        let start = rig.empty();
        rig.drag(start, Vec2::new(-100.0, 0.0), 6, 16);
        rig.tick(200);
        rig.release();
        rig.tick(16);
        rig.settle();
        assert_eq!(
            rig.page,
            1,
            "swipe inside {}",
            if host == Host::Card {
                "a card"
            } else {
                "a scroll area"
            }
        );
    }
}

#[test]
fn a_vertical_wheel_scrolls_the_enclosing_area_and_a_horizontal_one_turns_the_page() {
    let mut rig = Rig::new(3);
    rig.host = Host::Scroll;
    rig.settle();
    rig.c.move_pointer(rig.empty());
    rig.tick(16);
    assert!(rig.c.scroll_wheel(Vec2::new(0.0, 40.0)), "the area scrolls");
    rig.tick(16);
    assert_eq!(rig.page, 0);
    let at = rig.empty();
    rig.c.move_pointer(at - Vec2::new(0.0, 40.0));
    rig.tick(16);
    assert!(rig.c.scroll_wheel(Vec2::new(14.0, 0.0)));
    assert!(rig.tick(16).changed);
}

#[test]
fn photos_show_a_skeleton_then_fade_in_and_idle() {
    let mut rig = Rig::new(3);
    rig.images = true;
    rig.pass();
    assert!(
        rig.c.wants_animation_frame() || rig.c.next_repaint().is_some(),
        "a photo that is still loading animates its Skeleton"
    );
    let deadline = zaxis::Instant::now() + Duration::from_secs(10);
    while zaxis::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        rig.tick(16);
        if rig.c.image_metrics().pending_jobs == 0 && !rig.c.needs_repaint_at(rig.now) {
            break;
        }
    }
    rig.settle();
    assert!(rig.c.diagnostics().is_empty(), "{:?}", rig.c.diagnostics());
    assert!(rig.c.image_state(&rig.photos[0]).is_ready());
    assert!(!rig.c.needs_repaint_at(rig.now));
}

#[test]
fn a_photo_that_fails_to_load_is_reported_and_the_carousel_keeps_working() {
    let mut rig = Rig::new(3);
    rig.images = true;
    rig.broken = true;
    let deadline = zaxis::Instant::now() + Duration::from_secs(10);
    while zaxis::Instant::now() < deadline
        && !rig
            .c
            .diagnostics()
            .iter()
            .any(|d| d.kind == DiagnosticKind::External)
    {
        std::thread::sleep(Duration::from_millis(5));
        rig.tick(16);
    }
    assert!(rig
        .c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::External));
    rig.focus();
    assert!(rig.key(KeyCode::ArrowRight).changed);
    rig.settle();
    assert_eq!(rig.page, 1);
}

#[test]
fn a_disabled_carousel_ignores_input() {
    use zaxis::CarouselPage;
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(800, 600), 1.0);
    let mut page = 0;
    let run = |c: &mut Context, page: &mut usize| {
        let mut rect = Rect::default();
        c.run(|c| {
            zaxis::Window::new("Off").show(c, |ui| {
                rect = Carousel::new("off")
                    .pages(3)
                    .enabled(false)
                    .size(Vec2::new(300.0, 200.0))
                    .show(ui, CarouselPage::Index(page), |ui, i| {
                        ui.label(format!("{i}"));
                    })
                    .response
                    .rect;
            });
        });
        rect
    };
    run(&mut c, &mut page);
    let rect = run(&mut c, &mut page);
    let from = rect.center();
    c.move_pointer(from);
    c.primary_button(winit::event::ElementState::Pressed);
    c.move_pointer(from - Vec2::new(120.0, 0.0));
    run(&mut c, &mut page);
    c.primary_button(winit::event::ElementState::Released);
    run(&mut c, &mut page);
    assert_eq!(page, 0);
}
