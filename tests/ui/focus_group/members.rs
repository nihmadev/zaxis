//! Which members take part: pointer focus, disabled, hidden and clipped members,
//! reordering, removal, and the entry a control names.

use super::support::*;
use crate::prelude::*;
use zaxis::{Button, FocusGroup, Root, ScrollArea};

const FOUR: [&str; 4] = ["a", "b", "c", "d"];

fn ready(names: &[&'static str], o: &Opts) -> Context {
    let mut c = setup();
    bar(&mut c, names, o);
    c
}

#[test]
fn a_click_focuses_any_member_and_it_becomes_the_stop() {
    let o = Opts::default();
    let mut c = ready(&FOUR, &o);
    let at = bar(&mut c, &FOUR, &o).item("c").rect.center();
    click(&mut c, at);
    let b = bar(&mut c, &FOUR, &o);
    assert_eq!(
        b.focused(),
        Some("c"),
        "a member that was not the stop takes focus"
    );
    assert!(!b.item("c").focus_visible, "a pointer focus shows no ring");
    tab(&mut c);
    assert_eq!(bar(&mut c, &FOUR, &o).focused(), Some("after"));
    shift_tab(&mut c);
    assert_eq!(
        bar(&mut c, &FOUR, &o).focused(),
        Some("c"),
        "Shift+Tab comes back to it"
    );
}

#[test]
fn disabled_members_are_skipped_by_arrows_and_never_the_stop() {
    let o = Opts {
        disabled: vec!["a", "c"],
        ..Opts::default()
    };
    let mut c = ready(&FOUR, &o);
    tab(&mut c);
    tab(&mut c);
    assert_eq!(
        bar(&mut c, &FOUR, &o).focused(),
        Some("b"),
        "the first enabled member"
    );
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowRight, &FOUR, &o),
        Some("d")
    );
    assert_eq!(key_then_look(&mut c, KeyCode::Home, &FOUR, &o), Some("b"));
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowLeft, &FOUR, &o),
        Some("b"),
        "nothing before it"
    );
}

#[test]
fn a_click_on_a_disabled_member_focuses_nothing_in_the_group() {
    let o = Opts {
        disabled: vec!["b"],
        ..Opts::default()
    };
    let mut c = ready(&FOUR, &o);
    let at = bar(&mut c, &FOUR, &o).item("b").rect.center();
    click(&mut c, at);
    assert_eq!(bar(&mut c, &FOUR, &o).focused(), None);
}

#[test]
fn hidden_members_take_no_part() {
    let o = Opts {
        hidden: vec!["b"],
        ..Opts::default()
    };
    let mut c = ready(&FOUR, &o);
    tab(&mut c);
    tab(&mut c);
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowRight, &FOUR, &o),
        Some("c")
    );
}

#[test]
fn removing_the_remembered_member_hands_the_stop_to_its_nearest_neighbour() {
    let o = Opts::default();
    let mut c = ready(&FOUR, &o);
    let at = bar(&mut c, &FOUR, &o).item("b").rect.center();
    click(&mut c, at);
    tab(&mut c);
    assert_eq!(bar(&mut c, &FOUR, &o).focused(), Some("after"));
    let gone = Opts {
        hidden: vec!["b"],
        ..Opts::default()
    };
    bar(&mut c, &FOUR, &gone);
    let b = bar(&mut c, &FOUR, &gone);
    assert_eq!(
        b.stop,
        Some(b.item("c").id),
        "the one that now stands in its place"
    );
    shift_tab(&mut c);
    assert_eq!(bar(&mut c, &FOUR, &gone).focused(), Some("c"));
}

