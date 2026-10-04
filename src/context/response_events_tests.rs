//! `Response` events: double and secondary clicks, drag start/delta/stop and focus.
//! Every event is reported by exactly one pass per user input.
use super::*;
use crate::{
    Button, Checkbox, ContextMenuItem, DragSource, Id, Padding, Response, Root, Slider, Text,
    Widget, Window,
};
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}

struct Model {
    value: f32,
    checked: bool,
    last_menu: Option<Id>,
}
struct Seen {
    button: Response,
    other: Response,
    slider: Response,
    checkbox: Response,
    text: Response,
}

fn draw(c: &mut Context, m: &mut Model) -> Seen {
    let items = [ContextMenuItem::new("copy", "Copy")];
    let mut seen = None;
    c.run(|c| {
        Window::new("Test").show(c, |ui| {
            let button = ui.add(Button::new("First").context_menu(&items));
            let other = ui.button("Second");
            let slider = ui.add(Slider::new(&mut m.value, 0.0..=100.0).width(200.0));
            let checkbox = ui.add(Checkbox::new(&mut m.checked, "Check"));
            let text = ui.add(Text::new("passive"));
            m.last_menu = m.last_menu.or(button.menu_selected());
            seen = Some(Seen {
                button,
                other,
                slider,
                checkbox,
                text,
            });
        });
    });
    seen.unwrap()
}
fn model() -> Model {
    Model {
        value: 0.0,
        checked: false,
        last_menu: None,
    }
}
fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}
fn right_click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.secondary_button(ElementState::Pressed);
    c.secondary_button(ElementState::Released);
}

#[test]
fn double_click_is_reported_once_by_the_second_click() {
    let mut c = setup();
    let mut m = model();
    let at = draw(&mut c, &mut m).button.rect.center();
    click(&mut c, at);
    let first = draw(&mut c, &mut m).button;
    assert!(first.clicked() && !first.double_clicked());
    click(&mut c, at);
    let second = draw(&mut c, &mut m).button;
    assert!(second.clicked() && second.double_clicked());
    let after = draw(&mut c, &mut m).button;
    assert!(!after.clicked() && !after.double_clicked());
    // A third click starts a new sequence instead of being another double click.
    click(&mut c, at);
    assert!(!draw(&mut c, &mut m).button.double_clicked());
}

#[test]
fn slow_or_distant_clicks_are_not_double_clicks() {
    let mut c = setup();
    let mut m = model();
    let seen = draw(&mut c, &mut m);
    let at = seen.button.rect.center();
    click(&mut c, at);
    draw(&mut c, &mut m);
    std::thread::sleep(Duration::from_millis(560));
    click(&mut c, at);
    assert!(!draw(&mut c, &mut m).button.double_clicked());
    click(&mut c, seen.other.rect.center());
    click(&mut c, at);
    assert!(
        !draw(&mut c, &mut m).button.double_clicked(),
        "a click elsewhere in between breaks the sequence"
    );
}

#[test]
fn double_click_toggles_a_checkbox_twice_with_changed_on_both_passes() {
    let mut c = setup();
    let mut m = model();
    let at = draw(&mut c, &mut m).checkbox.rect.center();
    click(&mut c, at);
    let first = draw(&mut c, &mut m).checkbox;
    assert!(first.clicked() && first.changed() && m.checked);
    click(&mut c, at);
    let second = draw(&mut c, &mut m).checkbox;
    assert!(second.double_clicked() && second.changed() && !m.checked);
}

#[test]
fn secondary_click_fires_on_release_over_the_same_widget_once() {
    let mut c = setup();
    let mut m = model();
    let seen = draw(&mut c, &mut m);
    let at = seen.other.rect.center();
    c.move_pointer(at);
    c.secondary_button(ElementState::Pressed);
    assert!(
        !draw(&mut c, &mut m).other.secondary_clicked(),
        "not on press"
    );
    c.secondary_button(ElementState::Released);
    let released = draw(&mut c, &mut m);
    assert!(released.other.secondary_clicked());
    assert!(
        !released.other.clicked(),
        "a secondary click is not a click"
    );
    assert!(!draw(&mut c, &mut m).other.secondary_clicked());
    // Pressed on one widget and released over another: no secondary click for either.
    c.move_pointer(at);
    c.secondary_button(ElementState::Pressed);
    c.move_pointer(seen.checkbox.rect.center());
    c.secondary_button(ElementState::Released);
    let seen = draw(&mut c, &mut m);
    assert!(!seen.other.secondary_clicked() && !seen.checkbox.secondary_clicked());
    // A primary click never reports a secondary one.
    click(&mut c, at);
    assert!(!draw(&mut c, &mut m).other.secondary_clicked());
}

#[test]
fn context_menu_opens_on_the_same_secondary_signal_and_reports_the_choice() {
    let mut c = setup();
    let mut m = model();
    let at = draw(&mut c, &mut m).button.rect.center();
    c.move_pointer(at);
    c.secondary_button(ElementState::Pressed);
    draw(&mut c, &mut m);
    assert!(c.popup.is_none(), "the menu waits for the release");
    c.secondary_button(ElementState::Released);
    let opened = draw(&mut c, &mut m);
    assert!(opened.button.secondary_clicked());
    draw(&mut c, &mut m);
    assert!(c.popup.is_some());
    // Enter on the highlighted entry selects it; the widget's response carries the choice.
    c.on_key_event(KeyCode::Enter, ElementState::Pressed, false);
    c.on_key_event(KeyCode::Enter, ElementState::Released, false);
    let chosen = draw(&mut c, &mut m).button;
    assert_eq!(chosen.menu_selected(), Some(Id::new("copy")));
    assert!(draw(&mut c, &mut m).button.menu_selected().is_none());
}

