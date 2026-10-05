//! The page model: bindings, keys, looping, autoplay and degenerate input.
use super::{Rig, SIZE};
use crate::prelude::*;
use std::time::Duration;
use winit::dpi::PhysicalSize;
use zaxis::{Carousel, CarouselPage, DiagnosticKind, Id, Window};

#[test]
fn assigning_the_page_from_outside_animates_and_reports_no_change() {
    let mut rig = Rig::new(5);
    rig.settle();
    rig.page = 3;
    let out = rig.pass();
    assert!(!out.changed && out.page == 3);
    rig.tick(80);
    let position = rig.position();
    assert!(position > 0.0 && position < 3.0, "on its way: {position}");
    rig.settle();
    assert!((rig.position() - 3.0).abs() < 1e-3);
    assert_eq!((rig.page, rig.changes), (3, 0));
}

#[test]
fn an_index_past_the_end_is_clamped_with_a_diagnostic() {
    let mut rig = Rig::new(3);
    rig.page = 9;
    let out = rig.pass();
    assert_eq!((out.page, rig.page), (2, 2));
    assert!(rig
        .c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidValue));
}

#[test]
fn a_looping_carousel_wraps_both_ways_and_folds_its_position_at_rest() {
    let mut rig = Rig::new(3);
    rig.looping = true;
    rig.focus();
    for expected in [1, 2, 0, 1] {
        assert!(rig.key(KeyCode::ArrowRight).changed);
        assert_eq!(rig.page, expected);
        rig.settle();
    }
    assert!(rig.key(KeyCode::ArrowLeft).changed && rig.page == 0);
    assert!(rig.key(KeyCode::ArrowLeft).changed && rig.page == 2);
    rig.settle();
    let state = &rig.c.probe().carousels[&rig.id()];
    assert!(
        (0..3).contains(&state.target),
        "folded back: {}",
        state.target
    );
    assert!((rig.position() - 2.0).abs() < 1e-3);
}

#[test]
fn a_swipe_wraps_in_a_looping_carousel_instead_of_stretching() {
    let mut rig = Rig::new(3);
    rig.looping = true;
    let start = rig.empty();
    rig.drag(start, Vec2::new(160.0, 0.0), 6, 16);
    rig.tick(200);
    rig.release();
    let out = rig.tick(16);
    assert!(out.changed && out.page == 2);
    rig.settle();
}

fn keyed(
    c: &mut Context,
    now: Instant,
    keys: &[u32],
    key: &mut Id,
    seen: &mut Vec<u32>,
) -> zaxis::CarouselOutput {
    let mut out = None;
    c.run_at(now, |c| {
        Window::new("Keyed").show(c, |ui| {
            out = Some(
                Carousel::new("k")
                    .keys(keys.iter().copied())
                    .size(SIZE)
                    .show(ui, CarouselPage::Key(key), |ui, i| {
                        seen.push(keys[i]);
                        ui.button(format!("Card {}", keys[i]));
                    }),
            );
        });
    });
    out.unwrap()
}

#[test]
fn inserting_and_removing_pages_keeps_the_keyed_page_and_the_motion() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut now = Instant::now();
    let mut key = Id::new(30u32);
    let mut seen = Vec::new();
    let mut keys = vec![10, 20, 30, 40];
    for _ in 0..3 {
        now += Duration::from_millis(16);
        keyed(&mut c, now, &keys, &mut key, &mut seen);
    }
    assert_eq!(c.probe().carousels.values().next().unwrap().page, 2);
    // Insert two pages in front: the keyed page moves to index 4 without a visible jump.
    keys.splice(0..0, [1, 2]);
    now += Duration::from_millis(16);
    let out = keyed(&mut c, now, &keys, &mut key, &mut seen);
    assert_eq!((out.page, out.key, out.changed), (4, Id::new(30u32), false));
    let state = c.probe().carousels.values().next().unwrap();
    assert!((state.position - 4.0).abs() < 1e-3, "{}", state.position);
    // Remove the page itself: the carousel moves to a neighbour and tells the binding.
    keys.retain(|k| *k != 30);
    now += Duration::from_millis(16);
    let out = keyed(&mut c, now, &keys, &mut key, &mut seen);
    assert_eq!(out.page, 4);
    assert_eq!(key, Id::new(40u32), "the binding follows");
    assert!(!out.changed, "a removal is not a user action");
}

