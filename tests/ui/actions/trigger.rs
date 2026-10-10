use super::{support::*, widgets::*};
use crate::prelude::*;
use std::cell::Cell;
use zaxis::{ActionSource, Button, MenuBar, MenuItem};

const ALL: [Cmd; 3] = [Cmd::Save, Cmd::Open, Cmd::Delete];

#[test]
fn a_key_raises_one_event_that_is_taken_once() {
    let mut c = setup();
    idle(&mut c);
    assert!(press(&mut c, CTRL, KeyCode::KeyS), "the key was used");
    assert_eq!(taken(&mut c, &ALL), [Cmd::Save]);
    assert_eq!(taken(&mut c, &ALL), [], "gone after it was taken");
}

#[test]
fn asking_twice_in_one_pass_answers_true_once() {
    let mut c = setup();
    idle(&mut c);
    press(&mut c, CTRL, KeyCode::KeyS);
    let mut answers = Vec::new();
    pass(&mut c, |ui| {
        answers = (0..3).map(|_| ui.actions().triggered(Cmd::Save)).collect();
    });
    assert_eq!(answers, [true, false, false]);
}

#[test]
fn an_event_nobody_takes_lapses_after_a_pass() {
    let mut c = setup();
    idle(&mut c);
    press(&mut c, CTRL, KeyCode::KeyS);
    idle(&mut c);
    assert_eq!(taken(&mut c, &ALL), []);
}

#[test]
fn a_programmatic_trigger_takes_the_same_path() {
    let mut c = setup();
    idle(&mut c);
    assert!(c.actions().trigger(Cmd::Open));
    let mut source = None;
    pass(&mut c, |ui| source = ui.actions().take(Cmd::Open));
    assert_eq!(source, Some(ActionSource::Programmatic));
    assert!(!c.actions().trigger("not declared"));
}

#[test]
fn the_event_of_a_widget_reaches_code_that_asks_before_or_after_it() {
    for ask_first in [true, false] {
        let mut c = setup();
        let mut seen = Vec::new();
        for round in 0..4 {
            if round == 1 {
                let spot = rows_of_button(&c)[0].center();
                click(&mut c, spot);
            }
            pass(&mut c, |ui| {
                if ask_first {
                    seen.push(ui.actions().triggered(Cmd::Save));
                }
                ui.add(Button::action(Cmd::Save));
                if !ask_first {
                    seen.push(ui.actions().triggered(Cmd::Save));
                }
            });
        }
        assert_eq!(seen.iter().filter(|taken| **taken).count(), 1, "{seen:?}");
    }
}

#[test]
fn one_gesture_is_one_effect_with_the_action_on_three_widgets() {
    let mut c = setup();
    let effects = Cell::new(0);
    let frame = |c: &mut Context| {
        pass(c, |ui| {
            let items = [MenuItem::submenu("File", [MenuItem::action(Cmd::Save)])];
            MenuBar::new("bar", &items).show(ui);
            ui.add(Button::action(Cmd::Save));
            ui.add(Button::new("Also save").triggers(Cmd::Save));
            // Three places that handle the same action: only the first one gets it.
            for _ in 0..3 {
                effects.set(effects.get() + usize::from(ui.actions().triggered(Cmd::Save)));
            }
        });
    };
    frame(&mut c);
    let buttons = rows_of_button(&c);
    click(&mut c, buttons[1].center());
    frame(&mut c);
    frame(&mut c);
    assert_eq!(effects.get(), 1);
    press(&mut c, CTRL, KeyCode::KeyS);
    frame(&mut c);
    frame(&mut c);
    assert_eq!(effects.get(), 2, "a key is a second gesture");
}

#[test]
fn enabled_and_checked_last_one_pass() {
    let mut c = setup();
    pass(&mut c, |ui| {
        ui.actions().set_enabled(Cmd::Save, false);
        ui.actions().set_checked(Cmd::Open, true);
        assert!(!ui.actions().is_enabled(Cmd::Save));
        assert_eq!(ui.actions().checked(Cmd::Open), Some(true));
    });
    // The application stopped setting them: nothing sticks.
    pass(&mut c, |ui| {
        assert!(ui.actions().is_enabled(Cmd::Save));
        assert_eq!(ui.actions().checked(Cmd::Open), None);
    });
    assert!(press(&mut c, CTRL, KeyCode::KeyS));
}

#[test]
fn a_disabled_action_does_not_run_from_a_key_or_from_code() {
    let mut c = setup();
    pass(&mut c, |ui| ui.actions().set_enabled(Cmd::Save, false));
    assert!(!press(&mut c, CTRL, KeyCode::KeyS), "the key is left alone");
    assert!(!c.actions().trigger(Cmd::Save));
    assert_eq!(taken(&mut c, &ALL), []);
    // In the middle of a pass the same rule holds.
    pass(&mut c, |ui| {
        ui.actions().set_enabled(Cmd::Save, false);
        assert!(!ui.actions().trigger(Cmd::Save));
        assert!(ui.actions().trigger(Cmd::Open));
    });
}

#[test]
fn a_held_key_repeats_without_repeating_the_action() {
    let mut c = setup();
    idle(&mut c);
    c.on_input(zaxis::InputEvent::Modifiers(CTRL));
    let down = key_event(
        KeyCode::KeyS,
        winit::keyboard::Key::Character("s".into()),
        ElementState::Pressed,
    );
    c.on_input(down.clone());
    let zaxis::InputEvent::Key(mut repeat) = down else {
        unreachable!()
    };
    repeat.repeat = true;
    assert!(
        c.on_input(zaxis::InputEvent::Key(repeat)).consumed,
        "repeats are swallowed"
    );
    assert_eq!(taken(&mut c, &ALL), [Cmd::Save]);
}
