//! Events go to the focused widget that claimed the key, once, and to nobody else.

use super::support::*;
use crate::prelude::*;
use zaxis::KeyInterest;

/// Two pads with the same claims, built in a row. Returns what each received.
fn two(c: &mut Context, a: &mut Vec<KeyEvent>, b: &mut Vec<KeyEvent>) -> (Response, Response) {
    let mut out = None;
    pass(c, |ui| {
        let first = pad(ui, "a", KeyInterest::arrows(), a);
        let second = pad(ui, "b", KeyInterest::arrows(), b);
        out = Some((first, second));
    });
    out.unwrap()
}

#[test]
fn a_claimed_key_reaches_the_focused_widget_and_is_consumed_at_once() {
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, _) = two(&mut c, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, &mut a, &mut b);
    assert!(
        down(&mut c, KeyCode::ArrowRight),
        "consumed when the event arrives"
    );
    assert!(a.is_empty(), "nothing is delivered before the next pass");
    two(&mut c, &mut a, &mut b);
    assert_eq!(codes(&a), [KeyCode::ArrowRight]);
    assert!(b.is_empty(), "the other widget did not get it");
}

#[test]
fn two_widgets_with_the_same_claims_do_not_get_each_others_keys() {
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, second) = two(&mut c, &mut a, &mut b);
    assert_ne!(first.id, second.id);
    c.request_focus(second.id);
    two(&mut c, &mut a, &mut b);
    tap(&mut c, KeyCode::ArrowDown);
    two(&mut c, &mut a, &mut b);
    assert!(a.is_empty());
    assert_eq!(codes(&b), [KeyCode::ArrowDown]);
}

#[test]
fn a_key_is_delivered_once_and_not_again_on_later_passes() {
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, _) = two(&mut c, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, &mut a, &mut b);
    tap(&mut c, KeyCode::ArrowUp);
    for _ in 0..3 {
        two(&mut c, &mut a, &mut b);
    }
    assert_eq!(codes(&a), [KeyCode::ArrowUp]);
}

#[test]
fn an_event_stays_with_its_owner_when_focus_moves_before_the_pass() {
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, second) = two(&mut c, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, &mut a, &mut b);
    tap(&mut c, KeyCode::ArrowLeft);
    c.request_focus(second.id);
    two(&mut c, &mut a, &mut b);
    assert_eq!(
        codes(&a),
        [KeyCode::ArrowLeft],
        "it belongs to the widget that had focus"
    );
    assert!(b.is_empty(), "a later focus does not inherit it");
}

#[test]
fn a_widget_that_does_not_claim_a_key_leaves_it_unconsumed() {
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let (first, _) = two(&mut c, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, &mut a, &mut b);
    assert!(!tap(&mut c, KeyCode::KeyQ), "nobody claimed Q");
    assert!(!tap(&mut c, KeyCode::PageDown), "nor Page Down");
    two(&mut c, &mut a, &mut b);
    assert!(a.is_empty() && b.is_empty());
}

#[test]
fn before_the_first_pass_nothing_is_claimed() {
    let mut c = setup();
    assert!(!tap(&mut c, KeyCode::ArrowRight));
}

#[test]
fn without_focus_a_claim_is_inert() {
    let mut c = setup();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    two(&mut c, &mut a, &mut b);
    assert!(!tap(&mut c, KeyCode::ArrowRight));
    two(&mut c, &mut a, &mut b);
    assert!(a.is_empty() && b.is_empty());
}

#[test]
fn a_claim_lapses_when_the_widget_stops_declaring_it() {
    let mut c = setup();
    let mut a = Vec::new();
    let build = |c: &mut Context, claim: bool, a: &mut Vec<KeyEvent>| {
        let mut id = None;
        pass(c, |ui| {
            let rect = ui.allocate_space(vec2(90.0, 30.0));
            let r = ui.interact(rect, "p", Sense::CLICK | Sense::FOCUS);
            if claim {
                a.extend(ui.keys(&r, KeyInterest::arrows()));
            }
            id = Some(r.id);
        });
        id.unwrap()
    };
    let id = build(&mut c, true, &mut a);
    c.request_focus(id);
    build(&mut c, true, &mut a);
    assert!(tap(&mut c, KeyCode::ArrowLeft));
    build(&mut c, false, &mut a);
    build(&mut c, false, &mut a);
    assert!(!tap(&mut c, KeyCode::ArrowLeft), "the claim was given back");
}

#[test]
fn the_event_carries_both_keys_and_the_modifiers_of_its_moment() {
    let mut c = setup();
    let mut a = Vec::new();
    let mut b = Vec::new();
    let (first, _) = two(&mut c, &mut a, &mut b);
    c.request_focus(first.id);
    two(&mut c, &mut a, &mut b);
    mods(&mut c, winit::keyboard::ModifiersState::empty());
    down(&mut c, KeyCode::ArrowRight);
    up(&mut c, KeyCode::ArrowRight);
    two(&mut c, &mut a, &mut b);
    let event = &a[0];
    assert_eq!(event.code, KeyCode::ArrowRight);
    assert_eq!(
        event.physical,
        winit::keyboard::PhysicalKey::Code(KeyCode::ArrowRight)
    );
    assert_eq!(
        event.logical,
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight)
    );
    assert!(event.is_press() && !event.is_repeat() && !event.is_release());
}
