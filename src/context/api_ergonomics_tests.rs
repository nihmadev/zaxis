//! Shorter call forms, kept compatibility, and values that used to panic.
#![allow(deprecated)]
use super::*;
use crate::{
    Button, Checkbox, Column, ComboBox, ComboBoxOption, Grid, NumberInput, Popup, Presence,
    Progress, ProgressState, Response, ScrollArea, Slider, SplitPane, SplitPanel, SplitSize,
    Switch, Table, TextEdit, TreeView, Widget, Window,
};
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}
fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}
fn key(c: &mut Context, code: KeyCode) {
    c.on_key_event(code, ElementState::Pressed, false);
    c.on_key_event(code, ElementState::Released, false);
}

#[test]
fn integer_sliders_step_by_one_and_snap_without_precision_or_step_calls() {
    let mut c = setup();
    let mut count = 0_i32;
    let mut small = 250_u8;
    let draw = |c: &mut Context, count: &mut i32, small: &mut u8| {
        let mut out = None;
        c.run(|c| {
            Window::new("Ints").show(c, |ui| {
                let a = ui.add(Slider::new(count, 0..=10).width(200.0).text("n"));
                let b = ui.slider(small, 0..=255);
                out = Some((a, b));
            });
        });
        out.unwrap()
    };
    let (a, b) = draw(&mut c, &mut count, &mut small);
    assert_eq!(small, 250, "idle values are untouched");
    click(&mut c, a.rect.center());
    draw(&mut c, &mut count, &mut small);
    assert_eq!(count, 5, "the middle of 0..=10 is a whole number");
    c.move_pointer(a.rect.center());
    // The click focused the slider; arrows move by exactly one (not by span / 100 = 0.1).
    key(&mut c, KeyCode::ArrowRight);
    draw(&mut c, &mut count, &mut small);
    assert_eq!(count, 6);
    key(&mut c, KeyCode::PageDown);
    draw(&mut c, &mut count, &mut small);
    assert_eq!(count, 0, "ten steps down, clamped");
    key(&mut c, KeyCode::End);
    draw(&mut c, &mut count, &mut small);
    assert_eq!(count, 10);
    click(&mut c, Vec2::new(b.rect.max.x + 100.0, b.rect.center().y));
    click(&mut c, Vec2::new(b.rect.max.x - 1.0, b.rect.center().y));
    draw(&mut c, &mut count, &mut small);
    assert_eq!(small, 255);
}

