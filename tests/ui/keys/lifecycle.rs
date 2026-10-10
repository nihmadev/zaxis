//! A held key, window focus, widgets that come and go, bounded queues and idle.

use super::support::*;
use crate::prelude::*;
use std::time::Duration;
use winit::event::ElementState;
use zaxis::{FocusGroup, KeyInterest};

fn two(
    c: &mut Context,
    interest: KeyInterest<'_>,
    a: &mut Vec<KeyEvent>,
    b: &mut Vec<KeyEvent>,
) -> (Response, Response) {
    let mut out = None;
    pass(c, |ui| {
        let first = pad(ui, "a", interest, a);
        let second = pad(ui, "b", interest, b);
        out = Some((first, second));
    });
    out.unwrap()
}

#[test]
fn a_release_after_focus_moved_still_belongs_to_the_widget_that_took_the_press() {
    let interest = KeyInterest::arrows().releases();
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, second) = two(&mut c, interest, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, interest, &mut a, &mut b);
    assert!(down(&mut c, KeyCode::ArrowRight));
    c.request_focus(second.id);
    assert!(
        up(&mut c, KeyCode::ArrowRight),
        "the release is consumed with its press"
    );
    two(&mut c, interest, &mut a, &mut b);
    let kinds: Vec<_> = a.iter().map(KeyEvent::is_release).collect();
    assert_eq!(
        kinds,
        [false, true],
        "press and release both reached the first widget"
    );
    assert!(b.is_empty(), "the second never saw half a gesture");
    assert_eq!(c.input_stats().keys_owned, 0, "nothing is held any more");
}

#[test]
fn a_press_after_focus_moved_is_the_new_widgets_and_starts_its_own_gesture() {
    let interest = KeyInterest::arrows();
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, second) = two(&mut c, interest, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, interest, &mut a, &mut b);
    down(&mut c, KeyCode::ArrowLeft);
    c.request_focus(second.id);
    two(&mut c, interest, &mut a, &mut b);
    assert!(
        down(&mut c, KeyCode::ArrowLeft),
        "a fresh press while the old one is still held"
    );
    up(&mut c, KeyCode::ArrowLeft);
    two(&mut c, interest, &mut a, &mut b);
    assert_eq!((a.len(), b.len()), (1, 1));
}

#[test]
fn losing_the_window_focus_releases_every_key_and_drops_what_waits() {
    let interest = KeyInterest::arrows().releases();
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, _) = two(&mut c, interest, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, interest, &mut a, &mut b);
    down(&mut c, KeyCode::ArrowRight);
    tap(&mut c, KeyCode::ArrowLeft);
    assert!(c.input_stats().keys_owned > 0 && c.input_stats().key_events_pending > 0);
    c.on_input(InputEvent::Focus(false));
    let stats = c.input_stats();
    assert_eq!((stats.keys_owned, stats.key_events_pending), (0, 0));
    assert!(c.input().keys_down.is_empty());
    assert!(
        !up(&mut c, KeyCode::ArrowRight),
        "the owner is gone: the release goes the usual way"
    );
    c.on_input(InputEvent::Focus(true));
    two(&mut c, interest, &mut a, &mut b);
    assert!(a.is_empty(), "nothing is delivered after the loss");
    assert!(
        !tap(&mut c, KeyCode::ArrowRight),
        "and focus is gone with the window's"
    );
}

#[test]
fn what_the_owner_did_not_take_lapses_after_one_pass() {
    let interest = KeyInterest::arrows();
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, _) = two(&mut c, interest, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, interest, &mut a, &mut b);
    tap(&mut c, KeyCode::ArrowDown);
    pass(&mut c, |_| {});
    assert_eq!(c.input_stats().key_events_pending, 0);
    two(&mut c, interest, &mut a, &mut b);
    assert!(
        a.is_empty(),
        "the pass without the widget dropped the event"
    );
}

