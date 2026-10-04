//! Field validation state, `Field` layout and the field-like controls' status styling.
use super::*;
use crate::{
    Button, Checkbox, ColorPicker, ComboBox, ComboBoxOption, DragValue, Field, HoverStyle,
    NumberInput, Response, SemanticStatus, Slider, Switch, TextEdit, TypographyRole, Validation,
    Widget, Window,
};
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}
fn key(c: &mut Context, code: KeyCode) {
    c.on_key_event(code, ElementState::Pressed, false);
    c.on_key_event(code, ElementState::Released, false);
}

/// A field with a text edit followed by a button; returns (edit, button after the field).
fn form(c: &mut Context, field: Field) -> (Response, Response) {
    let mut text = String::from("value");
    let mut out = None;
    c.run(|c| {
        Window::new("Form").show(c, |ui| {
            let edit = field.show(ui, |ui| ui.add(TextEdit::new(&mut text).id_source("edit")));
            let after = ui.button("After");
            out = Some((edit, after));
        });
    });
    out.unwrap()
}
fn small_height(c: &mut Context, text: &str) -> f32 {
    let size = c.style().typography.small;
    c.measure_text(text, size, crate::FontWeight::REGULAR, 400.0)
        .y
}

#[test]
fn a_message_adds_exactly_its_height_and_the_gap_below_the_control() {
    let mut c = setup();
    let (_, plain) = form(&mut c, Field::new("Email"));
    let (_, with) = form(
        &mut c,
        Field::new("Email").validation(Validation::error("Required")),
    );
    let gap = c.style().spacing * 0.5;
    let expected = gap + small_height(&mut c, "Required");
    let jump = with.rect.min.y - plain.rect.min.y;
    assert!(
        (jump - expected).abs() < 0.51,
        "jump {jump}, message {expected}"
    );
    let (_, cleared) = form(&mut c, Field::new("Email"));
    assert_eq!(
        cleared.rect.min.y, plain.rect.min.y,
        "disappearing restores the layout exactly"
    );
}

#[test]
fn a_hint_and_an_error_share_one_slot_and_a_reserved_line_never_jumps() {
    let mut c = setup();
    let hint = Field::new("Email").hint("We never share it");
    let (_, with_hint) = form(&mut c, hint);
    let (_, with_error) = form(
        &mut c,
        Field::new("Email")
            .hint("We never share it")
            .validation(Validation::error("Invalid")),
    );
    assert_eq!(
        with_hint.rect.min.y, with_error.rect.min.y,
        "the message replaces the hint"
    );
    let (_, reserved_idle) = form(&mut c, Field::new("Email").reserve_message(true));
    let (_, reserved_error) = form(
        &mut c,
        Field::new("Email")
            .reserve_message(true)
            .validation(Validation::error("Invalid")),
    );
    assert!((reserved_idle.rect.min.y - reserved_error.rect.min.y).abs() < 0.51);
}

#[test]
fn messages_never_ask_for_frames_by_themselves() {
    let mut c = setup();
    form(
        &mut c,
        Field::new("Email").validation(Validation::error("Required")),
    );
    form(
        &mut c,
        Field::new("Email").validation(Validation::error("Required")),
    );
    assert!(!c.needs_repaint());
    assert!(c.next_repaint().is_none());
}

#[test]
fn validation_converts_from_a_result_and_a_bare_status() {
    let bad: Result<u8, &str> = Err("not a number");
    let good: Result<u8, &str> = Ok(3);
    assert_eq!(Validation::from(&bad), Validation::error("not a number"));
    assert_eq!(Validation::from(&good), Validation::ok());
    assert_eq!(
        Validation::from(SemanticStatus::Warning).status,
        SemanticStatus::Warning
    );
}

/// All vertices of the last pass whose color matches `color` (linear, any alpha).
fn tinted(c: &Context, color: crate::Color) -> usize {
    let linear = color.linear();
    c.draw_data()
        .vertices
        .iter()
        .filter(|v| (0..3).all(|i| (v.color[i] - linear[i]).abs() < 0.02) && v.color[3] > 0.0)
        .count()
}

fn edit_pass(c: &mut Context, status: SemanticStatus, enabled: bool) {
    let mut text = String::from("value");
    c.run(|c| {
        Window::new("Status").show(c, |ui| {
            ui.add(
                TextEdit::new(&mut text)
                    .id_source("e")
                    .status(status)
                    .enabled(enabled),
            );
        });
    });
}