#[test]
fn float_sliders_keep_their_free_movement_and_a_nan_value_is_repaired() {
    let mut c = setup();
    let mut value = f32::NAN;
    c.run(|c| {
        Window::new("F").show(c, |ui| {
            ui.slider(&mut value, 0.0..=1.0);
        });
    });
    assert_eq!(value, 0.0);
    let mut x = 0.123_f32;
    c.run(|c| {
        Window::new("F").show(c, |ui| {
            ui.slider(&mut x, 0.0..=1.0);
        });
    });
    assert_eq!(x, 0.123, "no implicit step for floats");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Mode {
    Fast,
    Exact,
    Custom,
}

#[test]
fn a_combo_box_over_pairs_binds_directly_to_an_enum() {
    let mut c = setup();
    let mut mode = Mode::Fast;
    let draw = |c: &mut Context, mode: &mut Mode| {
        let mut out = None;
        c.run(|c| {
            Window::new("Pairs").show(c, |ui| {
                out = Some(ui.combo_box_values(
                    mode,
                    [
                        (Mode::Fast, "Fast"),
                        (Mode::Exact, "Exact"),
                        (Mode::Custom, "Custom"),
                    ],
                ));
            });
        });
        out.unwrap()
    };
    draw(&mut c, &mut mode);
    let trigger = draw(&mut c, &mut mode);
    click(&mut c, trigger.rect.center());
    draw(&mut c, &mut mode);
    assert!(c.popup.is_some(), "opens like any combo box");
    key(&mut c, KeyCode::ArrowDown);
    key(&mut c, KeyCode::Enter);
    let after = draw(&mut c, &mut mode);
    assert_eq!(mode, Mode::Exact);
    assert!(after.changed());
    assert!(!draw(&mut c, &mut mode).changed(), "reported once");
    // The builder form accepts the same pairs with an optional binding and all options.
    let mut optional = None;
    c.run(|c| {
        Window::new("Builder").show(c, |ui| {
            ui.add(
                ComboBox::from_pairs(&mut optional, [(1_u8, "One"), (2, "Two")])
                    .label("pick")
                    .width(200.0),
            );
        });
    });
    assert_eq!(optional, None, "nothing selected until the user picks");
    let _ = ComboBoxOption::new(1, 1, "kept");
}

#[test]
fn deprecated_enabled_helpers_still_work_and_match_the_builders() {
    let mut c = setup();
    let (mut checked, mut on, mut v) = (false, false, 0.5_f32);
    let mut out = Vec::new();
    c.run(|c| {
        Window::new("Legacy").show(c, |ui| {
            out.push(ui.button_enabled(false, "a"));
            out.push(ui.checkbox_enabled(false, &mut checked, "b"));
            out.push(ui.switch_enabled(false, &mut on, "c"));
            out.push(ui.slider_enabled(false, &mut v, 0.0..=1.0));
            out.push(ui.add_enabled(false, Button::new("d")));
            out.push(ui.add(TextEdit::new(&mut String::new()).disabled(true)));
            out.push(ui.add(Button::new("e").rounding(crate::CornerRadius::all(2.0))));
        });
    });
    assert!(out[..6].iter().all(|r: &Response| !r.enabled));
    assert!(out[6].enabled);
}

#[test]
fn tooltip_in_one_expression_shows_after_the_hover_delay() {
    let mut c = setup();
    let t = std::time::Instant::now();
    let draw = |c: &mut Context, at: u64| {
        let mut out = None;
        c.run_at(t + Duration::from_millis(at), |c| {
            Window::new("Tip").show(c, |ui| {
                out = Some(ui.add(Button::new("Save").tooltip("Write the file to disk")));
            });
        });
        out.unwrap()
    };
    let r = draw(&mut c, 0);
    let plain = c.draw_data().vertices.len();
    c.move_pointer(r.rect.center());
    draw(&mut c, 10);
    assert_eq!(c.draw_data().vertices.len(), plain, "not yet");
    draw(&mut c, 600);
    assert!(
        c.draw_data().vertices.len() > plain,
        "tooltip is drawn once the delay has passed"
    );
}

#[test]
fn invalid_values_everywhere_are_normalized_and_the_geometry_stays_finite() {
    let mut c = setup();
    let bad = [f32::NAN, f32::INFINITY, -4.0, 0.0];
    for (round, value) in bad.into_iter().enumerate() {
        let (mut text, mut n, mut sel) = (String::new(), 3_i32, None::<u8>);
        let options = [ComboBoxOption::new(1, 1_u8, "One")];
        let mut open = true;
        c.run(|c| {
            Window::new("Bad").show(c, |ui| {
                ui.add(Button::new("b").width(value));
                ui.add(
                    TextEdit::new(&mut text)
                        .width(value)
                        .height(value)
                        .font_size(value),
                );
                ui.add(
                    NumberInput::new(&mut n)
                        .width(value)
                        .step(0)
                        .sensitivity(value as f64),
                );
                ui.add(
                    ComboBox::new(&mut sel, &options)
                        .width(value)
                        .row_height(value)
                        .trigger_height(value)
                        .row_gap(value)
                        .label_gap(value)
                        .font_size(value),
                );
                ui.add(
                    Progress::new(ProgressState::Determinate(value)).size(Vec2::new(value, value)),
                );
                Presence::fade()
                    .scaling(value)
                    .rotating(value)
                    .offset(Vec2::splat(value))
                    .pivot(Vec2::splat(value))
                    .motion(
                        crate::TweenOptions::new(Duration::from_millis(1))
                            .repeat(crate::Repeat::Forever),
                    )
                    .show(ui, "p", true, |ui| ui.label("p"));
                ScrollArea::vertical()
                    .id_source("rows")
                    .max_height(80.0)
                    .show_rows(ui, value, 50, |ui, i| {
                        ui.label(format!("row {i}"));
                    });
                let _ = TreeView::new("tree").row_height(value);
                Popup::new(
                    "p",
                    crate::Rect::from_min_size(Vec2::splat(20.0), Vec2::splat(30.0)),
                )
                .size(Vec2::new(value, value))
                .gap(value)
                .show(ui, &mut open, |ui| ui.label("x"));
                SplitPane::horizontal("split")
                    .panels([
                        SplitPanel::new(1)
                            .default_size(SplitSize::Weight(value))
                            .min_size(value),
                        SplitPanel::new(1).default_size(SplitSize::Pixels(value)),
                        SplitPanel::new(2)
                            .default_size(SplitSize::Fraction(value))
                            .max_size(value),
                    ])
                    .show(ui, |split| {
                        split.panel(1, |ui| ui.label("a"));
                        split.panel(1, |ui| ui.label("duplicate"));
                        split.panel(99, |ui| ui.label("unknown"));
                    });
                Grid::new("g").show(ui, |grid| {
                    grid.row("r", |row| {
                        row.cell(|ui| ui.label("no columns: one fallback column"));
                        row.cell(|ui| ui.label("an extra cell"));
                    });
                });
                Grid::new("g2")
                    .columns([
                        Column::fixed("a", value),
                        Column::fixed("a", 10.0),
                        Column::remainder("b").min_width(value),
                    ])
                    .width(value)
                    .show(ui, |grid| {
                        grid.row("r", |row| {
                            row.cell(|ui| ui.label("1"));
                        });
                    });
                Table::new("t")
                    .columns([Column::fixed("x", value)])
                    .show_rows(ui, value, 5, |body, i| {
                        body.row(i, |row| {
                            row.cell(|ui| ui.label("cell"));
                        });
                    });
                ui.add(Checkbox::new(&mut false, "c").size(value));
                ui.add(Switch::new(&mut false, "s").size(value));
                ui.visual("v", crate::Transform::IDENTITY, value, |ui| ui.label("v"));
            });
        });
        assert!(
            c.draw_data()
                .vertices
                .iter()
                .all(|v| v.position.iter().all(|p| p.is_finite())),
            "round {round} with {value}"
        );
        assert!(
            c.diagnostics()
                .iter()
                .any(|d| d.kind == crate::DiagnosticKind::InvalidValue),
            "round {round}"
        );
    }
}
