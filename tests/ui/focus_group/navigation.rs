use super::support::*;
use crate::prelude::*;
use zaxis::FocusAxis;

fn start() -> Context {
    let mut c = setup();
    bar(&mut c, &ABC, &Opts::default());
    c
}

#[test]
fn a_group_is_one_tab_stop() {
    let mut c = start();
    let o = Opts::default();
    tab(&mut c);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("before"));
    tab(&mut c);
    assert_eq!(
        bar(&mut c, &ABC, &o).focused(),
        Some("a"),
        "Tab enters at the first"
    );
    tab(&mut c);
    assert_eq!(
        bar(&mut c, &ABC, &o).focused(),
        Some("after"),
        "one Tab leaves the whole group"
    );
    tab(&mut c);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("before"));
}

#[test]
fn shift_tab_enters_at_the_member_focus_was_last_on() {
    let mut c = start();
    let o = Opts::default();
    tab(&mut c);
    tab(&mut c);
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowRight, &ABC, &o),
        Some("b")
    );
    tab(&mut c);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("after"));
    shift_tab(&mut c);
    assert_eq!(
        bar(&mut c, &ABC, &o).focused(),
        Some("b"),
        "re-entry remembers the member"
    );
    shift_tab(&mut c);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("before"));
}

#[test]
fn arrows_move_focus_along_the_axis_and_consume_the_key() {
    let mut c = start();
    let o = Opts::default();
    tab(&mut c);
    tab(&mut c);
    assert!(down(&mut c, KeyCode::ArrowRight), "consumed on arrival");
    up(&mut c, KeyCode::ArrowRight);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("b"));
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowRight, &ABC, &o),
        Some("c")
    );
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowLeft, &ABC, &o),
        Some("b")
    );
    assert!(
        !tap(&mut c, KeyCode::ArrowUp),
        "the other axis is not the group's"
    );
}

#[test]
fn home_and_end_jump_to_the_ends() {
    let mut c = start();
    let o = Opts::default();
    tab(&mut c);
    tab(&mut c);
    assert_eq!(key_then_look(&mut c, KeyCode::End, &ABC, &o), Some("c"));
    assert_eq!(key_then_look(&mut c, KeyCode::Home, &ABC, &o), Some("a"));
}

#[test]
fn at_the_end_nothing_wraps_by_default_and_the_key_stays_with_the_group() {
    let mut c = start();
    let o = Opts::default();
    tab(&mut c);
    tab(&mut c);
    assert!(
        down(&mut c, KeyCode::ArrowLeft),
        "owned even though focus cannot move"
    );
    up(&mut c, KeyCode::ArrowLeft);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("a"));
    tap(&mut c, KeyCode::End);
    assert!(tap(&mut c, KeyCode::ArrowRight));
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("c"));
}

#[test]
fn wrap_continues_at_the_other_end_when_asked() {
    let mut c = start();
    let o = Opts {
        wrap: true,
        ..Opts::default()
    };
    bar(&mut c, &ABC, &o);
    tab(&mut c);
    tab(&mut c);
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowLeft, &ABC, &o),
        Some("c")
    );
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowRight, &ABC, &o),
        Some("a")
    );
}

#[test]
fn a_vertical_group_uses_up_and_down_and_both_uses_all_four() {
    let mut c = start();
    let o = Opts {
        axis: FocusAxis::Vertical,
        ..Opts::default()
    };
    bar(&mut c, &ABC, &o);
    tab(&mut c);
    tab(&mut c);
    assert!(!tap(&mut c, KeyCode::ArrowRight));
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowDown, &ABC, &o),
        Some("b")
    );
    let o = Opts {
        axis: FocusAxis::Both,
        ..Opts::default()
    };
    bar(&mut c, &ABC, &o);
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowRight, &ABC, &o),
        Some("c")
    );
    assert_eq!(key_then_look(&mut c, KeyCode::ArrowUp, &ABC, &o), Some("b"));
}

#[test]
fn several_arrows_before_a_pass_all_count() {
    let mut c = start();
    let o = Opts::default();
    tab(&mut c);
    tab(&mut c);
    tap(&mut c, KeyCode::ArrowRight);
    tap(&mut c, KeyCode::ArrowRight);
    let b = bar(&mut c, &ABC, &o);
    assert_eq!(b.focused(), Some("c"), "no redraw in between");
    assert_eq!(
        b.navigated,
        Some(b.item("c").id),
        "the group reports where it went"
    );
    assert_eq!(bar(&mut c, &ABC, &o).navigated, None, "reported once");
}

#[test]
fn modified_arrows_and_held_repeats_follow_the_rules() {
    let mut c = start();
    let o = Opts::default();
    tab(&mut c);
    tab(&mut c);
    mods(&mut c, winit::keyboard::ModifiersState::CONTROL);
    assert!(
        !tap(&mut c, KeyCode::ArrowRight),
        "Ctrl+Arrow is not navigation"
    );
    mods(&mut c, winit::keyboard::ModifiersState::empty());
    assert!(down(&mut c, KeyCode::ArrowRight));
    assert!(
        again(&mut c, KeyCode::ArrowRight),
        "the repeat is owned too"
    );
    assert!(again(&mut c, KeyCode::ArrowRight));
    assert!(up(&mut c, KeyCode::ArrowRight), "and the release");
    assert_eq!(
        bar(&mut c, &ABC, &o).focused(),
        Some("c"),
        "each repeat steps once"
    );
}

#[test]
fn the_group_reports_focus_entering_and_leaving_once_like_a_widget() {
    let mut c = start();
    let o = Opts::default();
    tab(&mut c);
    let b = bar(&mut c, &ABC, &o);
    assert!(!b.group.has_focus && !b.group.gained_focus());
    tab(&mut c);
    let b = bar(&mut c, &ABC, &o);
    assert!(b.group.has_focus && b.group.gained_focus(), "Tab entered");
    assert!(b.group.focus_visible, "the keyboard shows its ring");
    tap(&mut c, KeyCode::ArrowRight);
    let b = bar(&mut c, &ABC, &o);
    assert!(
        b.group.has_focus && !b.group.gained_focus(),
        "moving inside is not entering"
    );
    let b = bar(&mut c, &ABC, &o);
    assert!(
        !b.group.gained_focus() && !b.group.lost_focus(),
        "events are one-shot"
    );
    tab(&mut c);
    let b = bar(&mut c, &ABC, &o);
    assert!(!b.group.has_focus && b.group.lost_focus(), "Tab left");
}