#[test]
fn invalid_controls_get_a_status_border_and_a_soft_ring_and_the_default_theme_is_untouched() {
    let mut c = setup();
    let error = c.style().error;
    edit_pass(&mut c, SemanticStatus::Normal, true);
    edit_pass(&mut c, SemanticStatus::Normal, true);
    let (normal_vertices, normal_tint) = (c.draw_data().vertices.len(), tinted(&c, error));
    assert_eq!(normal_tint, 0, "no status color without a status");
    edit_pass(&mut c, SemanticStatus::Error, true);
    edit_pass(&mut c, SemanticStatus::Error, true);
    assert!(
        tinted(&c, error) > 0,
        "border and ring use the theme error color"
    );
    assert!(
        c.draw_data().vertices.len() > normal_vertices,
        "the ring is extra geometry"
    );
}

#[test]
fn status_combines_with_disabled_focus_and_hover() {
    let mut c = setup();
    let error = c.style().error;
    edit_pass(&mut c, SemanticStatus::Error, true);
    edit_pass(&mut c, SemanticStatus::Error, true);
    let enabled_tint = tinted(&c, error);
    // Disabled keeps the border but draws no ring.
    edit_pass(&mut c, SemanticStatus::Error, false);
    edit_pass(&mut c, SemanticStatus::Error, false);
    let disabled_tint = tinted(&c, error);
    assert!(disabled_tint > 0, "the status border survives disabled");
    assert!(disabled_tint < enabled_tint, "no ring while disabled");
    // Focused: still the status color, with its ring, not the focus color.
    edit_pass(&mut c, SemanticStatus::Error, true);
    key(&mut c, KeyCode::Tab);
    edit_pass(&mut c, SemanticStatus::Error, true);
    edit_pass(&mut c, SemanticStatus::Error, true);
    assert!(tinted(&c, error) > 0);
    let focus = c.style().focus_border.color;
    assert_eq!(
        tinted(&c, focus),
        0,
        "focus does not replace the status color"
    );
    // Hovered: the status border stays.
    c.move_pointer(Vec2::new(40.0, 50.0));
    edit_pass(&mut c, SemanticStatus::Error, true);
    assert!(tinted(&c, error) > 0);
}

#[test]
fn every_field_like_control_takes_the_status_of_its_field() {
    let mut c = setup();
    let error = c.style().error;
    let (mut checked, mut on, mut v) = (false, false, 3.0_f32);
    let (mut n, mut d, mut sel) = (1_i32, 2_i32, Some(1_u8));
    let options = [ComboBoxOption::new(1, 1_u8, "One")];
    let mut counts = Vec::new();
    for status in [Validation::ok(), Validation::error("bad")] {
        for _ in 0..2 {
            c.run(|c| {
                Window::new("All").show(c, |ui| {
                    Field::new("Group")
                        .validation(status.clone())
                        .show(ui, |ui| {
                            ui.add(Checkbox::new(&mut checked, "check"));
                            ui.add(Switch::new(&mut on, "switch"));
                            ui.add(Slider::new(&mut v, 0.0..=10.0));
                            ui.add(NumberInput::new(&mut n));
                            ui.add(DragValue::new(&mut d));
                            ui.add(ComboBox::new(&mut sel, &options));
                        });
                    ui.add(Button::new("outside").tooltip("not in the field"));
                });
            });
        }
        counts.push(tinted(&c, error));
    }
    assert!(counts[1] > counts[0], "{counts:?}");
}

#[test]
fn a_control_status_wins_over_its_field_and_buttons_ignore_the_field() {
    let mut c = setup();
    let (warning, error) = (c.style().warning, c.style().error);
    let mut text = String::new();
    for _ in 0..2 {
        c.run(|c| {
            Window::new("Mixed").show(c, |ui| {
                Field::new("F")
                    .validation(Validation::error("e"))
                    .show(ui, |ui| {
                        ui.add(
                            TextEdit::new(&mut text)
                                .id_source("t")
                                .status(SemanticStatus::Warning),
                        );
                        ui.button("Plain button");
                    });
            });
        });
    }
    assert!(
        tinted(&c, warning) > 0,
        "the control's own status wins over the field's"
    );
    let _ = error;
}

