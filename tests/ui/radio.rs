use crate::prelude::*;
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};
use zaxis::{DiagnosticKind, RadioGroup, RadioLayout, RadioOption, Window};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Pick {
    A,
    B,
    C,
    D,
    Other,
}

type Tweak = fn(RadioGroup<'_, Pick>) -> RadioGroup<'_, Pick>;

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}

fn options() -> Vec<RadioOption<Pick>> {
    vec![
        RadioOption::new(Pick::A, "Alpha"),
        RadioOption::new(Pick::B, "Beta"),
        RadioOption::new(Pick::C, "Gamma"),
    ]
}

fn draw_with(
    c: &mut Context,
    pick: &mut Pick,
    options: Vec<RadioOption<Pick>>,
    tweak: Tweak,
    at: u64,
) -> zaxis::Response {
    let start = c.frame_time();
    let mut response = None;
    c.run_at(start + Duration::from_millis(at), |c| {
        Window::new("Test").show(c, |ui| {
            ui.button("Before");
            response = Some(ui.add(tweak(RadioGroup::new(pick, options))));
            ui.button("After");
        });
    });
    response.unwrap()
}

fn draw(c: &mut Context, pick: &mut Pick, at: u64) -> zaxis::Response {
    draw_with(c, pick, options(), |g| g, at)
}

fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}

/// Center of the indicator of row `index` in a default vertical group (32 px rows, 2 px gaps).
fn row(r: zaxis::Response, index: usize) -> Vec2 {
    Vec2::new(
        r.rect.min.x + 12.0,
        r.rect.min.y + index as f32 * 34.0 + 16.0,
    )
}

fn press(c: &mut Context, key: KeyCode) {
    c.key(key, ElementState::Pressed, false);
    c.key(key, ElementState::Released, false);
}

fn tab_into_group(c: &mut Context, pick: &mut Pick) {
    draw(c, pick, 0);
    for _ in 0..2 {
        press(c, KeyCode::Tab);
    }
}

fn settled(c: &Context) -> bool {
    !c.needs_repaint_at(c.frame_time() + Duration::from_secs(10))
}

#[test]
fn mouse_selects_once_on_release_and_repeat_click_changes_nothing() {
    let mut c = setup();
    let mut pick = Pick::A;
    let r = draw(&mut c, &mut pick, 0);
    click(&mut c, row(r, 1));
    assert!(draw(&mut c, &mut pick, 1).changed());
    assert_eq!(pick, Pick::B);
    assert!(!draw(&mut c, &mut pick, 2).changed());
    click(&mut c, row(r, 1));
    assert!(!draw(&mut c, &mut pick, 3).changed());
    c.move_pointer(row(r, 2));
    c.primary_button(ElementState::Pressed);
    c.move_pointer(Vec2::new(700.0, 500.0));
    c.primary_button(ElementState::Released);
    assert!(!draw(&mut c, &mut pick, 4).changed());
    assert_eq!(pick, Pick::B, "release away from the pressed row cancels");
}

#[test]
fn whole_row_is_the_hit_area_and_neighbours_stay_apart() {
    let mut c = setup();
    let mut pick = Pick::A;
    let r = draw(&mut c, &mut pick, 0);
    let label = Vec2::new(r.rect.min.x + 60.0, r.rect.min.y + 34.0 + 16.0);
    click(&mut c, label);
    draw(&mut c, &mut pick, 1);
    assert_eq!(pick, Pick::B, "the label selects its row");
    click(&mut c, Vec2::new(r.rect.min.x + 12.0, r.rect.min.y + 33.0));
    draw(&mut c, &mut pick, 2);
    assert_eq!(pick, Pick::B, "the gap between rows hits nothing");
}

