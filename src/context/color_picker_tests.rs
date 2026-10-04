use super::*;
use crate::{Color, ColorPicker, ColorPickerType, Rect, Response, Window};
use winit::{dpi::PhysicalSize, event::ElementState};

fn draw(
    context: &mut Context,
    color: &mut Color,
    kind: ColorPickerType,
    enabled: bool,
) -> Response {
    let mut response = None;
    context.run(|context| {
        Window::new("Test")
            .default_size(crate::vec2(520.0, 520.0))
            .show(context, |ui| {
                response = Some(
                    ui.add(
                        ColorPicker::new(color, "Color")
                            .id_source("picker")
                            .picker_type(kind)
                            .enabled(enabled)
                            .default_open(true),
                    ),
                );
                ui.button("After");
            });
    });
    response.unwrap()
}

fn context() -> Context {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(900, 800), 1.0);
    context
}

fn click(context: &mut Context, point: Vec2) {
    context.move_pointer(point);
    context.primary_button(ElementState::Pressed);
    context.primary_button(ElementState::Released);
}

fn region(context: &Context, id: Id) -> Rect {
    context
        .previous_hits
        .iter()
        .find(|hit| hit.id == id)
        .unwrap()
        .rect
}

#[test]
fn palette_capture_clamps_outside_and_preserves_alpha_and_selected_hue() {
    let mut context = context();
    let mut color = Color::rgba(255, 0, 0, 73);
    let response = draw(&mut context, &mut color, ColorPickerType::Internal, true);
    let palette = region(&context, response.id.with("palette"));
    context.move_pointer(palette.center());
    context.primary_button(ElementState::Pressed);
    context.move_pointer(palette.max + Vec2::splat(50.0));
    assert!(draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    assert_eq!(color, Color::rgba(0, 0, 0, 73));
    assert!(context.capture.is_some());
    context.primary_button(ElementState::Released);
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    let hue = region(&context, response.id.with("hue"));
    click(
        &mut context,
        hue.min + hue.size() * crate::vec2(1.0 / 3.0, 0.5),
    );
    assert!(!draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    click(
        &mut context,
        palette.min + crate::vec2(palette.size().x - 0.01, 0.01),
    );
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert_eq!(color, Color::rgba(0, 255, 0, 73));
}

#[test]
fn rgb_hex_validation_commit_cancel_and_cursor_edits_use_real_input() {
    let mut context = context();
    let mut color = Color::rgba(78, 133, 190, 99);
    let id = draw(&mut context, &mut color, ColorPickerType::Internal, true).id;
    let point = region(&context, id.with(("field", 3usize))).center();
    click(&mut context, point);
    assert!(context.on_text_event("#12aB34").consumed);
    context.key(KeyCode::Enter, ElementState::Pressed, false);
    assert!(draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    assert_eq!(color, Color::rgba(0x12, 0xab, 0x34, 99));
    context.on_text_event("#invalid");
    context.key(KeyCode::Enter, ElementState::Pressed, false);
    assert!(!draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    context.on_text_event("#FFFFFF");
    context.key(KeyCode::Escape, ElementState::Pressed, false);
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert_eq!(color, Color::rgba(0x12, 0xab, 0x34, 99));
    assert!(context
        .previous_hits
        .iter()
        .any(|hit| hit.id == id.with("palette")));
    let point = region(&context, id.with(("field", 0usize))).center();
    click(&mut context, point);
    context.on_text_event("999");
    // Tab changes focus before the first UI pass after clicking and typing.
    context.key(KeyCode::Tab, ElementState::Pressed, false);
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert_eq!(color.0[0], 255);
    context.on_text_event("123");
    context.key(KeyCode::ArrowLeft, ElementState::Pressed, false);
    context.key(KeyCode::Backspace, ElementState::Pressed, false);
    context.on_text_event("4");
    context.key(KeyCode::Enter, ElementState::Pressed, false);
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert_eq!(color.0[1], 143);
}

#[test]
fn floating_window_keeps_parent_layout_and_reopens_above_parent() {
    let mut context = context();
    let mut color = Color::rgb(78, 133, 190);
    let response = draw(&mut context, &mut color, ColorPickerType::Floating, true);
    assert_eq!(response.rect.size().y, context.style().control_height);
    let window_id = response.id.with("floating");
    assert_eq!(context.front_window(), Some(window_id));
    let initial = context.windows[&window_id].rect;
    context.move_pointer(initial.min + crate::vec2(100.0, 15.0));
    context.primary_button(ElementState::Pressed);
    context.move_pointer(initial.min + crate::vec2(180.0, 55.0));
    context.primary_button(ElementState::Released);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    assert_eq!(
        context.windows[&window_id].rect.min,
        initial.min + crate::vec2(80.0, 40.0)
    );
    let point = region(&context, response.id.with("close")).center();
    click(&mut context, point);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    assert!(!context.visible_windows.contains(&window_id));
    click(&mut context, response.rect.center());
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    assert_eq!(context.front_window(), Some(window_id));
    assert_eq!(
        context.windows[&window_id].rect.min,
        initial.min + crate::vec2(80.0, 40.0)
    );
    context.key(KeyCode::Escape, ElementState::Pressed, false);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    assert!(!context.visible_windows.contains(&window_id));
}

#[test]
fn keyboard_palette_disabled_capture_and_external_colors_stay_in_sync() {
    let mut context = context();
    let mut color = Color::rgb(255, 0, 0);
    let response = draw(&mut context, &mut color, ColorPickerType::Internal, true);
    context.key(KeyCode::Tab, ElementState::Pressed, false); // row
    context.key(KeyCode::Tab, ElementState::Pressed, false); // palette
    context.key(KeyCode::ArrowLeft, ElementState::Pressed, false);
    assert!(draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    assert_eq!(color, Color::rgb(255, 3, 3));
    let palette = region(&context, response.id.with("palette"));
    context.move_pointer(palette.center());
    context.primary_button(ElementState::Pressed);
    let original = color;
    assert!(!draw(&mut context, &mut color, ColorPickerType::Internal, false).changed());
    assert_eq!(color, original);
    assert!(context.capture.is_none());
    assert!(context.focused_widget.is_none());
    context.primary_button(ElementState::Released);
    color = Color::rgb(0, 0, 255);
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    click(
        &mut context,
        palette.min + crate::vec2(palette.size().x - 0.01, 0.01),
    );
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert_eq!(color, Color::rgb(0, 0, 255));
}

#[test]
fn stationary_editor_reuses_geometry_at_multiple_scales_and_cleans_up_state() {
    for scale in [1.0, 2.0] {
        let mut context = context();
        context.set_viewport(
            PhysicalSize::new((900.0 * scale) as u32, (800.0 * scale) as u32),
            scale,
        );
        let mut color = Color::rgb(78, 133, 190);
        draw(&mut context, &mut color, ColorPickerType::Internal, true);
        let revision = context.draw_data().revision;
        let stats = context.cache_stats();
        draw(&mut context, &mut color, ColorPickerType::Internal, true);
        assert_eq!(context.draw_data().revision, revision);
        assert_eq!(
            context.cache_stats().tessellated_elements,
            stats.tessellated_elements
        );
        assert!(!context.needs_repaint());
        assert!(context.draw_data().vertices.iter().all(|vertex| vertex
            .position
            .iter()
            .chain(&vertex.color)
            .all(|v| v.is_finite())));
        context.run(|_| {});
        assert!(context.color_pickers.is_empty());
    }
}

#[test]
fn themed_picker_in_demo_columns_routes_palette_hue_and_fields() {
    for theme in [
        crate::Theme::dark(),
        crate::Theme::light(),
        crate::Theme::high_contrast(),
    ] {
        for density in [crate::Density::Compact, crate::Density::Comfortable] {
            let mut context = context();
            let mut theme = theme.clone().density(density);
            theme.overrides.motion = Some(crate::MotionStyle {
                reduced_motion: true,
                ..Default::default()
            });
            context.set_theme(theme);
            let mut color = Color::rgba(255, 0, 0, 73);
            let draw = |context: &mut Context, color: &mut Color| {
                let mut id = Id::new(0);
                context.run(|context| {
                    crate::Root::new().show(context, |ui| {
                        ui.horizontal(|ui| {
                            ui.button("Dark");
                            ui.button("Light");
                        });
                        ui.horizontal(|ui| {
                            ui.with_width(280.0, |ui| {
                                ui.label("Controls");
                                ui.button("Button");
                            });
                            ui.vertical(|ui| {
                                ui.with_width(330.0, |ui| {
                                    ui.label("Colors");
                                    id = ui
                                        .add(
                                            ColorPicker::new(color, "Color")
                                                .id_source("picker")
                                                .width(310.0)
                                                .default_open(true),
                                        )
                                        .id;
                                })
                            });
                        });
                    })
                });
                id
            };
            let id = draw(&mut context, &mut color);
            draw(&mut context, &mut color);
            let palette = region(&context, id.with("palette"));
            click(&mut context, palette.center());
            assert_eq!(context.focused_widget, Some(id.with("palette")));
            draw(&mut context, &mut color);
            assert!(color.0[..3]
                .iter()
                .zip([128i16, 64, 64])
                .all(|(&actual, expected)| (i16::from(actual) - expected).abs() <= 1));
            assert_eq!(color.0[3], 73);
            let hue = region(&context, id.with("hue"));
            click(
                &mut context,
                hue.min + hue.size() * crate::vec2(1.0 / 3.0, 0.5),
            );
            draw(&mut context, &mut color);
            assert!(color.0[..3]
                .iter()
                .zip([64i16, 128, 64])
                .all(|(&actual, expected)| (i16::from(actual) - expected).abs() <= 1));
            assert_eq!(color.0[3], 73);
            let field = region(&context, id.with(("field", 3usize))).center();
            click(&mut context, field);
            context.on_text_event("#123456");
            context.key(KeyCode::Enter, ElementState::Pressed, false);
            draw(&mut context, &mut color);
            assert_eq!(color, Color::rgba(0x12, 0x34, 0x56, 73));
        }
    }
}
