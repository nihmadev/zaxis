//! Swipes, flings, rubber band, clicks on layers and interrupted transitions.
use super::{Rig, SIZE};
use crate::prelude::*;

fn left(rig: &Rig) -> Vec2 {
    rig.empty()
}

#[test]
fn a_swipe_follows_the_pointer_and_commits_once_on_release() {
    let mut rig = Rig::new(4);
    let start = left(&rig);
    rig.drag(start, Vec2::new(-90.0, 0.0), 6, 16);
    let mid = rig.out.unwrap();
    assert!(mid.dragging && !mid.changed);
    assert_eq!(mid.page, 0, "nothing is decided before the release");
    assert!(
        rig.position() > 0.2,
        "the card follows the pointer: {}",
        rig.position()
    );
    // Hold still, then release: the distance alone decides.
    rig.tick(200);
    rig.release();
    let released = rig.tick(16);
    assert!(released.changed && released.page == 1 && !released.dragging);
    rig.settle();
    assert_eq!((rig.page, rig.changes), (1, 1), "one change for one action");
    assert!((rig.position() - 1.0).abs() < 1e-3);
}

#[test]
fn a_short_slow_drag_springs_back_without_a_change() {
    let mut rig = Rig::new(4);
    let start = left(&rig);
    rig.drag(start, Vec2::new(-20.0, 0.0), 4, 16);
    rig.tick(200);
    rig.release();
    rig.settle();
    assert_eq!((rig.page, rig.changes), (0, 0));
    assert!(rig.position().abs() < 1e-3);
}

#[test]
fn a_quick_flick_turns_the_page_with_the_gesture_velocity() {
    let mut rig = Rig::new(4);
    let start = left(&rig);
    // 30 px in 48 ms is far below the commit distance but fast.
    rig.drag(start, Vec2::new(-30.0, 0.0), 3, 16);
    rig.release();
    let out = rig.tick(16);
    assert!(out.changed && out.page == 1, "a fling carries on: {out:?}");
    // The spring starts from the release velocity: the first frames keep moving forward.
    let first = rig.position();
    rig.tick(16);
    assert!(rig.position() > first);
    rig.settle();
    assert_eq!(rig.page, 1);
}

#[test]
fn a_flick_moves_one_page_at_most() {
    let mut rig = Rig::new(6);
    let start = left(&rig);
    rig.drag(start, Vec2::new(-300.0, 0.0), 3, 8);
    rig.release();
    rig.tick(8);
    rig.settle();
    assert_eq!(rig.page, 1);
}

#[test]
fn the_edge_of_a_non_looping_carousel_gives_like_rubber_and_springs_back() {
    let mut rig = Rig::new(3);
    let start = left(&rig);
    rig.drag(start, Vec2::new(200.0, 0.0), 8, 16);
    let pulled = rig.position();
    assert!(pulled < -0.05, "the first card is pulled right: {pulled}");
    assert!(pulled > -0.8, "with resistance: {pulled}");
    rig.tick(200);
    rig.release();
    let out = rig.tick(16);
    assert!(!out.changed && out.page == 0);
    rig.settle();
    assert!(rig.position().abs() < 1e-3);
    assert_eq!(rig.changes, 0);
}

#[test]
fn dragging_backwards_returns_the_page_that_left() {
    let mut rig = Rig::new(3);
    rig.page = 2;
    rig.pass();
    rig.settle();
    let start = left(&rig);
    rig.drag(start, Vec2::new(150.0, 0.0), 6, 16);
    rig.tick(200);
    rig.release();
    let out = rig.tick(16);
    assert!(out.changed && out.page == 1);
    rig.settle();
    assert_eq!(rig.page, 1);
}

#[test]
fn clicking_a_sheet_goes_to_that_page_and_clicking_the_front_card_activates_it() {
    let mut rig = Rig::new(4);
    rig.settle();
    // The second sheet peeks out below the front card.
    let layers = rig.c.probe().carousels[&rig.id()].layers.clone();
    let (k, sheet) = layers.iter().rev().find(|(k, _)| *k == 2).copied().unwrap();
    let front = layers.iter().find(|(k, _)| *k == 0).unwrap().1;
    assert_eq!(k, 2);
    let peek = Vec2::new(sheet.center().x, (front.max.y + sheet.max.y) * 0.5);
    assert!(
        sheet.contains(peek) && !front.contains(peek),
        "{sheet:?} {front:?}"
    );
    let out = rig.click(peek);
    assert!(out.changed && out.page == 2 && out.activated.is_none());
    rig.settle();
    let at = rig.empty();
    let out = rig.click(at);
    assert!(!out.changed && out.activated == Some(2));
}