#[test]
fn group_is_one_tab_stop_and_arrows_wrap_and_select_once() {
    let mut c = setup();
    let mut pick = Pick::B;
    tab_into_group(&mut c, &mut pick);
    let r = draw(&mut c, &mut pick, 1);
    assert!(r.has_focus && r.focus_visible, "Tab lands on the selection");
    press(&mut c, KeyCode::ArrowDown);
    assert!(draw(&mut c, &mut pick, 2).changed());
    assert_eq!(pick, Pick::C);
    assert!(!draw(&mut c, &mut pick, 3).changed());
    press(&mut c, KeyCode::ArrowDown);
    draw(&mut c, &mut pick, 4);
    assert_eq!(pick, Pick::A, "wraps past the end");
    press(&mut c, KeyCode::ArrowUp);
    draw(&mut c, &mut pick, 5);
    assert_eq!(pick, Pick::C, "wraps before the start");
    press(&mut c, KeyCode::Home);
    draw(&mut c, &mut pick, 6);
    assert_eq!(pick, Pick::A);
    press(&mut c, KeyCode::End);
    draw(&mut c, &mut pick, 7);
    assert_eq!(pick, Pick::C);
    press(&mut c, KeyCode::Tab);
    assert!(!draw(&mut c, &mut pick, 8).has_focus, "one Tab leaves");
}

#[test]
fn tab_lands_on_first_enabled_when_nothing_is_selected() {
    let mut c = setup();
    let mut pick = Pick::Other;
    let opts = || {
        vec![
            RadioOption::new(Pick::A, "Alpha").enabled(false),
            RadioOption::new(Pick::B, "Beta"),
        ]
    };
    draw_with(&mut c, &mut pick, opts(), |g| g, 0);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    draw_with(&mut c, &mut pick, opts(), |g| g, 1);
    press(&mut c, KeyCode::Space);
    draw_with(&mut c, &mut pick, opts(), |g| g, 2);
    assert_eq!(pick, Pick::B, "focus was on the first enabled option");
}

#[test]
fn keyboard_skips_disabled_and_selected_disabled_stays_selected() {
    let mut c = setup();
    let mut pick = Pick::A;
    let opts = || {
        vec![
            RadioOption::new(Pick::A, "Alpha"),
            RadioOption::new(Pick::B, "Beta").enabled(false),
            RadioOption::new(Pick::C, "Gamma"),
        ]
    };
    draw_with(&mut c, &mut pick, opts(), |g| g, 0);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    draw_with(&mut c, &mut pick, opts(), |g| g, 1);
    press(&mut c, KeyCode::ArrowDown);
    draw_with(&mut c, &mut pick, opts(), |g| g, 2);
    assert_eq!(pick, Pick::C);
    let r = draw_with(&mut c, &mut pick, opts(), |g| g, 3);
    click(&mut c, row(r, 1));
    assert!(!draw_with(&mut c, &mut pick, opts(), |g| g, 4).changed());
    assert_eq!(pick, Pick::C, "pointer ignores the disabled row");
    let mut locked = Pick::B;
    let r = draw_with(&mut c, &mut locked, opts(), |g| g, 5);
    click(&mut c, row(r, 0));
    draw_with(&mut c, &mut locked, opts(), |g| g, 6);
    assert_eq!(locked, Pick::A, "the selected disabled option can be left");
}

#[test]
fn focus_only_mode_selects_with_space_and_enter() {
    let mut c = setup();
    let mut pick = Pick::A;
    let tweak: Tweak = |g| g.focus_follows_selection(false);
    draw_with(&mut c, &mut pick, options(), tweak, 0);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    draw_with(&mut c, &mut pick, options(), tweak, 1);
    press(&mut c, KeyCode::ArrowDown);
    assert!(!draw_with(&mut c, &mut pick, options(), tweak, 2).changed());
    assert_eq!(pick, Pick::A, "arrows only move the focus");
    press(&mut c, KeyCode::Space);
    assert!(draw_with(&mut c, &mut pick, options(), tweak, 3).changed());
    assert_eq!(pick, Pick::B);
    press(&mut c, KeyCode::ArrowDown);
    draw_with(&mut c, &mut pick, options(), tweak, 4);
    press(&mut c, KeyCode::Enter);
    draw_with(&mut c, &mut pick, options(), tweak, 5);
    assert_eq!(pick, Pick::C);
}

