//! Layers, windows, transforms, scale, window focus, unwinding and cleanup.

use super::support::*;
use crate::prelude::*;
use winit::dpi::PhysicalSize;
use zaxis::{
    Button, Column, DiagnosticKind, FocusGroup, Grid, Modal, Popup, Root, ScrollArea, Window,
};

fn names(rs: &[(&'static str, Response)]) -> Option<&'static str> {
    rs.iter().find(|(_, r)| r.has_focus).map(|(n, _)| *n)
}

#[test]
fn content_that_opens_in_another_layer_does_not_join_the_group() {
    let mut c = setup();
    let mut open = false;
    let build = |c: &mut Context, open: &mut bool| {
        c.run(|c| {
            Window::new("w").show(c, |ui| {
                FocusGroup::new("g").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Button::new("a"));
                        let anchor = ui.add(Button::new("b")).rect;
                        Popup::new("pop", anchor).show(ui, open, |ui| {
                            ui.add(Button::new("in popup 1"));
                            ui.add(Button::new("in popup 2"));
                        });
                    });
                });
            });
        });
        c.input_stats().focus_group_members
    };
    build(&mut c, &mut open);
    let closed = build(&mut c, &mut open);
    open = true;
    build(&mut c, &mut open);
    assert_eq!(
        build(&mut c, &mut open),
        closed,
        "popup buttons are not members"
    );
    assert_eq!(closed, 2);
}

#[test]
fn a_group_inside_a_modal_is_a_stop_of_the_modal_and_focus_is_restored() {
    let mut c = setup();
    let mut open = false;
    let build = |c: &mut Context, open: &mut bool| {
        let mut rs: Vec<(&'static str, Response)> = Vec::new();
        pass(c, |ui| {
            rs.push(("outside", ui.add(Button::new("outside"))));
            Modal::new("m").show(ui, open, |ui| {
                FocusGroup::new("mg").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        rs.push(("m1", ui.add(Button::new("m1"))));
                        rs.push(("m2", ui.add(Button::new("m2"))));
                    });
                });
            });
        });
        rs
    };
    let rs = build(&mut c, &mut open);
    c.request_focus(rs[0].1.id);
    build(&mut c, &mut open);
    open = true;
    for _ in 0..4 {
        build(&mut c, &mut open);
    }
    let rs = build(&mut c, &mut open);
    assert_eq!(
        names(&rs),
        Some("m1"),
        "the modal took focus into its group"
    );
    tap(&mut c, KeyCode::ArrowRight);
    assert_eq!(names(&build(&mut c, &mut open)), Some("m2"));
    tab(&mut c);
    let rs = build(&mut c, &mut open);
    assert_ne!(names(&rs), Some("outside"), "Tab never leaves the modal");
    assert_ne!(
        names(&rs),
        Some("m1"),
        "and the group is one stop: m1 is not visited"
    );
    shift_tab(&mut c);
    assert_eq!(
        names(&build(&mut c, &mut open)),
        Some("m2"),
        "Shift+Tab comes back to the stop"
    );
    open = false;
    for _ in 0..4 {
        build(&mut c, &mut open);
    }
    assert_eq!(
        names(&build(&mut c, &mut open)),
        Some("outside"),
        "focus returns"
    );
}

#[test]
fn two_windows_have_independent_groups() {
    let mut c = setup();
    let build = |c: &mut Context| {
        let mut rs: Vec<(&'static str, Response)> = Vec::new();
        c.run(|c| {
            for (title, names) in [("W1", ["a1", "a2"]), ("W2", ["b1", "b2"])] {
                Window::new(title).show(c, |ui| {
                    FocusGroup::new("g").show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for n in names {
                                rs.push((n, ui.add(Button::new(n))));
                            }
                        });
                    });
                });
            }
        });
        rs
    };
    let rs = build(&mut c);
    assert_eq!(c.input_stats().focus_groups, 2);
    c.request_focus(rs[2].1.id);
    build(&mut c);
    tap(&mut c, KeyCode::ArrowRight);
    assert_eq!(names(&build(&mut c)), Some("b2"));
    tap(&mut c, KeyCode::ArrowLeft);
    tap(&mut c, KeyCode::ArrowLeft);
    assert_eq!(
        names(&build(&mut c)),
        Some("b1"),
        "never crosses into the other window"
    );
}

