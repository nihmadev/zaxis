use crate::prelude::*;
use std::time::Duration;
use winit::dpi::PhysicalSize;
use zaxis::{
    vec2, Button, Presence, Rect, Reorder, Response, ScrollArea, Transform, TweenOptions, Window,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn setup() -> (Context, Instant) {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(900, 700), 1.0);
    let t = c.frame_time() + ms(1000);
    (c, t)
}
fn activate(c: &Context, id: Id) -> Option<HitRegion> {
    c.probe()
        .previous_hits
        .iter()
        .copied()
        .find(|hit| hit.id == id && hit.action == HitAction::Activate)
}
fn hit(c: &Context, id: Id) -> HitRegion {
    *c.probe()
        .previous_hits
        .iter()
        .find(|hit| hit.id == id)
        .unwrap()
}

#[test]
fn rotating_presence_turns_to_upright_and_is_usable_only_when_done() {
    let (mut c, t) = setup();
    let draw = |c: &mut Context, at, visible| {
        let mut response = None;
        c.run_at(t + ms(at), |c| {
            Window::new("rotation").show(c, |ui| {
                Presence::fade()
                    .rotating(std::f32::consts::PI)
                    .motion(TweenOptions::new(ms(200)))
                    .show(ui, "spinner", visible, |ui| {
                        response = Some(ui.button("Spin"));
                    });
            });
        });
        response
    };
    draw(&mut c, 0, false);
    let id = draw(&mut c, 1, true).unwrap().id;
    draw(&mut c, 101, true);
    let channel = Id::new(("window", "rotation"))
        .with("content")
        .with(("presence", Id::new("spinner")));
    let progress = c.sample_animation::<f32>(channel).unwrap().value;
    assert!(progress > 0.0 && progress < 1.0, "{progress}");
    assert!(activate(&c, id).is_none(), "turned content takes no input");

    draw(&mut c, 400, true);
    assert!(activate(&c, id).is_some());
    // Once the turn and the button's own state transitions are over, it sleeps.
    draw(&mut c, 1000, true);
    assert_eq!(c.next_repaint(), None);

    // Leaving turns it back and blocks input at once.
    draw(&mut c, 1001, false);
    assert!(activate(&c, id).is_none());
}

#[test]
fn appear_false_starts_shown_while_the_default_animates_in() {
    let (mut c, t) = setup();
    let channel = |name: &str| {
        Id::new(("window", name))
            .with("content")
            .with(("presence", Id::new("p")))
    };
    c.run_at(t, |c| {
        for (name, appear) in [("snap", false), ("fade", true)] {
            Window::new(name).show(c, |ui| {
                Presence::fade().appear(appear).show(ui, "p", true, |ui| {
                    ui.label("content");
                });
            });
        }
    });
    assert_eq!(
        c.sample_animation::<f32>(channel("snap")).unwrap().value,
        1.0
    );
    assert_eq!(
        c.sample_animation::<f32>(channel("fade")).unwrap().value,
        0.0
    );
}

#[test]
fn visual_accepts_a_rotation_but_only_upright_content_is_interactive() {
    let (mut c, t) = setup();
    let draw = |c: &mut Context, at, angle| {
        let mut response = None;
        c.run_at(t + ms(at), |c| {
            Window::new("visual").show(c, |ui| {
                let pivot = vec2(100.0, 100.0);
                ui.visual("turned", Transform::rotation(pivot, angle), 1.0, |ui| {
                    response = Some(ui.button("Turn"));
                });
            });
        });
        response.unwrap().id
    };
    let id = draw(&mut c, 0, 0.4);
    assert!(activate(&c, id).is_none());
    draw(&mut c, 1, 0.0);
    assert!(activate(&c, id).is_some());
}

/// Two scroll areas stacked in one window; the card is built in `area`.
fn move_card(c: &mut Context, at: Instant, area: usize) -> (Response, Rect) {
    let mut out = None;
    let mut second = Rect::default();
    c.run_at(at, |c| {
        Window::new("cards").show(c, |ui| {
            for index in 0..2 {
                let output = ScrollArea::vertical()
                    .id_source(("area", index))
                    .max_height(70.0)
                    .show(ui, |ui| {
                        if index == area {
                            ui.shared_with("card", TweenOptions::new(ms(200)), |ui| {
                                out = Some(ui.add(Button::new("Card").min_size(vec2(140.0, 36.0))));
                            });
                        } else {
                            ui.allocate_space(vec2(140.0, 36.0));
                        }
                    });
                if index == 1 {
                    second = output.viewport;
                }
            }
        });
    });
    (out.unwrap(), second)
}

