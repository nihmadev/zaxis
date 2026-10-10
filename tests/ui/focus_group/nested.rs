//! A group inside a group is one member of the outer one, and a slot navigates itself.

use super::support::*;
use crate::prelude::*;
use zaxis::{Button, FocusAxis, FocusGroup, Root};

struct Shape {
    inner: FocusAxis,
    slot: bool,
}

fn build(c: &mut Context, shape: &Shape) -> Option<&'static str> {
    let mut rs: Vec<(&'static str, Response)> = Vec::new();
    c.run(|c| {
        Root::new().show(c, |ui| {
            rs.push(("before", ui.add(Button::new("before"))));
            FocusGroup::new("outer").show(ui, |ui| {
                ui.horizontal(|ui| {
                    rs.push(("a", ui.add(Button::new("a"))));
                    let inner = if shape.slot {
                        FocusGroup::slot("inner")
                    } else {
                        FocusGroup::new("inner").axis(shape.inner)
                    };
                    inner.show(ui, |ui| {
                        ui.vertical(|ui| {
                            for name in ["x", "y", "z"] {
                                rs.push((name, ui.add(Button::new(name))));
                            }
                        });
                    });
                    rs.push(("b", ui.add(Button::new("b"))));
                });
            });
            rs.push(("after", ui.add(Button::new("after"))));
        });
    });
    rs.iter().find(|(_, r)| r.has_focus).map(|(n, _)| *n)
}

fn vertical() -> Shape {
    Shape {
        inner: FocusAxis::Vertical,
        slot: false,
    }
}

fn enter_at_a(c: &mut Context, shape: &Shape) {
    build(c, shape);
    tab(c);
    tab(c);
    assert_eq!(build(c, shape), Some("a"));
}

fn go(c: &mut Context, shape: &Shape, key: KeyCode) -> Option<&'static str> {
    tap(c, key);
    build(c, shape)
}

#[test]
fn a_nested_group_is_one_member_of_the_outer_group() {
    let (mut c, s) = (setup(), vertical());
    enter_at_a(&mut c, &s);
    assert_eq!(
        go(&mut c, &s, KeyCode::ArrowRight),
        Some("x"),
        "enters at its stop"
    );
    assert_eq!(
        go(&mut c, &s, KeyCode::ArrowRight),
        Some("b"),
        "and the next step leaves it whole"
    );
    assert_eq!(
        go(&mut c, &s, KeyCode::ArrowLeft),
        Some("x"),
        "re-enters where it was"
    );
    assert_eq!(go(&mut c, &s, KeyCode::ArrowLeft), Some("a"));
}

#[test]
fn the_innermost_group_with_the_axis_takes_the_key_and_it_runs_on_one_level_only() {
    let (mut c, s) = (setup(), vertical());
    enter_at_a(&mut c, &s);
    go(&mut c, &s, KeyCode::ArrowRight);
    assert_eq!(go(&mut c, &s, KeyCode::ArrowDown), Some("y"));
    assert_eq!(go(&mut c, &s, KeyCode::ArrowDown), Some("z"));
    assert!(
        tap(&mut c, KeyCode::ArrowDown),
        "at the end of the inner group the key is still owned"
    );
    assert_eq!(
        build(&mut c, &s),
        Some("z"),
        "the outer axis does not use Down"
    );
    assert_eq!(
        go(&mut c, &s, KeyCode::Home),
        Some("x"),
        "Home stays with the inner group"
    );
    assert_eq!(go(&mut c, &s, KeyCode::End), Some("z"));
}

#[test]
fn an_arrow_at_the_end_of_a_same_axis_inner_group_goes_to_the_outer_one() {
    let (mut c, s) = (
        setup(),
        Shape {
            inner: FocusAxis::Horizontal,
            slot: false,
        },
    );
    enter_at_a(&mut c, &s);
    go(&mut c, &s, KeyCode::ArrowRight);
    assert_eq!(go(&mut c, &s, KeyCode::ArrowRight), Some("y"));
    assert_eq!(go(&mut c, &s, KeyCode::ArrowRight), Some("z"));
    assert_eq!(
        go(&mut c, &s, KeyCode::ArrowRight),
        Some("b"),
        "one step, one level"
    );
}

#[test]
fn tab_leaves_the_outer_group_from_inside_a_nested_one() {
    let (mut c, s) = (setup(), vertical());
    enter_at_a(&mut c, &s);
    go(&mut c, &s, KeyCode::ArrowRight);
    go(&mut c, &s, KeyCode::ArrowDown);
    assert_eq!(go(&mut c, &s, KeyCode::Tab), Some("after"));
    shift_tab(&mut c);
    assert_eq!(
        build(&mut c, &s),
        Some("y"),
        "back into the member focus was last on"
    );
}