#[test]
fn a_widget_that_vanishes_leaves_no_claim_and_no_events() {
    let interest = KeyInterest::arrows();
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, _) = two(&mut c, interest, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, interest, &mut a, &mut b);
    assert_eq!(c.input_stats().key_claims, 2);
    pass(&mut c, |_| {});
    assert_eq!(c.input_stats().key_claims, 0);
    assert!(!tap(&mut c, KeyCode::ArrowLeft), "no widget, no owner");
    assert_eq!(c.input_stats().key_events_pending, 0);
}

#[test]
fn an_input_storm_without_a_pass_keeps_the_queue_bounded_and_pairs_whole() {
    let interest = KeyInterest::arrows().releases();
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, _) = two(&mut c, interest, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, interest, &mut a, &mut b);
    for _ in 0..5000 {
        assert!(
            down(&mut c, KeyCode::ArrowRight),
            "dropped or not, the key is consumed"
        );
        assert!(up(&mut c, KeyCode::ArrowRight));
    }
    let stats = c.input_stats();
    assert!(stats.key_events_pending <= 1024 + 8, "{stats:?}");
    assert!(stats.key_events_dropped > 0);
    assert_eq!(stats.keys_owned, 0, "no half gesture left holding a key");
    two(&mut c, interest, &mut a, &mut b);
    assert!(!a.is_empty() && a.len() % 2 == 0);
    for pair in a.chunks(2) {
        assert!(
            pair[0].is_press() && pair[1].is_release(),
            "whole pairs only"
        );
    }
    assert!(
        c.diagnostics()
            .iter()
            .filter(|d| d.message.contains("queue full"))
            .count()
            <= 1,
        "one notice per overflow, not one per dropped event"
    );
    assert_eq!(c.input_stats().key_events_pending, 0);
}

#[test]
fn a_dropped_press_swallows_its_repeats_and_release_instead_of_leaking_them() {
    let interest = KeyInterest::arrows().repeats().releases();
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, _) = two(&mut c, interest, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, interest, &mut a, &mut b);
    for _ in 0..300 {
        down(&mut c, KeyCode::ArrowDown);
        up(&mut c, KeyCode::ArrowDown);
    }
    assert!(
        down(&mut c, KeyCode::ArrowUp),
        "queue full: this press is dropped"
    );
    assert!(
        c.on_input(key_input(KeyCode::ArrowUp, ElementState::Pressed, true))
            .consumed
    );
    assert!(
        up(&mut c, KeyCode::ArrowUp),
        "and so are its repeat and release, consumed"
    );
    two(&mut c, interest, &mut a, &mut b);
    assert!(
        a.iter().all(|e| e.code == KeyCode::ArrowDown),
        "no loose Up in the queue"
    );
}

#[test]
fn mounting_and_unmounting_widgets_keeps_every_table_bounded() {
    let interest = KeyInterest::arrows();
    let mut c = setup();
    for round in 0..200_u32 {
        let name = format!("w{round}");
        let mut log = Vec::new();
        let mut id = None;
        pass(&mut c, |ui| {
            FocusGroup::new(format!("g{round}")).show(ui, |ui| {
                id = Some(pad(ui, &name, interest, &mut log).id);
            });
        });
        c.request_focus(id.unwrap());
        tap(&mut c, KeyCode::ArrowLeft);
    }
    pass(&mut c, |_| {});
    let stats = c.input_stats();
    assert_eq!(stats.key_claims, 0);
    assert_eq!((stats.focus_groups, stats.focus_group_members), (0, 0));
    assert_eq!((stats.keys_owned, stats.key_events_pending), (0, 0));
}

#[test]
fn contexts_are_independent() {
    let interest = KeyInterest::arrows();
    let (mut one, mut two_) = (setup(), setup());
    let (mut a1, mut a2) = (Vec::new(), Vec::new());
    let build = |c: &mut Context, log: &mut Vec<KeyEvent>| {
        let mut out = None;
        pass(c, |ui| out = Some(pad(ui, "same", interest, log)));
        out.unwrap()
    };
    let r1 = build(&mut one, &mut a1);
    build(&mut two_, &mut a2);
    one.request_focus(r1.id);
    build(&mut one, &mut a1);
    assert!(tap(&mut one, KeyCode::ArrowRight));
    assert!(
        !tap(&mut two_, KeyCode::ArrowRight),
        "the other context has no focus"
    );
    build(&mut one, &mut a1);
    build(&mut two_, &mut a2);
    assert_eq!((a1.len(), a2.len()), (1, 0));
}

