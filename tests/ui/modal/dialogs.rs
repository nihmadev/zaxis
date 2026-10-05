use super::*;
use zaxis::{Confirm, Confirmation, Dialog, DialogAction, Text};

struct Form {
    open: bool,
    danger: bool,
    items: Vec<&'static str>,
    results: Vec<Confirmation>,
    actions: Vec<usize>,
    closed: Vec<CloseReason>,
    dialog_open: bool,
}

fn form(danger: bool) -> Form {
    Form {
        open: false,
        danger,
        items: vec!["alpha", "beta", "gamma"],
        results: Vec::new(),
        actions: Vec::new(),
        closed: Vec::new(),
        dialog_open: false,
    }
}

fn run(c: &mut Context, f: &mut Form) {
    c.run(|c| {
        Root::new().show(c, |ui| {
            for item in &f.items {
                ui.add(Text::new(*item));
            }
            let mut confirm = Confirm::new("delete")
                .title("Delete beta?")
                .description("It cannot be restored.")
                .confirm_label("Delete");
            if f.danger {
                confirm = confirm.danger();
            }
            if let Some(result) = confirm.show(ui, &mut f.open) {
                f.results.push(result);
                if result == Confirmation::Confirmed {
                    f.items.retain(|item| *item != "beta");
                }
            }
            let dialog = Dialog::new("dialog", "Rename")
                .description("Pick one")
                .action(DialogAction::new("Cancel"))
                .action(DialogAction::new("Save").primary())
                .show(ui, &mut f.dialog_open, |ui| {
                    ui.label("body");
                });
            if let Some(dialog) = dialog {
                f.actions.extend(dialog.action);
                f.closed.extend(dialog.closed);
            }
        });
    });
}

fn settle_form(c: &mut Context, f: &mut Form) {
    for _ in 0..4 {
        run(c, f);
    }
}

fn buttons(c: &Context) -> Vec<Vec2> {
    let top = c.top_modal_id().unwrap();
    c.probe()
        .previous_hits
        .iter()
        .filter(|h| h.window == top && h.action == HitAction::Activate)
        .map(|h| center(h.rect))
        .collect()
}

fn tap(c: &mut Context, f: &mut Form, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    run(c, f);
    c.primary_button(ElementState::Released);
    settle_form(c, f);
}

fn key(c: &mut Context, f: &mut Form, code: KeyCode) {
    c.on_key_event(code, ElementState::Pressed, false);
    run(c, f);
    c.on_key_event(code, ElementState::Released, false);
    settle_form(c, f);
}

#[test]
fn confirm_returns_the_choice_once_and_the_item_is_really_removed() {
    let mut c = setup();
    let mut f = form(true);
    f.open = true;
    settle_form(&mut c, &mut f);
    let buttons = buttons(&c);
    assert_eq!(buttons.len(), 2, "cancel and confirm");
    tap(&mut c, &mut f, buttons[1]);
    assert_eq!(f.results, vec![Confirmation::Confirmed]);
    assert_eq!(f.items, vec!["alpha", "gamma"]);
    assert!(!f.open);
}

#[test]
fn danger_confirm_closes_only_by_an_explicit_choice() {
    let mut c = setup();
    let mut f = form(true);
    f.open = true;
    settle_form(&mut c, &mut f);
    // Initial focus is on the safe (cancel) button.
    let cancel = buttons(&c)[0];
    let focused = c.probe().focused_widget.unwrap();
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .any(|h| h.id == focused && h.rect.contains(cancel)));
    key(&mut c, &mut f, KeyCode::Escape);
    assert!(
        f.open && f.results.is_empty(),
        "Escape does not close a danger confirm"
    );
    tap(&mut c, &mut f, vec2(5.0, 5.0));
    assert!(
        f.open && f.results.is_empty(),
        "the overlay does not close it"
    );
    // Enter on the safe default cancels, never confirms.
    key(&mut c, &mut f, KeyCode::Enter);
    assert_eq!(
        f.results,
        vec![Confirmation::Cancelled(CloseReason::Action)]
    );
    assert_eq!(f.items.len(), 3);
}