#[test]
fn controls_on_the_front_card_work_and_do_not_start_a_swipe() {
    let mut rig = Rig::new(3);
    rig.settle();
    let button = rig
        .c
        .probe()
        .previous_hits
        .iter()
        .find(|h| {
            h.rect.size().x < 200.0 && h.rect.size().y < 60.0 && h.rect.min.y > rig.rect().min.y
        })
        .map(|h| h.rect.center())
        .expect("the card button has a hit region");
    rig.drag(button, Vec2::new(-120.0, 0.0), 6, 16);
    assert!(
        !rig.out.unwrap().dragging,
        "a pressed button owns the gesture"
    );
    rig.release();
    rig.tick(16);
    assert_eq!(rig.page, 0);
}

#[test]
fn a_transition_can_be_interrupted_without_a_jump() {
    let mut rig = Rig::new(5);
    rig.focus();
    rig.key(KeyCode::ArrowRight);
    rig.tick(90);
    let before = rig.position();
    assert!(before > 0.05 && before < 0.95, "in flight: {before}");
    rig.key(KeyCode::ArrowRight);
    let after = rig.position();
    assert!(
        (after - before).abs() < 0.2,
        "continues from where it was: {before} -> {after}"
    );
    rig.settle();
    assert_eq!(rig.page, 2);
    assert_eq!(rig.changes, 2);
    let _ = SIZE;
}

#[test]
fn a_card_on_its_way_in_looks_enabled_but_takes_no_input() {
    let mut rig = Rig::new(4);
    rig.focus();
    rig.key(KeyCode::ArrowRight);
    rig.tick(60);
    assert!(rig.built.len() == 2, "two cards in flight: {:?}", rig.built);
    assert!(
        rig.looked.iter().all(|enabled| *enabled),
        "greyed-out text that turns dark on arrival reads as a flicker: {:?} {:?}",
        rig.built,
        rig.looked
    );
    // Only the card we are heading to has live hit regions.
    let hits = rig
        .c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| {
            h.rect.size().x < 200.0
                && h.rect.size().y < 60.0
                && rig.rect().contains(h.rect.center())
        })
        .count();
    assert!(hits <= 1, "{hits} buttons accept input");
    rig.settle();
}

/// Centers of the indicator items, left to right, from the regions the last pass registered.
fn dots(rig: &Rig) -> Vec<Vec2> {
    let pages = rig.rect();
    let mut dots: Vec<_> = (rig.c.probe().previous_hits.iter())
        .filter(|h| h.rect.min.y >= pages.max.y && h.rect.size().max_element() < 60.0)
        .map(|h| h.rect)
        .collect();
    dots.sort_by(|a, b| a.min.x.total_cmp(&b.min.x));
    for pair in dots.windows(2) {
        assert!(
            pair[0].max.x <= pair[1].min.x + 1e-3,
            "targets overlap: {pair:?}"
        );
    }
    dots.iter().map(|r| r.center()).collect()
}

#[test]
fn every_indicator_item_goes_to_its_own_page() {
    for start in [0, 2, 4] {
        for target in 0..5 {
            let mut rig = Rig::new(5);
            rig.page = start;
            rig.settle();
            let at = dots(&rig)[target];
            rig.click(at);
            rig.tick(16);
            assert_eq!(rig.page, target, "item {target} pressed on page {start}");
            rig.settle();
        }
    }
}

#[test]
fn a_jump_of_several_pages_does_not_flash_the_cards_in_between() {
    for looping in [false, true] {
        let mut rig = Rig::new(6);
        rig.looping = looping;
        rig.settle();
        let at = dots(&rig)[3];
        rig.click(at);
        let mut shown = std::collections::BTreeSet::new();
        for _ in 0..60 {
            rig.tick(16);
            shown.extend(rig.built.iter().copied());
        }
        assert_eq!(rig.page, 3);
        assert!(
            shown.iter().all(|page| *page == 0 || *page == 3),
            "cards 1 and 2 must not pass through the front (looping {looping}): {shown:?}"
        );
        assert!((rig.position() - 3.0).abs() < 1e-3 || looping);
    }
}
