//! Guarantees of every modal pass: each part is built once, one close reason per
//! closing, input released at once while the exit is still drawn, and a follow-up
//! repaint only when the measured sizes change.
use super::*;
use std::time::Duration;
use zaxis::{Confirm, Confirmation, Dialog, DialogAction};

const STEP: Duration = Duration::from_millis(16);

fn animated() -> Context {
    let mut c = setup();
    let mut style = c.style().clone();
    style.motion.reduced_motion = false;
    c.set_style(style);
    c
}

/// A modal with header, body and actions (or only a body when `plain`), counting
/// how often each part runs in every pass.
#[derive(Default)]
struct Parts {
    plain: bool,
    open: bool,
    lines: usize,
    /// The body asks to close through `Ui::close_modal` in the next pass.
    close: bool,
    runs: Vec<(u32, u32, u32)>,
    mounted: Vec<bool>,
    closed: Vec<CloseReason>,
    rect: Rect,
}

fn pass(c: &mut Context, p: &mut Parts, now: Instant) {
    let mut runs = (0, 0, 0);
    let mut mounted = false;
    c.run_at(now, |c| {
        Root::new().show(c, |ui| {
            ui.add(Button::new("Under").id_source("under"));
            let (lines, close) = (p.lines, p.close);
            let body = |ui: &mut Ui<'_>| {
                runs.1 += 1;
                for i in 0..lines {
                    ui.label(format!("Line {i}"));
                }
                if close {
                    ui.close_modal();
                }
            };
            let out = if p.plain {
                Modal::new("parts").show(ui, &mut p.open, body)
            } else {
                Modal::new("parts").show_parts(
                    ui,
                    &mut p.open,
                    |ui| {
                        runs.0 += 1;
                        ui.label("Title");
                    },
                    body,
                    |ui| {
                        runs.2 += 1;
                        ui.add(Button::new("OK").id_source("ok"));
                    },
                )
            };
            if let Some(out) = out {
                mounted = true;
                p.rect = out.rect;
                p.closed.extend(out.closed);
            }
        });
    });
    p.runs.push(runs);
    p.mounted.push(mounted);
}

/// Open, settle, close through the body and run until the exit animation ended.
fn open_and_close(plain: bool) -> Parts {
    let mut c = animated();
    let mut p = Parts {
        plain,
        open: true,
        lines: 3,
        ..Parts::default()
    };
    let mut t = Instant::now();
    for _ in 0..30 {
        t += STEP;
        pass(&mut c, &mut p, t);
    }
    p.close = true;
    t += STEP;
    pass(&mut c, &mut p, t);
    p.close = false;
    for _ in 0..40 {
        t += STEP;
        pass(&mut c, &mut p, t);
    }
    p
}

#[test]
fn header_body_and_footer_run_once_per_pass() {
    for plain in [false, true] {
        let p = open_and_close(plain);
        let once = if plain { (0, 1, 0) } else { (1, 1, 1) };
        for (i, (runs, mounted)) in p.runs.iter().zip(&p.mounted).enumerate() {
            let expected = if *mounted { once } else { (0, 0, 0) };
            assert_eq!(*runs, expected, "pass {i}, plain {plain}");
        }
        assert_eq!(p.closed, vec![CloseReason::Action], "plain {plain}");
        let exit = p.mounted[31..].iter().filter(|m| **m).count();
        assert!(exit > 1, "the exit animation keeps building the parts");
        assert!(!p.mounted.last().unwrap(), "fully closed in the end");
    }
}

#[test]
fn bodyless_confirmation_builds_its_parts_once_per_pass() {
    let mut c = animated();
    let mut open = true;
    let mut results = Vec::new();
    let mut t = Instant::now();
    let mut run = |c: &mut Context, open: &mut bool, t: Instant| {
        c.run_at(t, |c| {
            Root::new().show(c, |ui| {
                let confirm = Confirm::new("bodyless").title("Delete?");
                results.extend(confirm.show(ui, open));
            });
        });
    };
    for i in 0..30 {
        t += STEP;
        run(&mut c, &mut open, t);
        if i == 0 {
            // Unmeasured, the surface is still hidden and takes no input.
            continue;
        }
        let top = c.top_modal_id().unwrap();
        let actions = c
            .probe()
            .previous_hits
            .iter()
            .filter(|h| h.window == top && h.action == HitAction::Activate)
            .count();
        assert_eq!(actions, 2, "one cancel and one confirm button");
    }
    // Focus is on the confirming button: Enter activates it.
    c.on_key_event(KeyCode::Enter, ElementState::Pressed, false);
    t += STEP;
    run(&mut c, &mut open, t);
    c.on_key_event(KeyCode::Enter, ElementState::Released, false);
    for _ in 0..40 {
        t += STEP;
        run(&mut c, &mut open, t);
    }
    assert_eq!(results, vec![Confirmation::Confirmed]);
    assert!(!open && c.top_modal_id().is_none());
}

#[test]
fn escape_and_close_button_in_one_pass_close_once_by_escape() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    let corner = vec2(s.surface.max.x - 8.0 - 14.0, s.surface.min.y + 8.0 + 14.0);
    c.move_pointer(corner);
    c.primary_button(ElementState::Pressed);
    draw(&mut c, &mut s);
    c.primary_button(ElementState::Released);
    c.on_key_event(KeyCode::Escape, ElementState::Pressed, false);
    draw(&mut c, &mut s);
    c.on_key_event(KeyCode::Escape, ElementState::Released, false);
    settle(&mut c, &mut s);
    assert!(!s.open);
    assert_eq!(s.closed, vec![CloseReason::Escape]);
}

