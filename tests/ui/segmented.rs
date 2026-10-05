use crate::prelude::*;
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};
use zaxis::{
    DiagnosticKind, SegmentOption, SegmentWidth, SegmentedControl, SegmentedOrientation, Window,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Mode {
    A,
    B,
    C,
    Other,
}

type Tweak = fn(SegmentedControl<'_, Mode>) -> SegmentedControl<'_, Mode>;

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}

fn options() -> Vec<SegmentOption<Mode>> {
    vec![
        SegmentOption::new(Mode::A, "Alpha"),
        SegmentOption::new(Mode::B, "Beta"),
        SegmentOption::new(Mode::C, "Gamma"),
    ]
}

fn draw_with(
    c: &mut Context,
    mode: &mut Mode,
    options: Vec<SegmentOption<Mode>>,
    tweak: Tweak,
    at: u64,
) -> zaxis::Response {
    let start = c.frame_time();
    let mut response = None;
    c.run_at(start + Duration::from_millis(at), |c| {
        Window::new("Test").show(c, |ui| {
            ui.button("Before");
            response = Some(ui.add(tweak(SegmentedControl::new(mode, options))));
            ui.button("After");
        });
    });
    response.unwrap()
}

fn draw(c: &mut Context, mode: &mut Mode, at: u64) -> zaxis::Response {
    draw_with(c, mode, options(), |s| s, at)
}

fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}

fn segment_center(r: zaxis::Response, index: usize, count: usize) -> Vec2 {
    Vec2::new(
        r.rect.min.x + r.rect.size().x * (index as f32 + 0.5) / count as f32,
        r.rect.center().y,
    )
}

fn press(c: &mut Context, key: KeyCode) {
    c.key(key, ElementState::Pressed, false);
    c.key(key, ElementState::Released, false);
}

fn tab_into_group(c: &mut Context, mode: &mut Mode) {
    draw(c, mode, 0);
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
    let mut mode = Mode::A;
    let r = draw(&mut c, &mut mode, 0);
    click(&mut c, segment_center(r, 1, 3));
    assert!(draw(&mut c, &mut mode, 1).changed());
    assert_eq!(mode, Mode::B);
    assert!(!draw(&mut c, &mut mode, 2).changed());
    click(&mut c, segment_center(r, 1, 3));
    assert!(!draw(&mut c, &mut mode, 3).changed());
    c.move_pointer(segment_center(r, 2, 3));
    c.primary_button(ElementState::Pressed);
    c.move_pointer(Vec2::new(700.0, 500.0));
    c.primary_button(ElementState::Released);
    assert!(!draw(&mut c, &mut mode, 4).changed());
    assert_eq!(
        mode,
        Mode::B,
        "release away from the pressed segment cancels"
    );
}

#[test]
fn group_is_one_tab_stop_and_arrows_move_focus_and_selection() {
    let mut c = setup();
    let mut mode = Mode::B;
    tab_into_group(&mut c, &mut mode);
    let r = draw(&mut c, &mut mode, 1);
    assert!(r.has_focus && r.focus_visible, "Tab lands on the group");
    press(&mut c, KeyCode::ArrowRight);
    assert!(draw(&mut c, &mut mode, 2).changed());
    assert_eq!(mode, Mode::C);
    assert!(!draw(&mut c, &mut mode, 3).changed());
    press(&mut c, KeyCode::ArrowRight);
    assert!(
        !draw(&mut c, &mut mode, 4).changed(),
        "no wrap past the end"
    );
    press(&mut c, KeyCode::Home);
    assert!(draw(&mut c, &mut mode, 5).changed());
    assert_eq!(mode, Mode::A);
    press(&mut c, KeyCode::End);
    draw(&mut c, &mut mode, 6);
    assert_eq!(mode, Mode::C);
    press(&mut c, KeyCode::Tab);
    assert!(
        !draw(&mut c, &mut mode, 7).has_focus,
        "one Tab leaves the group"
    );
}

#[test]
fn keyboard_skips_disabled_and_ignores_pointer_on_them() {
    let mut c = setup();
    let mut mode = Mode::A;
    let with_disabled = || {
        vec![
            SegmentOption::new(Mode::A, "Alpha"),
            SegmentOption::new(Mode::B, "Beta").enabled(false),
            SegmentOption::new(Mode::C, "Gamma"),
        ]
    };
    draw_with(&mut c, &mut mode, with_disabled(), |s| s, 0);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    draw_with(&mut c, &mut mode, with_disabled(), |s| s, 1);
    press(&mut c, KeyCode::ArrowRight);
    draw_with(&mut c, &mut mode, with_disabled(), |s| s, 2);
    assert_eq!(mode, Mode::C);
    let r = draw_with(&mut c, &mut mode, with_disabled(), |s| s, 3);
    click(&mut c, segment_center(r, 1, 3));
    assert!(!draw_with(&mut c, &mut mode, with_disabled(), |s| s, 4).changed());
    assert_eq!(mode, Mode::C);
}