#[test]
fn horizontal_uses_left_right_and_grid_uses_all_arrows() {
    let mut c = setup();
    let mut pick = Pick::A;
    let h: Tweak = |g| g.horizontal();
    let four = || {
        [Pick::A, Pick::B, Pick::C, Pick::D]
            .into_iter()
            .map(|p| RadioOption::new(p, format!("{p:?}")))
            .collect::<Vec<_>>()
    };
    draw_with(&mut c, &mut pick, four(), h, 0);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    draw_with(&mut c, &mut pick, four(), h, 1);
    press(&mut c, KeyCode::ArrowDown);
    draw_with(&mut c, &mut pick, four(), h, 2);
    assert_eq!(pick, Pick::A, "Down does nothing in a row");
    press(&mut c, KeyCode::ArrowRight);
    draw_with(&mut c, &mut pick, four(), h, 3);
    assert_eq!(pick, Pick::B);

    let mut c = setup();
    let mut pick = Pick::A;
    let g: Tweak = |g| g.columns(2);
    draw_with(&mut c, &mut pick, four(), g, 0);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    draw_with(&mut c, &mut pick, four(), g, 1);
    press(&mut c, KeyCode::ArrowDown);
    draw_with(&mut c, &mut pick, four(), g, 2);
    assert_eq!(pick, Pick::C, "Down stays in the column");
    press(&mut c, KeyCode::ArrowDown);
    draw_with(&mut c, &mut pick, four(), g, 3);
    assert_eq!(pick, Pick::A, "and wraps inside it");
    press(&mut c, KeyCode::ArrowRight);
    draw_with(&mut c, &mut pick, four(), g, 4);
    assert_eq!(pick, Pick::B);
}

#[test]
fn disabled_group_ignores_input_and_is_not_a_tab_stop() {
    let mut c = setup();
    let mut pick = Pick::A;
    let off: Tweak = |g| g.enabled(false);
    let r = draw_with(&mut c, &mut pick, options(), off, 0);
    click(&mut c, row(r, 2));
    draw_with(&mut c, &mut pick, options(), off, 1);
    assert_eq!(pick, Pick::A);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    assert!(!draw_with(&mut c, &mut pick, options(), off, 2).has_focus);
}

#[test]
fn external_assignment_and_unknown_value_never_report_changed() {
    let mut c = setup();
    let mut pick = Pick::A;
    draw(&mut c, &mut pick, 0);
    pick = Pick::C;
    assert!(!draw(&mut c, &mut pick, 1).changed());
    pick = Pick::Other;
    let r = draw(&mut c, &mut pick, 2);
    assert!(!r.changed());
    assert_eq!(pick, Pick::Other, "an unmatched value is kept");
    click(&mut c, row(r, 1));
    assert!(draw(&mut c, &mut pick, 3).changed());
    assert_eq!(pick, Pick::B);
}

#[test]
fn option_values_work_for_option_t_and_radio_value() {
    let mut c = setup();
    let mut choice: Option<Pick> = None;
    let mut single = Pick::A;
    let frame = |c: &mut Context, choice: &mut Option<Pick>, single: &mut Pick, at| {
        let start = c.frame_time();
        c.run_at(start + Duration::from_millis(at), |c| {
            Window::new("Test").show(c, |ui| {
                ui.radio_group(choice, [(Some(Pick::A), "A"), (Some(Pick::B), "B")]);
                ui.radio_value(single, Pick::C, "C");
            });
        });
    };
    frame(&mut c, &mut choice, &mut single, 0);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    frame(&mut c, &mut choice, &mut single, 1);
    press(&mut c, KeyCode::Space);
    frame(&mut c, &mut choice, &mut single, 2);
    assert_eq!(single, Pick::C);
    assert_eq!(choice, None, "an empty Option selects nothing");
}

#[test]
fn rapid_switching_ends_on_the_last_value_and_settles() {
    let mut c = setup();
    let mut pick = Pick::A;
    let r = draw(&mut c, &mut pick, 0);
    for (step, index) in [1usize, 2, 0, 2, 1].into_iter().enumerate() {
        click(&mut c, row(r, index));
        draw(&mut c, &mut pick, 10 + step as u64 * 20);
    }
    assert_eq!(pick, Pick::B);
    assert!(!settled(&c), "the dot is still moving");
    draw(&mut c, &mut pick, 3000);
    draw(&mut c, &mut pick, 3001);
    assert!(settled(&c), "a settled group requests no redraw");
}

