//! Members that keep keys of their own: a text field and a slider inside a group.
//! The control's own keys win over the group; the group's other members reach them, and
//! Tab leaves.

use super::support::*;
use crate::prelude::*;
use winit::event::ElementState;
use zaxis::{Button, FocusGroup, Root, Slider, TextEdit};

struct Model {
    text: String,
    value: f32,
}

struct Out {
    focus: Option<&'static str>,
}

fn build(c: &mut Context, m: &mut Model) -> Out {
    let mut rs: Vec<(&'static str, Response)> = Vec::new();
    c.run(|c| {
        Root::new().show(c, |ui| {
            rs.push(("before", ui.add(Button::new("before"))));
            FocusGroup::new("find").label("Find").show(ui, |ui| {
                ui.horizontal(|ui| {
                    rs.push(("a", ui.add(Button::new("a"))));
                    rs.push(("text", ui.add(TextEdit::new(&mut m.text).id_source("text"))));
                    rs.push((
                        "slider",
                        ui.add(Slider::new(&mut m.value, 0.0..=1.0).step(0.1)),
                    ));
                    rs.push(("z", ui.add(Button::new("z"))));
                });
            });
            rs.push(("after", ui.add(Button::new("after"))));
        });
    });
    Out {
        focus: rs.iter().find(|(_, r)| r.has_focus).map(|(n, _)| *n),
    }
}

fn model() -> Model {
    Model {
        text: "hello".into(),
        value: 0.5,
    }
}

fn type_text(c: &mut Context, text: &str) {
    for ch in text.chars() {
        let s = ch.to_string();
        let code = KeyCode::KeyX;
        c.on_input(crate::keys::support::key_as(
            code,
            winit::keyboard::Key::Character(s.as_str().into()),
            ElementState::Pressed,
            false,
            Some(&s),
        ));
        up(c, code);
    }
}

#[test]
fn a_neighbour_moves_focus_onto_the_text_field_and_the_field_keeps_its_keys() {
    let mut c = setup();
    let mut m = model();
    build(&mut c, &mut m);
    tab(&mut c);
    tab(&mut c);
    assert_eq!(build(&mut c, &mut m).focus, Some("a"));
    tap(&mut c, KeyCode::ArrowRight);
    assert_eq!(build(&mut c, &mut m).focus, Some("text"));
    tap(&mut c, KeyCode::End);
    tap(&mut c, KeyCode::ArrowLeft);
    tap(&mut c, KeyCode::Home);
    assert_eq!(
        build(&mut c, &mut m).focus,
        Some("text"),
        "caret keys never leave the field"
    );
    type_text(&mut c, "AB");
    build(&mut c, &mut m);
    assert!(
        m.text.contains("AB"),
        "typing still edits the field: {:?}",
        m.text
    );
    assert!(
        m.text.starts_with("ABhello"),
        "Home put the caret first: {:?}",
        m.text
    );
}

#[test]
fn tab_leaves_the_group_from_a_text_field_and_comes_back_to_it() {
    let mut c = setup();
    let mut m = model();
    build(&mut c, &mut m);
    tab(&mut c);
    tab(&mut c);
    tap(&mut c, KeyCode::ArrowRight);
    assert_eq!(build(&mut c, &mut m).focus, Some("text"));
    tab(&mut c);
    assert_eq!(build(&mut c, &mut m).focus, Some("after"));
    shift_tab(&mut c);
    assert_eq!(build(&mut c, &mut m).focus, Some("text"));
}

#[test]
fn the_slider_changes_its_value_and_does_not_start_navigation() {
    let mut c = setup();
    let mut m = model();
    build(&mut c, &mut m);
    tab(&mut c);
    tab(&mut c);
    tap(&mut c, KeyCode::End);
    tap(&mut c, KeyCode::ArrowLeft);
    assert_eq!(build(&mut c, &mut m).focus, Some("slider"));
    let value = m.value;
    tap(&mut c, KeyCode::ArrowRight);
    let out = build(&mut c, &mut m);
    assert_eq!(
        out.focus,
        Some("slider"),
        "the arrow adjusted the slider, focus stayed"
    );
    assert!(m.value > value, "{} > {}", m.value, value);
    tap(&mut c, KeyCode::ArrowLeft);
    tap(&mut c, KeyCode::ArrowLeft);
    build(&mut c, &mut m);
    assert!(m.value < value);
    tap(&mut c, KeyCode::Home);
    assert_eq!(
        build(&mut c, &mut m).focus,
        Some("slider"),
        "Home is the slider's too"
    );
    assert_eq!(m.value, 0.0);
    tab(&mut c);
    assert_eq!(
        build(&mut c, &mut m).focus,
        Some("after"),
        "Tab is how to leave a slider"
    );
}

#[test]
fn a_member_after_the_slider_is_reached_from_the_other_side() {
    let mut c = setup();
    let mut m = model();
    build(&mut c, &mut m);
    tab(&mut c);
    tab(&mut c);
    tap(&mut c, KeyCode::End);
    assert_eq!(build(&mut c, &mut m).focus, Some("z"));
    tap(&mut c, KeyCode::ArrowLeft);
    assert_eq!(
        build(&mut c, &mut m).focus,
        Some("slider"),
        "moves onto the slider"
    );
}