#[test]
fn focus_only_mode_selects_with_space_and_enter() {
    let mut c = setup();
    let mut mode = Mode::A;
    let tweak: Tweak = |s| s.focus_follows_selection(false);
    draw_with(&mut c, &mut mode, options(), tweak, 0);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    draw_with(&mut c, &mut mode, options(), tweak, 1);
    press(&mut c, KeyCode::ArrowRight);
    assert!(!draw_with(&mut c, &mut mode, options(), tweak, 2).changed());
    assert_eq!(mode, Mode::A, "arrows only move the focus");
    press(&mut c, KeyCode::Space);
    assert!(draw_with(&mut c, &mut mode, options(), tweak, 3).changed());
    assert_eq!(mode, Mode::B);
    press(&mut c, KeyCode::ArrowRight);
    draw_with(&mut c, &mut mode, options(), tweak, 4);
    press(&mut c, KeyCode::Enter);
    draw_with(&mut c, &mut mode, options(), tweak, 5);
    assert_eq!(mode, Mode::C);
}

#[test]
fn vertical_uses_up_down_arrows() {
    let mut c = setup();
    let mut mode = Mode::A;
    let tweak: Tweak = |s| s.orientation(SegmentedOrientation::Vertical);
    draw_with(&mut c, &mut mode, options(), tweak, 0);
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    draw_with(&mut c, &mut mode, options(), tweak, 1);
    press(&mut c, KeyCode::ArrowRight);
    draw_with(&mut c, &mut mode, options(), tweak, 2);
    assert_eq!(mode, Mode::A);
    press(&mut c, KeyCode::ArrowDown);
    draw_with(&mut c, &mut mode, options(), tweak, 3);
    assert_eq!(mode, Mode::B);
}

#[test]
fn disabled_control_ignores_input_and_selected_disabled_option_stays() {
    let mut c = setup();
    let mut mode = Mode::B;
    let off: Tweak = |s| s.enabled(false);
    let r = draw_with(&mut c, &mut mode, options(), off, 0);
    click(&mut c, segment_center(r, 2, 3));
    assert!(!draw_with(&mut c, &mut mode, options(), off, 1).changed());
    assert_eq!(mode, Mode::B);
    let opts = || {
        vec![
            SegmentOption::new(Mode::B, "Beta").enabled(false),
            SegmentOption::new(Mode::C, "Gamma"),
        ]
    };
    draw_with(&mut c, &mut mode, opts(), |s| s, 2);
    assert_eq!(mode, Mode::B, "a disabled selected option stays selected");
}

#[test]
fn external_assignment_and_unknown_value_never_report_changed() {
    let mut c = setup();
    let mut mode = Mode::A;
    draw(&mut c, &mut mode, 0);
    mode = Mode::C;
    assert!(!draw(&mut c, &mut mode, 1).changed());
    mode = Mode::Other;
    let r = draw(&mut c, &mut mode, 2);
    assert!(!r.changed());
    assert_eq!(mode, Mode::Other, "an unmatched value is not replaced");
    press(&mut c, KeyCode::Tab);
    press(&mut c, KeyCode::Tab);
    draw(&mut c, &mut mode, 3);
    assert_eq!(mode, Mode::Other);
}

#[test]
fn rapid_switching_ends_on_the_last_value_and_settles() {
    let mut c = setup();
    let mut mode = Mode::A;
    let r = draw(&mut c, &mut mode, 0);
    for (step, index) in [1usize, 2, 0, 2, 1].into_iter().enumerate() {
        click(&mut c, segment_center(r, index, 3));
        draw(&mut c, &mut mode, 10 + step as u64 * 20);
    }
    assert_eq!(mode, Mode::B);
    assert!(!settled(&c), "the thumb is still sliding");
    draw(&mut c, &mut mode, 3000);
    draw(&mut c, &mut mode, 3001);
    assert!(settled(&c), "a settled control requests no redraw");
}

#[test]
fn first_frame_and_option_set_change_place_the_thumb_without_animation() {
    let mut c = setup();
    let mut mode = Mode::B;
    draw(&mut c, &mut mode, 0);
    assert!(settled(&c), "first appearance does not fly in");
    draw_with(
        &mut c,
        &mut mode,
        vec![
            SegmentOption::new(Mode::B, "Beta"),
            SegmentOption::new(Mode::C, "Gamma"),
        ],
        |s| s,
        1,
    );
    draw_with(
        &mut c,
        &mut mode,
        vec![
            SegmentOption::new(Mode::B, "Beta"),
            SegmentOption::new(Mode::Other, "Other"),
        ],
        |s| s,
        2,
    );
    assert!(settled(&c), "a new option set snaps");
}