#[test]
fn shared_element_glides_between_scroll_areas_and_keeps_its_identity() {
    let (mut c, t) = setup();
    let (first, _) = move_card(&mut c, t, 0);
    let start = hit(&c, first.id).rect;

    // Rebuilt in the other area: continues from where it was on screen. (The
    // destination's clip hides it until it arrives, so read the motion state.)
    let key = Id::new(("shared", "card"));
    let screen =
        |c: &Context| c.probe().effect_states[&key].position + c.probe().effect_states[&key].origin;
    let (moved, lower) = move_card(&mut c, t + ms(1), 1);
    assert_eq!(moved.id, first.id, "identity does not depend on the parent");
    assert!(
        (screen(&c) - start.min).length() < 0.5,
        "{:?} vs {:?}",
        screen(&c),
        start.min
    );

    move_card(&mut c, t + ms(101), 1);
    let middle = screen(&c).y;
    assert!(
        middle > start.min.y && middle < lower.min.y + 3.0,
        "{middle}"
    );

    move_card(&mut c, t + ms(400), 1);
    let end = hit(&c, moved.id).rect;
    assert!(
        end.min.y >= lower.min.y && end.max.y <= lower.max.y + 0.5,
        "{end:?} {lower:?}"
    );
    assert_eq!(c.next_repaint(), None, "settled elements sleep");
}

#[test]
fn shared_element_ignores_scrolling_and_starts_fresh_after_an_absent_pass() {
    let (mut c, t) = setup();
    let draw = |c: &mut Context, at, offset: f32, build: bool| {
        let mut rect = None;
        c.run_at(t + ms(at), |c| {
            Window::new("scrolling").show(c, |ui| {
                ScrollArea::vertical()
                    .max_height(80.0)
                    .scroll_offset(vec2(0.0, offset))
                    .show(ui, |ui| {
                        ui.allocate_space(vec2(100.0, 20.0));
                        if build {
                            ui.shared("row", |ui| {
                                rect = Some(ui.add(Button::new("Row")).rect);
                            });
                        }
                        ui.allocate_space(vec2(100.0, 300.0));
                    });
            });
        });
        rect
    };
    let before = draw(&mut c, 0, 0.0, true).unwrap();
    let scrolled = draw(&mut c, 1, 10.0, true).unwrap();
    assert_eq!(before.min.y - scrolled.min.y, 10.0);
    assert_eq!(c.next_repaint(), None, "scrolling is not movement");
    // Not built for a pass: its next appearance does not animate from the past.
    draw(&mut c, 2, 10.0, false);
    draw(&mut c, 3, 10.0, false);
    draw(&mut c, 4, 0.0, true);
    assert_eq!(c.next_repaint(), None);
}

#[test]
fn shared_sources_must_be_unique_within_a_pass() {
    let (mut c, t) = setup();
    c.run_at(t, |c| {
        Window::new("dup").show(c, |ui| {
            ui.shared("same", |ui| ui.label("one"));
            ui.shared("same", |ui| ui.label("two"));
        });
    });
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == zaxis::DiagnosticKind::IdCollision));
}

#[test]
fn removed_row_plays_its_exit_then_neighbours_glide_into_the_gap() {
    let (mut c, t) = setup();
    let ids: Vec<_> = (0..3).map(Id::new).collect();
    let presence = || {
        Presence::fade()
            .appear(false)
            .motion(TweenOptions::new(ms(200)))
    };
    // (id, present) rows; returns each row's button hit rect and exit flags.
    let draw = |c: &mut Context, at, rows: &[(Id, bool)]| {
        let mut out = Vec::new();
        c.run_at(t + ms(at), |c| {
            Window::new("list").show(c, |ui| {
                Reorder::new("rows")
                    .motion(TweenOptions::new(ms(200)))
                    .show(ui, rows.iter().map(|(id, _)| *id), |list| {
                        for &(id, present) in rows {
                            let row = list.item_presence(id, present, presence(), |ui| {
                                ui.add(Button::new("Row").min_size(vec2(140.0, 36.0))).id
                            });
                            out.push((id, row.inner, row.exited));
                        }
                    });
            });
        });
        out
    };
    let all: Vec<_> = ids.iter().map(|id| (*id, true)).collect();
    let rows = draw(&mut c, 0, &all);
    let third_button = rows[2].1.unwrap();
    let home = hit(&c, third_button).rect;
    assert!(rows
        .iter()
        .all(|(_, inner, exited)| inner.is_some() && !exited));

    // Leaving: still built and still occupying its space, not yet exited.
    let leaving = [(ids[0], true), (ids[1], false), (ids[2], true)];
    let rows = draw(&mut c, 1, &leaving);
    assert!(rows[1].1.is_some() && !rows[1].2);
    draw(&mut c, 101, &leaving);
    assert!(
        activate(&c, rows[1].1.unwrap()).is_none(),
        "leaving rows take no input"
    );
    assert_eq!(
        hit(&c, third_button).rect,
        home,
        "the gap is kept during the exit"
    );

    let rows = draw(&mut c, 400, &leaving);
    assert!(rows[1].1.is_none() && rows[1].2, "exit finished");

    // The application drops it: the next row continues from where it was.
    let remaining = [(ids[0], true), (ids[2], true)];
    draw(&mut c, 401, &remaining);
    let start = hit(&c, third_button).rect;
    assert!(
        (start.min.y - home.min.y).abs() < 1.0,
        "{start:?} vs {home:?}"
    );
    draw(&mut c, 501, &remaining);
    let middle = hit(&c, third_button).rect;
    assert!(middle.min.y < home.min.y && middle.min.y > home.min.y - 40.0);
    draw(&mut c, 700, &remaining);
    let end = hit(&c, third_button).rect;
    assert!(end.min.y < middle.min.y);
    assert_eq!(c.next_repaint(), None);
}