#[test]
fn plain_confirm_focuses_confirm_and_escape_cancels() {
    let mut c = setup();
    let mut f = form(false);
    f.open = true;
    settle_form(&mut c, &mut f);
    let confirm = buttons(&c)[1];
    let focused = c.probe().focused_widget.unwrap();
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .any(|h| h.id == focused && h.rect.contains(confirm)));
    key(&mut c, &mut f, KeyCode::Escape);
    assert_eq!(
        f.results,
        vec![Confirmation::Cancelled(CloseReason::Escape)]
    );
    f.open = true;
    settle_form(&mut c, &mut f);
    key(&mut c, &mut f, KeyCode::Enter);
    assert_eq!(f.results.last(), Some(&Confirmation::Confirmed));
    assert_eq!(f.items, vec!["alpha", "gamma"]);
}

#[test]
fn dialog_reports_the_action_and_the_reason_once() {
    let mut c = setup();
    let mut f = form(false);
    f.dialog_open = true;
    settle_form(&mut c, &mut f);
    let actions = buttons(&c);
    assert!(
        actions.len() >= 3,
        "two actions and the corner close button"
    );
    tap(&mut c, &mut f, actions[1]);
    assert_eq!(f.actions, vec![1]);
    assert_eq!(f.closed, vec![CloseReason::Action]);
    f.dialog_open = true;
    settle_form(&mut c, &mut f);
    // Focus on a button: Enter activates that button.
    key(&mut c, &mut f, KeyCode::Enter);
    assert_eq!(f.actions, vec![1, 0]);
    // Focus elsewhere: Enter runs the default (last) action.
    f.dialog_open = true;
    settle_form(&mut c, &mut f);
    c.set_focus(None);
    key(&mut c, &mut f, KeyCode::Enter);
    assert_eq!(f.actions, vec![1, 0, 1]);
    f.dialog_open = true;
    settle_form(&mut c, &mut f);
    key(&mut c, &mut f, KeyCode::Escape);
    assert_eq!(f.actions.len(), 3, "Escape chooses no action");
    assert_eq!(f.closed.last(), Some(&CloseReason::Escape));
}

#[test]
fn small_window_keeps_the_actions_visible() {
    let mut c = setup();
    let mut f = form(false);
    f.dialog_open = true;
    for (w, h) in [(800, 600), (300, 200), (230, 130)] {
        c.set_viewport(PhysicalSize::new(w, h), 1.0);
        settle_form(&mut c, &mut f);
        let viewport = c.viewport();
        let actions = buttons(&c);
        assert!(actions.len() >= 2, "{w}x{h}");
        for p in actions {
            assert!(viewport.contains(p), "action at {p:?} inside {w}x{h}");
        }
    }
}

#[test]
fn animated_dialog_buttons_work() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        animated_dialog_buttons_at(scale);
    }
}

fn animated_dialog_buttons_at(scale: f64) {
    let mut c = setup();
    c.set_viewport(
        PhysicalSize::new((860.0 * scale) as u32, (620.0 * scale) as u32),
        scale,
    );
    let mut style = c.style().clone();
    style.motion.reduced_motion = false;
    c.set_style(style);
    let mut f = form(false);
    f.dialog_open = true;
    let mut t = zaxis::Instant::now();
    let step = |c: &mut Context, f: &mut Form, t: &mut zaxis::Instant| {
        *t += std::time::Duration::from_millis(16);
        c.run_at(*t, |c| {
            Root::new().show(c, |ui| {
                let d = Dialog::new("dialog", "Rename")
                    .action(DialogAction::new("Cancel"))
                    .action(DialogAction::new("Save").primary())
                    .show(ui, &mut f.dialog_open, |ui| {
                        ui.label("body");
                    });
                if let Some(d) = d {
                    f.actions.extend(d.action);
                }
            });
        });
    };
    for _ in 0..60 {
        step(&mut c, &mut f, &mut t);
    }
    let p = buttons(&c)[0];
    c.move_pointer(p);
    step(&mut c, &mut f, &mut t);
    c.primary_button(ElementState::Pressed);
    step(&mut c, &mut f, &mut t);
    c.primary_button(ElementState::Released);
    for _ in 0..5 {
        step(&mut c, &mut f, &mut t);
    }
    assert_eq!(f.actions, vec![0], "scale {scale}");
}