#[test]
fn reduced_motion_is_instant_even_when_the_thumb_changes_width() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    let mut mode;
    let uneven = || {
        vec![
            SegmentOption::new(Mode::A, "A"),
            SegmentOption::new(Mode::B, "Medium"),
            SegmentOption::new(Mode::C, "Longer label"),
        ]
    };
    let modes: [Tweak; 3] = [
        |s| s.width(SegmentWidth::Content),
        |s| s.width(SegmentWidth::Fill),
        |s| s,
    ];
    for (round, tweak) in modes.into_iter().enumerate() {
        let at = round as u64 * 20;
        mode = Mode::A;
        draw_with(&mut c, &mut mode, uneven(), tweak, at);
        draw_with(&mut c, &mut mode, uneven(), tweak, at + 1);
        let r = draw_with(&mut c, &mut mode, uneven(), tweak, at + 2);
        click(&mut c, segment_center(r, 2, 3));
        draw_with(&mut c, &mut mode, uneven(), tweak, at + 3);
        draw_with(&mut c, &mut mode, uneven(), tweak, at + 4);
        assert_eq!(mode, Mode::C, "round {round}");
        assert!(settled(&c), "round {round}");
    }
}

#[test]
fn invalid_input_is_reported_without_panicking() {
    let mut c = setup();
    let mut mode = Mode::A;
    draw_with(&mut c, &mut mode, Vec::new(), |s| s, 0);
    draw_with(&mut c, &mut mode, Vec::new(), |s| s, 1);
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidValue));
    let dup = vec![
        SegmentOption::new(Mode::A, "One"),
        SegmentOption::new(Mode::A, "Two"),
    ];
    draw_with(&mut c, &mut mode, dup, |s| s, 2);
    draw_with(
        &mut c,
        &mut mode,
        vec![SegmentOption::new(Mode::A, "Only")],
        |s| s,
        3,
    );
    let nan: Tweak = |s| {
        s.width(SegmentWidth::Fixed(f32::NAN))
            .min_segment_width(f32::NAN)
    };
    draw_with(&mut c, &mut mode, options(), nan, 4);
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::InvalidValue));
}

#[test]
fn fill_width_is_whole_physical_pixels_at_every_scale() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let mut c = Context::new();
        c.set_viewport(
            PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
            scale,
        );
        let mut mode = Mode::A;
        let fill: Tweak = |s| s.width(SegmentWidth::Fill);
        let r = draw_with(&mut c, &mut mode, options(), fill, 0);
        let px = r.rect.size().x * scale as f32;
        assert!(
            (px - px.round()).abs() < 1e-3,
            "{scale}: {px} px is fractional"
        );
    }
}

#[test]
fn arrows_belong_to_a_focused_text_edit() {
    let mut c = setup();
    let mut mode = Mode::B;
    let mut text = String::from("abc");
    let mut frame = |c: &mut Context, mode: &mut Mode, at: u64| {
        let start = c.frame_time();
        c.run_at(start + Duration::from_millis(at), |c| {
            Window::new("Test").show(c, |ui| {
                ui.add(zaxis::TextEdit::new(&mut text));
                ui.segmented(mode, [(Mode::A, "Alpha"), (Mode::B, "Beta")]);
            });
        });
    };
    frame(&mut c, &mut mode, 0);
    press(&mut c, KeyCode::Tab);
    frame(&mut c, &mut mode, 1);
    press(&mut c, KeyCode::ArrowLeft);
    frame(&mut c, &mut mode, 2);
    assert_eq!(mode, Mode::B);
}

#[test]
fn outlined_variant_selects_by_mouse_and_keyboard_without_a_thumb() {
    let mut c = setup();
    let mut mode = Mode::A;
    let outlined: Tweak = |s| s.variant(zaxis::SegmentedVariant::Outlined);
    draw_with(&mut c, &mut mode, options(), outlined, 0);
    let r = draw_with(&mut c, &mut mode, options(), outlined, 1);
    click(&mut c, segment_center(r, 2, 3));
    assert!(draw_with(&mut c, &mut mode, options(), outlined, 2).changed());
    assert_eq!(mode, Mode::C);
    c.move_pointer(Vec2::new(790.0, 590.0));
    for at in 3..40 {
        draw_with(&mut c, &mut mode, options(), outlined, at * 100);
    }
    assert!(settled(&c), "fills fade out and stop requesting redraws");
    for _ in 0..2 {
        press(&mut c, KeyCode::Tab);
    }
    draw_with(&mut c, &mut mode, options(), outlined, 5000);
    press(&mut c, KeyCode::Home);
    draw_with(&mut c, &mut mode, options(), outlined, 5001);
    assert_eq!(mode, Mode::A);
}