#[test]
fn removing_the_last_member_falls_back_to_the_new_last() {
    let o = Opts::default();
    let mut c = ready(&FOUR, &o);
    tap(&mut c, KeyCode::Tab);
    tap(&mut c, KeyCode::Tab);
    tap(&mut c, KeyCode::End);
    bar(&mut c, &FOUR, &o);
    tab(&mut c);
    let gone = Opts {
        hidden: vec!["d"],
        ..Opts::default()
    };
    bar(&mut c, &FOUR, &gone);
    let b = bar(&mut c, &FOUR, &gone);
    assert_eq!(b.stop, Some(b.item("c").id));
}

#[test]
fn reordering_keeps_each_members_identity_and_the_stop() {
    let o = Opts::default();
    let mut c = ready(&FOUR, &o);
    let before = bar(&mut c, &FOUR, &o);
    let ids: Vec<Id> = FOUR.iter().map(|n| before.item(n).id).collect();
    let at = before.item("b").rect.center();
    click(&mut c, at);
    let reordered = ["d", "c", "b", "a"];
    let after = bar(&mut c, &reordered, &o);
    for (name, id) in FOUR.iter().zip(&ids) {
        assert_eq!(after.item(name).id, *id, "{name} keeps its id");
    }
    assert_eq!(
        after.focused(),
        Some("b"),
        "focus stays on the same control"
    );
    assert_eq!(
        key_then_look(&mut c, KeyCode::ArrowRight, &reordered, &o),
        Some("a")
    );
}

#[test]
fn an_empty_group_has_no_stop_and_leaves_no_state() {
    let o = Opts {
        hidden: vec!["a", "b", "c"],
        ..Opts::default()
    };
    let mut c = ready(&ABC, &Opts::default());
    bar(&mut c, &ABC, &o);
    let b = bar(&mut c, &ABC, &o);
    assert!(b.items.is_empty() && b.stop.is_none());
    assert_eq!(c.input_stats().focus_groups, 0);
    tab(&mut c);
    tab(&mut c);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("after"));
    assert_eq!(c.input_stats().focus_group_members, 0);
}

#[test]
fn an_entry_overrides_the_remembered_member_while_focus_is_outside() {
    let mut c = setup();
    let mut chosen = Vec::<&str>::new();
    let build = |c: &mut Context, entry: &str| {
        let mut out = Vec::new();
        c.run(|c| {
            Root::new().show(c, |ui| {
                ui.add(Button::new("before"));
                FocusGroup::new("g").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for name in ["a", "b", "c"] {
                            let r = ui.add(Button::new(name));
                            if name == entry {
                                ui.focus_entry(&r);
                            }
                            out.push((name, r));
                        }
                    });
                });
            });
        });
        out
    };
    let items = build(&mut c, "c");
    tab(&mut c);
    tab(&mut c);
    let items2 = build(&mut c, "c");
    let focused = items2.iter().find(|(_, r)| r.has_focus).map(|(n, _)| *n);
    chosen.extend(focused);
    assert_eq!(chosen, ["c"], "Tab lands on the entry, not the first");
    let _ = items;
}

#[test]
fn members_that_scroll_out_of_view_are_not_reachable() {
    let mut c = setup();
    let run = |c: &mut Context| {
        let mut rects = Vec::new();
        c.run(|c| {
            Root::new().show(c, |ui| {
                FocusGroup::new("list").vertical().show(ui, |ui| {
                    ScrollArea::vertical()
                        .id_source("s")
                        .max_height(70.0)
                        .show(ui, |ui| {
                            for name in ["1", "2", "3", "4", "5", "6"] {
                                rects.push((name, ui.add(Button::new(name))));
                            }
                        });
                });
            });
        });
        rects
    };
    run(&mut c);
    tab(&mut c);
    let mut seen = vec![];
    for _ in 0..6 {
        tap(&mut c, KeyCode::ArrowDown);
        let items = run(&mut c);
        seen.push(items.iter().find(|(_, r)| r.has_focus).map(|(n, _)| *n));
    }
    let last = seen.last().copied().flatten();
    assert!(
        last.is_some_and(|n| n != "6"),
        "the group never moves focus onto a member that is clipped away: {seen:?}"
    );
}
