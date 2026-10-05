//! Pass lifecycle of the Context owners: retained state follows its widget, routed input
//! that nobody takes is dropped, focus and capture settle against the published regions,
//! and a repaint asked for while building survives the pass.
use crate::prelude::*;
use winit::{dpi::PhysicalSize, event::ElementState};
use zaxis::{ColorPicker, ComboBox, ComboBoxOption, Slider, TextEdit, Window};

struct Model {
    text: String,
    choice: Option<u32>,
    color: Color,
    value: f32,
}

fn context() -> Context {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    context
}

/// One pass. With `shown` false the controls are not built at all.
fn pass(context: &mut Context, model: &mut Model, shown: bool) -> Vec<Response> {
    let options = [
        ComboBoxOption::new(1, 1, "One"),
        ComboBoxOption::new(2, 2, "Two"),
    ];
    let mut responses = Vec::new();
    context.run(|context| {
        Window::new("Lifecycle").show(context, |ui| {
            if shown {
                responses.push(ui.add(TextEdit::new(&mut model.text).id_source("text")));
                responses
                    .push(ui.add(ComboBox::new(&mut model.choice, &options).id_source("combo")));
                responses
                    .push(ui.add(ColorPicker::new(&mut model.color, "Color").id_source("color")));
                responses.push(ui.add(Slider::new(&mut model.value, 0.0..=1.0)));
            }
            ui.button("Stays");
        });
    });
    responses
}

fn model() -> Model {
    Model {
        text: String::new(),
        choice: None,
        color: Color::rgb(10, 20, 30),
        value: 0.5,
    }
}

#[test]
fn retained_state_is_dropped_with_its_widget_and_starts_fresh_when_it_returns() {
    let (mut c, mut m) = (context(), model());
    pass(&mut c, &mut m, true);
    let probe = c.probe();
    let counts = (
        probe.text_edits.len(),
        probe.combo_boxes.len(),
        probe.color_pickers.len(),
    );
    assert_eq!(counts, (1, 1, 1));
    pass(&mut c, &mut m, false);
    let probe = c.probe();
    assert!(probe.text_edits.is_empty());
    assert!(probe.combo_boxes.is_empty());
    assert!(probe.color_pickers.is_empty());
    // Passes without the widgets do not grow the mesh cache.
    let cached = c.probe().cache.len();
    for _ in 0..5 {
        pass(&mut c, &mut m, false);
    }
    assert_eq!(c.probe().cache.len(), cached);
    pass(&mut c, &mut m, true);
    let probe = c.probe();
    assert_eq!(probe.text_edits.len(), 1);
    assert_eq!(probe.combo_boxes.len(), 1);
    assert_eq!(probe.color_pickers.len(), 1);
}

#[test]
fn a_focused_field_that_vanishes_loses_focus_and_returns_unfocused() {
    let (mut c, mut m) = (context(), model());
    let field = pass(&mut c, &mut m, true)[0];
    c.request_focus(field.id);
    assert!(pass(&mut c, &mut m, true)[0].has_focus);
    pass(&mut c, &mut m, false);
    assert_eq!(c.probe().focused_widget, None);
    let field = pass(&mut c, &mut m, true)[0];
    assert!(!field.has_focus);
    // Text typed now has no field to go to.
    assert!(!c.on_text_event("x").consumed);
    pass(&mut c, &mut m, true);
    assert_eq!(m.text, "");
}

#[test]
fn routed_input_of_a_control_that_is_not_built_is_dropped() {
    let (mut c, mut m) = (context(), model());
    let slider = pass(&mut c, &mut m, true)[3];
    c.request_focus(slider.id);
    pass(&mut c, &mut m, true);
    assert!(c.key(KeyCode::ArrowRight, ElementState::Pressed, false));
    assert!(!c.probe().slider_input.is_empty());
    pass(&mut c, &mut m, false);
    assert!(
        c.probe().slider_input.is_empty(),
        "queued keys were dropped"
    );
    assert_eq!(m.value, 0.5);
    pass(&mut c, &mut m, true);
    assert_eq!(
        m.value, 0.5,
        "a returning slider does not replay stale input"
    );
}

#[test]
fn a_capture_ends_when_its_widget_vanishes_and_not_while_it_is_built() {
    let (mut c, mut m) = (context(), model());
    let slider = pass(&mut c, &mut m, true)[3];
    c.move_pointer(slider.rect.center());
    assert!(c.primary_button(ElementState::Pressed));
    assert!(c.probe().capture.is_some());
    c.move_pointer(slider.rect.center() + vec2(400.0, 300.0));
    pass(&mut c, &mut m, true);
    assert!(
        c.probe().capture.is_some(),
        "a capture holds outside its rect"
    );
    pass(&mut c, &mut m, false);
    assert!(c.probe().capture.is_none());
    assert!(!c.primary_button(ElementState::Released));
}

#[test]
fn a_repaint_requested_while_building_survives_the_pass() {
    let mut c = context();
    c.run(|_| {});
    c.run(|_| {});
    assert!(!c.needs_repaint());
    c.run(|c| c.request_repaint());
    assert!(c.needs_repaint());
    c.run(|_| {});
    assert!(!c.needs_repaint());
}