#[test]
fn first_frame_does_not_animate_and_reduced_motion_is_instant() {
    let mut c = setup();
    let mut pick = Pick::B;
    draw(&mut c, &mut pick, 0);
    assert!(settled(&c), "an already selected dot does not grow in");
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    let r = draw(&mut c, &mut pick, 1);
    click(&mut c, row(r, 2));
    draw(&mut c, &mut pick, 2);
    draw(&mut c, &mut pick, 3);
    assert_eq!(pick, Pick::C);
    assert!(settled(&c));
}

#[test]
fn reordering_and_removing_options_keeps_the_focused_value() {
    let mut c = setup();
    let mut pick = Pick::B;
    tab_into_group(&mut c, &mut pick);
    draw(&mut c, &mut pick, 1);
    let reversed = || options().into_iter().rev().collect::<Vec<_>>();
    let r = draw_with(&mut c, &mut pick, reversed(), |g| g, 2);
    assert!(r.has_focus, "focus follows the value, not the index");
    press(&mut c, KeyCode::ArrowDown);
    draw_with(&mut c, &mut pick, reversed(), |g| g, 3);
    assert_eq!(pick, Pick::A, "next after Beta in the reversed order");
    let fewer = vec![RadioOption::new(Pick::B, "Beta")];
    draw_with(&mut c, &mut pick, fewer, |g| g, 4);
    draw_with(&mut c, &mut pick, options(), |g| g, 5);
}

#[test]
fn invalid_input_is_reported_without_panicking() {
    let mut c = setup();
    let mut pick = Pick::A;
    draw_with(&mut c, &mut pick, Vec::new(), |g| g, 0);
    draw_with(&mut c, &mut pick, Vec::new(), |g| g, 1);
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidValue));
    let dup = vec![
        RadioOption::new(Pick::A, "One"),
        RadioOption::new(Pick::A, "Two"),
    ];
    draw_with(&mut c, &mut pick, dup, |g| g, 2);
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidValue));
    let nan: Tweak = |g| {
        g.row_gap(f32::NAN)
            .column_gap(f32::INFINITY)
            .label_gap(-1.0)
            .min_row_height(f32::NAN)
            .columns(0)
    };
    draw_with(&mut c, &mut pick, options(), nan, 3);
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidValue));
}

#[test]
fn long_labels_wrap_inside_the_container_at_every_scale() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let mut c = Context::new();
        c.set_viewport(
            PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
            scale,
        );
        let mut pick = Pick::A;
        let long = || {
            vec![
                RadioOption::new(Pick::A, "Short"),
                RadioOption::new(
                    Pick::B,
                    "A very long label that has to wrap onto several lines to fit a narrow window",
                )
                .description("Second line of muted text that wraps as well when it is long"),
            ]
        };
        let mut rect = None;
        let start = c.frame_time();
        c.run_at(start, |c| {
            Window::new("Test").show(c, |ui| {
                ui.with_max_width(200.0, |ui| {
                    rect = Some(ui.radio_group(&mut pick, long()).rect);
                });
            });
        });
        let rect = rect.unwrap();
        assert!(rect.size().x <= 200.5, "{scale}: {} wide", rect.size().x);
        assert!(
            rect.size().y > 2.0 * 32.0,
            "{scale}: wrapped rows are taller"
        );
        let px = rect.size().y * scale as f32;
        assert!(
            (px - px.round()).abs() < 0.01,
            "{scale}: {px} px is fractional"
        );
    }
}

#[test]
fn arrows_belong_to_a_focused_text_edit() {
    let mut c = setup();
    let mut pick = Pick::B;
    let mut text = String::from("abc");
    let mut frame = |c: &mut Context, pick: &mut Pick, at: u64| {
        let start = c.frame_time();
        c.run_at(start + Duration::from_millis(at), |c| {
            Window::new("Test").show(c, |ui| {
                ui.add(zaxis::TextEdit::new(&mut text));
                ui.radio_group(pick, [(Pick::A, "Alpha"), (Pick::B, "Beta")]);
            });
        });
    };
    frame(&mut c, &mut pick, 0);
    press(&mut c, KeyCode::Tab);
    frame(&mut c, &mut pick, 1);
    press(&mut c, KeyCode::ArrowUp);
    frame(&mut c, &mut pick, 2);
    assert_eq!(pick, Pick::B);
    let _ = RadioLayout::Vertical;
}