#[test]
fn delivery_asks_for_one_pass_and_idle_asks_for_none() {
    let interest = KeyInterest::arrows();
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, _) = two(&mut c, interest, &mut a, &mut b);
    c.request_focus(first.id);
    for _ in 0..3 {
        two(&mut c, interest, &mut a, &mut b);
    }
    let later = c.frame_time() + Duration::from_secs(60);
    assert!(
        !c.needs_repaint_at(later),
        "no widget, claim or group makes a timer"
    );
    assert!(c.next_repaint().is_none());
    tap(&mut c, KeyCode::ArrowRight);
    assert!(
        c.needs_repaint(),
        "the owner needs a pass to apply the event"
    );
    for _ in 0..3 {
        two(&mut c, interest, &mut a, &mut b);
    }
    assert!(!c.needs_repaint_at(later), "and idle again afterwards");
}

#[test]
fn a_disabled_widget_claims_nothing_and_drops_what_was_queued() {
    let interest = KeyInterest::arrows();
    let mut c = setup();
    let mut log = Vec::new();
    let build = |c: &mut Context, enabled: bool, log: &mut Vec<KeyEvent>| {
        let mut out = None;
        pass(c, |ui| {
            ui.add_enabled_ui(enabled, |ui| out = Some(pad(ui, "p", interest, log)));
        });
        out.unwrap()
    };
    let r = build(&mut c, true, &mut log);
    c.request_focus(r.id);
    build(&mut c, true, &mut log);
    tap(&mut c, KeyCode::ArrowDown);
    build(&mut c, false, &mut log);
    assert!(
        log.is_empty(),
        "events of a widget that is no longer enabled are dropped"
    );
    assert_eq!(c.input_stats().key_claims, 0);
}

#[test]
fn opening_a_modal_or_closing_a_popup_drops_what_waits_and_who_holds_a_key() {
    use zaxis::{Modal, Popup};
    let interest = KeyInterest::arrows();
    let mut c = setup();
    let mut log = Vec::new();
    let mut open = false;
    let build = |c: &mut Context, open: &mut bool, log: &mut Vec<KeyEvent>| {
        let mut id = None;
        pass(c, |ui| {
            id = Some(pad(ui, "p", interest, log).id);
            Modal::new("m").show(ui, open, |ui| {
                ui.label("modal");
            });
        });
        id.unwrap()
    };
    let id = build(&mut c, &mut open, &mut log);
    c.request_focus(id);
    build(&mut c, &mut open, &mut log);
    assert!(down(&mut c, KeyCode::ArrowRight));
    assert_eq!(c.input_stats().keys_owned, 1);
    open = true;
    build(&mut c, &mut open, &mut log);
    assert_eq!(
        c.input_stats().keys_owned,
        0,
        "a modal took over: nobody holds a key"
    );
    assert!(
        !up(&mut c, KeyCode::ArrowRight),
        "the release goes the usual way"
    );

    let mut c = setup();
    let mut open = false;
    let build = |c: &mut Context, open: &mut bool, log: &mut Vec<KeyEvent>| {
        let mut id = None;
        pass(c, |ui| {
            id = Some(pad(ui, "p", interest, log).id);
            let anchor = ui.button("anchor").rect;
            Popup::new("pop", anchor).show(ui, open, |ui| {
                ui.label("popup");
            });
        });
        id.unwrap()
    };
    let id = build(&mut c, &mut open, &mut log);
    c.request_focus(id);
    build(&mut c, &mut open, &mut log);
    assert!(down(&mut c, KeyCode::ArrowRight));
    open = true;
    build(&mut c, &mut open, &mut log);
    build(&mut c, &mut open, &mut log);
    assert_eq!(
        c.input_stats().keys_owned,
        1,
        "opening a popup does not end a gesture"
    );
    c.close_popup();
    assert_eq!(
        c.input_stats().keys_owned,
        0,
        "closing the popup released the key"
    );
}