#[test]
fn a_slot_is_one_member_and_keeps_the_arrows_for_itself() {
    let (mut c, s) = (
        setup(),
        Shape {
            inner: FocusAxis::None,
            slot: true,
        },
    );
    enter_at_a(&mut c, &s);
    assert_eq!(
        go(&mut c, &s, KeyCode::ArrowRight),
        Some("x"),
        "the outer arrows move onto the slot"
    );
    assert!(
        !tap(&mut c, KeyCode::ArrowRight),
        "inside, no group takes the key: the parts do"
    );
    assert_eq!(build(&mut c, &s), Some("x"));
    assert!(!tap(&mut c, KeyCode::ArrowDown));
    assert_eq!(
        go(&mut c, &s, KeyCode::Tab),
        Some("after"),
        "Tab leaves it, whole"
    );
    shift_tab(&mut c);
    assert_eq!(
        build(&mut c, &s),
        Some("x"),
        "and comes back to the part it was on"
    );
}

#[test]
fn nested_groups_report_focus_to_every_group_around_the_member() {
    let (mut c, s) = (setup(), vertical());
    enter_at_a(&mut c, &s);
    let mut seen = Vec::new();
    for key in [KeyCode::ArrowRight, KeyCode::ArrowRight] {
        tap(&mut c, key);
        let mut flags = (false, false, false, false);
        c.run(|c| {
            Root::new().show(c, |ui| {
                let outer = FocusGroup::new("outer").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Button::new("a"));
                        let inner = FocusGroup::new("inner").vertical().show(ui, |ui| {
                            ui.vertical(|ui| {
                                for name in ["x", "y", "z"] {
                                    ui.add(Button::new(name));
                                }
                            });
                        });
                        flags.2 = inner.response.has_focus;
                        flags.3 = inner.response.gained_focus() || inner.response.lost_focus();
                        ui.add(Button::new("b"));
                    });
                });
                flags.0 = outer.response.has_focus;
                flags.1 = outer.response.gained_focus() || outer.response.lost_focus();
            });
        });
        seen.push(flags);
    }
    assert_eq!(
        seen[0],
        (true, false, true, true),
        "entering the inner group: outer stays focused"
    );
    assert_eq!(
        seen[1],
        (true, false, false, true),
        "leaving the inner group for b"
    );
}

#[test]
fn a_tab_bar_in_a_slot_keeps_its_own_arrows_and_the_group_does_not_double_them() {
    let mut c = setup();
    let mut selected = 0_usize;
    let build = |c: &mut Context, selected: &mut usize| {
        let mut out = (None, 0);
        c.run(|c| {
            Root::new().show(c, |ui| {
                ui.add(Button::new("before"));
                FocusGroup::new("bar").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let a = ui.add(Button::new("a"));
                        let tabs = FocusGroup::slot("tabs").show(ui, |ui| {
                            ui.tab_bar(selected, [(0, "One"), (1, "Two"), (2, "Three")])
                        });
                        let b = ui.add(Button::new("b"));
                        out.0 = [("a", a), ("b", b)]
                            .into_iter()
                            .find(|(_, r)| r.has_focus)
                            .map(|(n, _)| n)
                            .or(tabs.response.has_focus.then_some("tabs"));
                    });
                });
            });
        });
        out.1 = *selected;
        out
    };
    build(&mut c, &mut selected);
    tab(&mut c);
    tab(&mut c);
    assert_eq!(build(&mut c, &mut selected).0, Some("a"));
    tap(&mut c, KeyCode::ArrowRight);
    assert_eq!(
        build(&mut c, &mut selected).0,
        Some("tabs"),
        "the arrow entered the tab bar"
    );
    tap(&mut c, KeyCode::ArrowRight);
    let (focus, now) = build(&mut c, &mut selected);
    assert_eq!(focus, Some("tabs"), "the second arrow stays in the bar");
    assert_eq!(now, 1, "and moves exactly one tab");
    tap(&mut c, KeyCode::ArrowRight);
    let (focus, now) = build(&mut c, &mut selected);
    assert_eq!((focus, now), (Some("tabs"), 2));
}

#[test]
fn after_navigation_moves_focus_the_next_key_goes_to_the_new_widget_without_a_redraw() {
    use crate::keys::support::pad;
    use zaxis::KeyInterest;
    let mut c = setup();
    let (mut one, mut two) = (Vec::new(), Vec::new());
    let build = |c: &mut Context, one: &mut Vec<KeyEvent>, two: &mut Vec<KeyEvent>| {
        pass(c, |ui| {
            FocusGroup::new("g").show(ui, |ui| {
                ui.horizontal(|ui| {
                    pad(ui, "one", KeyInterest::keys(&[KeyCode::KeyQ]), one);
                    pad(ui, "two", KeyInterest::keys(&[KeyCode::KeyQ]), two);
                });
            });
        });
    };
    build(&mut c, &mut one, &mut two);
    tab(&mut c);
    build(&mut c, &mut one, &mut two);
    tap(&mut c, KeyCode::KeyQ);
    tap(&mut c, KeyCode::ArrowRight);
    tap(&mut c, KeyCode::KeyQ);
    tap(&mut c, KeyCode::KeyQ);
    build(&mut c, &mut one, &mut two);
    assert_eq!(
        (one.len(), two.len()),
        (1, 2),
        "each Q went to who had focus when it came"
    );
}