#[test]
fn nothing_to_show_is_harmless_and_reported() {
    let mut rig = Rig::new(0);
    let out = rig.pass();
    assert_eq!((out.page, out.changed, out.dragging), (0, false, false));
    assert!(rig.built.is_empty());
    assert!(rig
        .c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidValue));
    assert_eq!(rig.state_pages(), 0);
    rig.pages = 3;
    rig.pass();
    assert_eq!(rig.state_pages(), 1, "a carousel comes back from empty");
}

#[test]
fn one_page_cannot_move_and_degenerate_sizes_do_not_panic() {
    let mut rig = Rig::new(1);
    rig.looping = true;
    rig.focus();
    assert!(!rig.key(KeyCode::ArrowRight).changed);
    for size in [
        Vec2::new(0.0, 0.0),
        Vec2::new(f32::NAN, 40.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(-5.0, 10.0),
    ] {
        let mut c = Context::new();
        c.set_viewport(PhysicalSize::new(800, 600), 1.0);
        for _ in 0..2 {
            c.run(|c| {
                Window::new("Tiny").show(c, |ui| {
                    let mut page = 0;
                    Carousel::new("t").pages(3).size(size).arrows(true).show(
                        ui,
                        &mut page,
                        |ui, i| {
                            ui.label(format!("{i}"));
                        },
                    );
                });
            });
        }
    }
}

#[test]
fn autoplay_turns_pages_on_a_timer_and_holds_for_hover_focus_and_drag() {
    let mut rig = Rig::new(4);
    rig.autoplay = Some(Duration::from_millis(1000));
    rig.rest();
    assert_eq!(rig.page, 0);
    let wake = rig
        .c
        .next_repaint()
        .expect("autoplay schedules a deadline, not frames");
    assert!(wake > rig.now && !rig.c.wants_animation_frame());
    rig.now = wake;
    let out = rig.pass();
    assert!(out.changed && out.autoplayed && rig.page == 1);
    rig.rest();
    // Hovering holds it; leaving restarts the whole interval.
    rig.c.move_pointer(rig.empty());
    rig.tick(16);
    rig.tick(5000);
    assert_eq!(rig.page, 1);
    rig.c.move_pointer(Vec2::new(790.0, 590.0));
    rig.tick(16);
    rig.tick(900);
    assert_eq!(rig.page, 1, "the interval starts again after the hold");
    rig.tick(300);
    assert_eq!(rig.page, 2);
    // Keyboard focus holds it, and so does a swipe in progress.
    rig.rest();
    rig.focus();
    rig.c.move_pointer(Vec2::new(790.0, 590.0));
    rig.tick(16);
    rig.tick(6000);
    assert_eq!(rig.page, 2);
    rig.c.set_focus(None);
    rig.tick(16);
    let start = rig.empty();
    rig.drag(start, Vec2::new(-10.0, 0.0), 2, 16);
    rig.tick(6000);
    assert_eq!(rig.page, 2, "no turn while the pointer holds the card");
}

#[test]
fn autoplay_stops_at_the_end_unless_it_loops_and_never_runs_with_reduced_motion() {
    let mut rig = Rig::new(2);
    rig.autoplay = Some(Duration::from_millis(500));
    rig.settle();
    rig.tick(600);
    rig.settle();
    rig.tick(600);
    rig.tick(600);
    assert_eq!(rig.page, 1, "a non-looping carousel stops at the end");
    let mut rig = Rig::new(3);
    rig.autoplay = Some(Duration::from_millis(300));
    rig.reduced();
    rig.settle();
    for _ in 0..10 {
        rig.tick(500);
    }
    assert_eq!(rig.page, 0, "reduced motion has no automatic motion");
    assert!(rig.c.next_repaint().is_none());
}