#[test]
fn enabled_is_the_one_way_to_switch_a_control_off_everywhere() {
    let mut c = setup();
    let (mut checked, mut on, mut v, mut text) = (false, false, 1.0_f32, String::new());
    let (mut n, mut d) = (1_i32, 1_i32);
    let (mut sel, options) = (None::<u8>, [ComboBoxOption::new(1, 1_u8, "One")]);
    let mut color = crate::Color::WHITE;
    let mut responses = Vec::new();
    c.run(|c| {
        Window::new("Off").show(c, |ui| {
            responses.push(ui.add(Button::new("b").enabled(false)));
            responses.push(ui.add(Checkbox::new(&mut checked, "c").enabled(false)));
            responses.push(ui.add(Switch::new(&mut on, "s").enabled(false)));
            responses.push(ui.add(Slider::new(&mut v, 0.0..=1.0).enabled(false)));
            responses.push(ui.add(TextEdit::new(&mut text).enabled(false)));
            responses.push(ui.add(NumberInput::new(&mut n).enabled(false)));
            responses.push(ui.add(DragValue::new(&mut d).enabled(false)));
            responses.push(ui.add(ComboBox::new(&mut sel, &options).enabled(false)));
            responses.push(ui.add(ColorPicker::new(&mut color, "color").enabled(false)));
            ui.add_enabled_ui(false, |ui| responses.push(ui.button("group")));
        });
    });
    assert_eq!(responses.len(), 10);
    assert!(responses.iter().all(|r| !r.enabled), "{responses:?}");
}

#[test]
fn matching_builders_exist_with_matching_types_on_every_field_like_control() {
    // Compile-time surface: the same names and argument types everywhere.
    let (mut b, mut t, mut n, mut m, mut f) = (false, String::new(), 1_i32, 2_i32, 1.0_f32);
    let mut sel = None::<u8>;
    let options = [ComboBoxOption::new(1, 1_u8, "One")];
    let hover = HoverStyle::NONE;
    let radius = 4.0_f32;
    let _ = (
        Button::new("x")
            .id_source(1)
            .width(80.0)
            .corner_radius(radius)
            .hover_style(hover)
            .status(SemanticStatus::Error)
            .enabled(true),
        Checkbox::new(&mut b, "x")
            .id_source(1)
            .corner_radius(radius)
            .hover_style(hover)
            .status(SemanticStatus::Error)
            .enabled(true),
        TextEdit::new(&mut t)
            .id_source(1)
            .width(80.0)
            .corner_radius(radius)
            .hover_style(hover)
            .status(SemanticStatus::Error)
            .enabled(true),
        NumberInput::new(&mut n)
            .id_source(1)
            .width(80.0)
            .corner_radius(radius)
            .hover_style(hover)
            .status(SemanticStatus::Error)
            .enabled(true),
        DragValue::new(&mut m)
            .id_source(1)
            .width(80.0)
            .corner_radius(radius)
            .hover_style(hover)
            .status(SemanticStatus::Error)
            .enabled(true),
        ComboBox::new(&mut sel, &options)
            .id_source(1)
            .width(80.0)
            .corner_radius(radius)
            .hover_style(hover)
            .status(SemanticStatus::Error)
            .enabled(true),
        Slider::new(&mut f, 0.0..=1.0)
            .id_source(1)
            .width(80.0)
            .hover_style(hover)
            .status(SemanticStatus::Error)
            .enabled(true),
    );
    let mut on = false;
    let _ = Switch::new(&mut on, "x")
        .id_source(1)
        .corner_radius(radius)
        .hover_style(hover)
        .status(SemanticStatus::Error)
        .enabled(true);
}

#[test]
fn typography_helpers_use_the_theme_scale_not_numbers_in_the_call() {
    let mut c = setup();
    let mut heights = Vec::new();
    let typography = c.style().typography;
    c.run(|c| {
        Window::new("Type").show(c, |ui| {
            for role in [
                TypographyRole::Small,
                TypographyRole::Body,
                TypographyRole::Heading,
                TypographyRole::Title,
            ] {
                heights.push(ui.typography(role, "Ag").rect.size().y);
            }
            assert_eq!(ui.heading("Ag").rect.size().y, heights[2]);
            assert_eq!(ui.title("Ag").rect.size().y, heights[3]);
            assert_eq!(ui.small("Ag").rect.size().y, heights[0]);
        });
    });
    assert!(
        heights.windows(2).all(|w| w[0] < w[1]),
        "{heights:?} for {typography:?}"
    );
}