#[test]
fn losing_and_regaining_the_window_focus_returns_to_the_remembered_member() {
    let mut c = setup();
    let o = Opts::default();
    bar(&mut c, &ABC, &o);
    tab(&mut c);
    tab(&mut c);
    tap(&mut c, KeyCode::ArrowRight);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("b"));
    c.on_input(InputEvent::Focus(false));
    assert_eq!(bar(&mut c, &ABC, &o).focused(), None);
    c.on_input(InputEvent::Focus(true));
    tab(&mut c);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("before"));
    tab(&mut c);
    assert_eq!(
        bar(&mut c, &ABC, &o).focused(),
        Some("b"),
        "the group remembered b"
    );
}

#[test]
fn members_in_grid_cells_navigate_in_their_published_order() {
    let mut c = setup();
    let build = |c: &mut Context| {
        let mut rs: Vec<(&'static str, Response)> = Vec::new();
        pass(c, |ui| {
            FocusGroup::new("grid").vertical().show(ui, |ui| {
                Grid::new("g")
                    .columns([Column::fixed("l", 80.0), Column::fixed("r", 80.0)])
                    .show(ui, |grid| {
                        for (row, pair) in [("r1", ["a", "b"]), ("r2", ["c", "d"])] {
                            grid.row(row, |r| {
                                for n in pair {
                                    r.cell(|ui| rs.push((n, ui.add(Button::new(n)))));
                                }
                            });
                        }
                    });
            });
        });
        rs
    };
    build(&mut c);
    tab(&mut c);
    assert_eq!(names(&build(&mut c)), Some("a"));
    let mut seen = Vec::new();
    for _ in 0..3 {
        tap(&mut c, KeyCode::ArrowDown);
        seen.push(names(&build(&mut c)));
    }
    assert_eq!(seen, [Some("b"), Some("c"), Some("d")]);
    let rs = build(&mut c);
    let d = rs.iter().find(|(n, _)| *n == "d").unwrap().1;
    assert!(
        d.rect.min.x > rs[0].1.rect.min.x,
        "d is in the second column and row"
    );
}

#[test]
fn a_scroll_area_inside_the_group_keeps_pointer_and_keyboard_in_step() {
    let mut c = setup();
    let build = |c: &mut Context| {
        let mut rs: Vec<(&'static str, Response)> = Vec::new();
        pass(c, |ui| {
            FocusGroup::new("list").vertical().show(ui, |ui| {
                ScrollArea::vertical()
                    .id_source("s")
                    .max_height(90.0)
                    .show(ui, |ui| {
                        for n in ["1", "2", "3"] {
                            rs.push((n, ui.add(Button::new(n))));
                        }
                    });
            });
        });
        rs
    };
    let rs = build(&mut c);
    click(&mut c, rs[1].1.rect.center());
    assert_eq!(
        names(&build(&mut c)),
        Some("2"),
        "a click focuses the member under it"
    );
    tap(&mut c, KeyCode::ArrowDown);
    assert_eq!(names(&build(&mut c)), Some("3"));
}

#[test]
fn a_scaled_display_moves_the_members_and_their_hits_together() {
    let mut c = setup();
    c.set_viewport(PhysicalSize::new(1600, 1200), 2.0);
    let o = Opts::default();
    let b = bar(&mut c, &ABC, &o);
    click(&mut c, b.item("c").rect.center() * 2.0);
    let b = bar(&mut c, &ABC, &o);
    assert_eq!(b.focused(), Some("c"));
    tap(&mut c, KeyCode::Home);
    assert_eq!(bar(&mut c, &ABC, &o).focused(), Some("a"));
}

#[test]
fn a_disabled_group_is_not_a_stop_and_ignores_arrows() {
    let mut c = setup();
    let build = |c: &mut Context| {
        let mut rs: Vec<(&'static str, Response)> = Vec::new();
        pass(c, |ui| {
            rs.push(("before", ui.add(Button::new("before"))));
            ui.add_enabled_ui(false, |ui| {
                FocusGroup::new("g").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Button::new("x"));
                        ui.add(Button::new("y"));
                    });
                });
            });
            rs.push(("after", ui.add(Button::new("after"))));
        });
        rs
    };
    build(&mut c);
    tab(&mut c);
    assert_eq!(names(&build(&mut c)), Some("before"));
    tab(&mut c);
    assert_eq!(
        names(&build(&mut c)),
        Some("after"),
        "the disabled group has no stop"
    );
    assert_eq!(c.input_stats().focus_groups, 0);
}