#[test]
fn a_secondary_click_on_passive_text_reaches_only_menu_anchors() {
    let mut c = setup();
    let mut m = model();
    let at = draw(&mut c, &mut m).text.rect.center();
    right_click(&mut c, at);
    assert!(!draw(&mut c, &mut m).text.secondary_clicked());
}

#[test]
fn drag_starts_past_the_threshold_and_the_deltas_sum_to_the_displacement() {
    let mut c = setup();
    let mut m = model();
    let at = draw(&mut c, &mut m).slider.rect.center();
    c.move_pointer(at);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(at + Vec2::new(2.0, 0.0));
    let held = draw(&mut c, &mut m).slider;
    assert!(held.pressed && !held.dragged() && !held.drag_started());
    c.move_pointer(at + Vec2::new(10.0, 3.0));
    let started = draw(&mut c, &mut m).slider;
    assert!(started.drag_started() && started.dragged());
    assert_eq!(started.drag_delta(), Vec2::new(10.0, 3.0));
    c.move_pointer(at + Vec2::new(25.0, 3.0));
    c.move_pointer(at + Vec2::new(30.0, 3.0));
    let moving = draw(&mut c, &mut m).slider;
    assert!(!moving.drag_started() && moving.dragged());
    assert_eq!(moving.drag_delta(), Vec2::new(20.0, 0.0));
    let idle = draw(&mut c, &mut m).slider;
    assert!(idle.dragged() && idle.drag_delta() == Vec2::ZERO && idle.pressed);
    c.primary_button(ElementState::Released);
    let stopped = draw(&mut c, &mut m).slider;
    assert!(stopped.drag_stopped() && stopped.dragged());
    let done = draw(&mut c, &mut m).slider;
    assert!(!done.dragged() && !done.drag_stopped() && !done.drag_started());
}

#[test]
fn a_press_without_drag_reports_no_drag_and_a_dragged_press_is_no_click() {
    let mut c = setup();
    let mut m = model();
    let at = draw(&mut c, &mut m).button.rect.center();
    c.move_pointer(at);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(at + Vec2::new(30.0, 0.0));
    c.move_pointer(at);
    c.primary_button(ElementState::Released);
    let button = draw(&mut c, &mut m).button;
    assert!(button.clicked(), "buttons keep their release-inside click");
    assert!(!button.drag_started() && !button.dragged());
}

#[test]
fn focus_events_follow_tab_and_presses_once() {
    let mut c = setup();
    let mut m = model();
    draw(&mut c, &mut m);
    c.key(KeyCode::Tab, ElementState::Pressed, false);
    c.key(KeyCode::Tab, ElementState::Released, false);
    let first = draw(&mut c, &mut m);
    assert!(first.button.gained_focus() && first.button.has_focus);
    assert!(!draw(&mut c, &mut m).button.gained_focus(), "reported once");
    click(&mut c, first.other.rect.center());
    let moved = draw(&mut c, &mut m);
    assert!(moved.button.lost_focus() && !moved.button.has_focus);
    assert!(moved.other.gained_focus() && moved.other.has_focus);
    assert!(!draw(&mut c, &mut m).button.lost_focus());
    // Clicking empty space drops focus.
    c.move_pointer(Vec2::new(780.0, 580.0));
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    assert!(draw(&mut c, &mut m).other.lost_focus());
}

#[test]
fn new_events_cost_no_extra_passes_when_nothing_happens() {
    let mut c = setup();
    let mut m = model();
    draw(&mut c, &mut m);
    draw(&mut c, &mut m);
    assert!(!c.needs_repaint());
    let before = c.cache_stats();
    draw(&mut c, &mut m);
    let after = c.cache_stats();
    assert_eq!(after.tessellated_elements, before.tessellated_elements);
    assert_eq!(after.geometry_rebuilds, before.geometry_rebuilds);
}

#[test]
fn a_drag_and_drop_source_reports_the_same_drag_signals() {
    let mut c = setup();
    let run = |c: &mut Context| {
        let mut response = None;
        c.run(|c| {
            Root::new().padding(Padding::all(20.0)).show(c, |ui| {
                let out =
                    DragSource::new(Id::new("source"), 7_u32).show(ui, |ui| ui.button("Drag me"));
                response = Some(out.response);
            });
        });
        response.unwrap()
    };
    let at = run(&mut c).rect.center();
    c.move_pointer(at);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(at + Vec2::new(12.0, 0.0));
    let started = run(&mut c);
    assert!(started.drag_started() && started.dragged());
    assert_eq!(started.drag_delta(), Vec2::new(12.0, 0.0));
    c.primary_button(ElementState::Released);
    let stopped = run(&mut c);
    assert!(stopped.drag_stopped());
    assert!(!run(&mut c).drag_stopped());
}