#[test]
fn overlay_click_and_close_modal_in_one_pass_close_once_by_overlay() {
    let mut c = setup();
    let mut p = Parts {
        open: true,
        lines: 3,
        ..Parts::default()
    };
    let mut t = Instant::now();
    for _ in 0..4 {
        t += STEP;
        pass(&mut c, &mut p, t);
    }
    c.move_pointer(vec2(2.0, 2.0));
    c.primary_button(ElementState::Pressed);
    t += STEP;
    pass(&mut c, &mut p, t);
    c.primary_button(ElementState::Released);
    p.close = true;
    t += STEP;
    pass(&mut c, &mut p, t);
    p.close = false;
    for _ in 0..4 {
        t += STEP;
        pass(&mut c, &mut p, t);
    }
    assert!(!p.open);
    assert_eq!(p.closed, vec![CloseReason::Overlay]);
}

#[test]
fn default_action_and_escape_in_one_pass_close_once_choosing_nothing() {
    let mut c = setup();
    let (mut open, mut actions, mut closed) = (true, Vec::new(), Vec::new());
    let mut run = |c: &mut Context, open: &mut bool| {
        c.run(|c| {
            Root::new().show(c, |ui| {
                let dialog = Dialog::new("both", "Rename")
                    .action(DialogAction::new("Cancel"))
                    .action(DialogAction::new("Save").primary())
                    .show(ui, open, |ui| {
                        ui.label("body");
                    });
                if let Some(dialog) = dialog {
                    actions.extend(dialog.action);
                    closed.extend(dialog.closed);
                }
            });
        });
    };
    for _ in 0..4 {
        run(&mut c, &mut open);
    }
    c.set_focus(None);
    c.on_key_event(KeyCode::Enter, ElementState::Pressed, false);
    c.on_key_event(KeyCode::Escape, ElementState::Pressed, false);
    for _ in 0..4 {
        run(&mut c, &mut open);
    }
    assert!(!open);
    assert_eq!(closed, vec![CloseReason::Escape]);
    assert!(actions.is_empty(), "Escape wins: no action is chosen");
}

#[test]
fn closing_modal_releases_input_at_once_while_its_exit_is_drawn() {
    let mut c = animated();
    let mut s = Scene::default();
    let mut t = Instant::now();
    let mut step = |c: &mut Context, s: &mut Scene| {
        t += STEP;
        draw_at(c, s, t);
    };
    step(&mut c, &mut s);
    s.open = true;
    for _ in 0..30 {
        step(&mut c, &mut s);
    }
    let id = c.top_modal_id().unwrap();
    let (inner, under) = (center(s.inner), center(s.under));
    c.on_key_event(KeyCode::Escape, ElementState::Pressed, false);
    step(&mut c, &mut s);
    c.on_key_event(KeyCode::Escape, ElementState::Released, false);
    assert_eq!(s.closed, vec![CloseReason::Escape]);
    assert!(c.top_modal_id().is_none() && !c.input_blocked());
    // Content of the closing modal takes no input while it fades.
    for p in [inner, under] {
        c.move_pointer(p);
        c.primary_button(ElementState::Pressed);
        step(&mut c, &mut s);
        c.primary_button(ElementState::Released);
        step(&mut c, &mut s);
    }
    assert_eq!(s.inner_clicks, 0, "the closing surface ignores clicks");
    assert_eq!(s.under_clicks, 1, "the click below lands at once");
    c.on_key_event(KeyCode::F3, ElementState::Pressed, false);
    step(&mut c, &mut s);
    c.on_key_event(KeyCode::F3, ElementState::Released, false);
    assert_eq!(s.shortcut, 1, "keys reach the application below");
    c.on_key_event(KeyCode::Escape, ElementState::Pressed, false);
    step(&mut c, &mut s);
    c.on_key_event(KeyCode::Escape, ElementState::Released, false);
    // Still drawn: the layer and its paint are there until the exit ends.
    assert!(c.probe().windows.contains_key(&id));
    assert!(c.probe().elements.iter().any(|e| e.layer == id));
    for _ in 0..30 {
        step(&mut c, &mut s);
    }
    assert!(!c.probe().windows.contains_key(&id));
    assert_eq!(s.closed, vec![CloseReason::Escape], "reported once");
}

#[test]
fn measures_ask_for_one_follow_up_pass_only_when_they_change() {
    let mut c = setup();
    let mut p = Parts {
        open: true,
        lines: 3,
        ..Parts::default()
    };
    let mut t = Instant::now();
    let mut quiet = |c: &mut Context, p: &mut Parts| {
        t += STEP;
        pass(c, p, t);
        c.probe().dirty
    };
    for _ in 0..4 {
        quiet(&mut c, &mut p);
    }
    assert!(!quiet(&mut c, &mut p), "stable sizes need no frames");
    let mut heights = vec![p.rect.size().y];
    for lines in [12, 4] {
        p.lines = lines;
        assert!(quiet(&mut c, &mut p), "new sizes ask for one follow-up");
        assert!(!quiet(&mut c, &mut p), "placed with them, no more frames");
        assert!(!quiet(&mut c, &mut p));
        heights.push(p.rect.size().y);
    }
    assert!(
        heights[1] > heights[2] && heights[2] > heights[0],
        "the surface follows the body: {heights:?}"
    );
}