#[test]
fn a_panic_in_the_group_closure_leaves_no_scope_open() {
    let mut c = setup();
    pass(&mut c, |ui| {
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            FocusGroup::new("boom").show(ui, |ui| {
                ui.add(Button::new("x"));
                panic!("in the closure");
            });
        }));
        assert!(caught.is_err());
        ui.add(Button::new("later"));
    });
    assert!(
        c.diagnostics()
            .iter()
            .all(|d| d.kind != DiagnosticKind::UnbalancedScope),
        "{:?}",
        c.diagnostics()
    );
    bar(&mut c, &ABC, &Opts::default());
    assert_eq!(
        c.input_stats().focus_groups,
        1,
        "the next pass starts clean"
    );
}

#[test]
fn two_groups_with_one_id_are_reported_and_do_not_panic() {
    let mut c = setup();
    pass(&mut c, |ui| {
        for _ in 0..2 {
            FocusGroup::new("same").show(ui, |ui| {
                ui.add(Button::new("x"));
            });
        }
    });
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::IdCollision));
}

#[test]
fn a_group_is_free_when_idle() {
    let mut c = setup();
    let o = Opts::default();
    for _ in 0..3 {
        bar(&mut c, &ABC, &o);
    }
    assert!(!c.needs_repaint_at(c.frame_time() + std::time::Duration::from_secs(60)));
    assert!(c.next_repaint().is_none());
    let _ = Root::new;
}

#[test]
fn a_visual_transform_moves_paint_hits_and_navigation_together() {
    let mut c = setup();
    let transform = zaxis::Transform::around(Vec2::ZERO, 1.5, Vec2::new(40.0, 30.0));
    let build = |c: &mut Context| {
        let mut rs: Vec<(&'static str, Response)> = Vec::new();
        pass(c, |ui| {
            ui.visual("v", transform, 1.0, |ui| {
                FocusGroup::new("g").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for n in ["a", "b", "c"] {
                            rs.push((n, ui.add(Button::new(n))));
                        }
                    });
                });
            });
        });
        rs
    };
    build(&mut c);
    let rs = build(&mut c);
    let b = rs[1].1;
    // The response carries layout coordinates; the pointer works in displayed ones.
    let displayed = transform.point(b.rect.center());
    click(&mut c, displayed);
    assert_eq!(
        names(&build(&mut c)),
        Some("b"),
        "a click lands on the displayed member"
    );
    tap(&mut c, KeyCode::ArrowRight);
    assert_eq!(names(&build(&mut c)), Some("c"));
    tap(&mut c, KeyCode::Home);
    assert_eq!(names(&build(&mut c)), Some("a"));
}

#[test]
fn two_contexts_keep_separate_groups_and_memory() {
    let o = Opts::default();
    let (mut one, mut two) = (setup(), setup());
    bar(&mut one, &ABC, &o);
    bar(&mut two, &ABC, &o);
    tab(&mut one);
    tab(&mut one);
    tap(&mut one, KeyCode::End);
    assert_eq!(bar(&mut one, &ABC, &o).focused(), Some("c"));
    assert_eq!(
        bar(&mut two, &ABC, &o).focused(),
        None,
        "the other context has no focus"
    );
    tab(&mut two);
    tab(&mut two);
    assert_eq!(
        bar(&mut two, &ABC, &o).focused(),
        Some("a"),
        "its group remembers nothing of the first"
    );
    assert_eq!(bar(&mut one, &ABC, &o).focused(), Some("c"));
}
