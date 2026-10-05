use crate::prelude::*;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Ime, WindowEvent},
};
use zaxis::{Color, ColorPicker, ColorPickerType, Rect, Response, Window};

fn draw(
    context: &mut Context,
    color: &mut Color,
    kind: ColorPickerType,
    enabled: bool,
) -> Response {
    let mut response = None;
    context.run(|context| {
        Window::new("Test")
            .default_size(zaxis::vec2(520.0, 520.0))
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
        .probe()
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
    assert!(context.probe().capture.is_some());
    context.primary_button(ElementState::Released);
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    let hue = region(&context, response.id.with("hue"));
    click(
        &mut context,
        hue.min + hue.size() * zaxis::vec2(1.0 / 3.0, 0.5),
    );
    assert!(!draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    click(
        &mut context,
        palette.min + zaxis::vec2(palette.size().x - 0.01, 0.01),
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
        .probe()
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
    let initial = context.probe().windows[&window_id].rect;
    context.move_pointer(initial.min + zaxis::vec2(100.0, 15.0));
    context.primary_button(ElementState::Pressed);
    context.move_pointer(initial.min + zaxis::vec2(180.0, 55.0));
    context.primary_button(ElementState::Released);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    assert_eq!(
        context.probe().windows[&window_id].rect.min,
        initial.min + zaxis::vec2(80.0, 40.0)
    );
    let point = region(&context, response.id.with("close")).center();
    click(&mut context, point);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    assert!(!context.probe().visible_windows.contains(&window_id));
    click(&mut context, response.rect.center());
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    assert_eq!(context.front_window(), Some(window_id));
    assert_eq!(
        context.probe().windows[&window_id].rect.min,
        initial.min + zaxis::vec2(80.0, 40.0)
    );
    context.key(KeyCode::Escape, ElementState::Pressed, false);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    draw(&mut context, &mut color, ColorPickerType::Floating, true);
    assert!(!context.probe().visible_windows.contains(&window_id));
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
    assert!(context.probe().capture.is_none());
    assert!(context.probe().focused_widget.is_none());
    context.primary_button(ElementState::Released);
    color = Color::rgb(0, 0, 255);
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    click(
        &mut context,
        palette.min + zaxis::vec2(palette.size().x - 0.01, 0.01),
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
        assert!(context.probe().color_pickers.is_empty());
    }
}

#[test]
fn themed_picker_in_demo_columns_routes_palette_hue_and_fields() {
    for theme in [
        zaxis::Theme::dark(),
        zaxis::Theme::light(),
        zaxis::Theme::high_contrast(),
    ] {
        for density in [zaxis::Density::Compact, zaxis::Density::Comfortable] {
            let mut context = context();
            let mut theme = theme.clone().density(density);
            theme.overrides.motion = Some(zaxis::MotionStyle {
                reduced_motion: true,
                ..Default::default()
            });
            context.set_theme(theme);
            let mut color = Color::rgba(255, 0, 0, 73);
            let draw = |context: &mut Context, color: &mut Color| {
                let mut id = Id::new(0);
                context.run(|context| {
                    zaxis::Root::new().show(context, |ui| {
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
            assert_eq!(context.probe().focused_widget, Some(id.with("palette")));
            draw(&mut context, &mut color);
            assert!(color.0[..3]
                .iter()
                .zip([128i16, 64, 64])
                .all(|(&actual, expected)| (i16::from(actual) - expected).abs() <= 1));
            assert_eq!(color.0[3], 73);
            let hue = region(&context, id.with("hue"));
            click(
                &mut context,
                hue.min + hue.size() * zaxis::vec2(1.0 / 3.0, 0.5),
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

/// An open editor takes input on its palette; a closed (or closing) one blocks it.
fn editor_open(context: &Context, id: Id) -> bool {
    context
        .probe()
        .previous_hits
        .iter()
        .any(|hit| hit.id == id.with("palette") && hit.action == HitAction::Slider)
}

fn press(context: &mut Context, code: KeyCode, mods: winit::keyboard::ModifiersState) {
    context.set_modifiers(mods);
    context.key(code, ElementState::Pressed, false);
    context.key(code, ElementState::Released, false);
    context.set_modifiers(winit::keyboard::ModifiersState::empty());
}

#[test]
fn field_escape_drops_the_draft_then_escape_outside_the_fields_closes_the_picker() {
    let mut context = context();
    let mut color = Color::rgba(78, 133, 190, 99);
    let id = draw(&mut context, &mut color, ColorPickerType::Internal, true).id;
    let at = region(&context, id.with(("field", 3usize))).center();
    click(&mut context, at);
    context.on_text_event("#FFFFFF");
    press(&mut context, KeyCode::Escape, Default::default());
    let r = draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert!(!r.changed());
    assert_eq!(color, Color::rgba(78, 133, 190, 99));
    assert!(editor_open(&context, id), "a field takes Escape");
    // Focus leaves the fields for the hue strip; the next Escape belongs to the picker.
    for _ in 0..4 {
        press(
            &mut context,
            KeyCode::Tab,
            winit::keyboard::ModifiersState::SHIFT,
        );
    }
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert_eq!(context.probe().focused_widget, Some(id.with("hue")));
    press(&mut context, KeyCode::Escape, Default::default());
    assert!(!draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    // The pass that closes it still built the open editor; the next one has it closed.
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert!(!editor_open(&context, id));
}

#[test]
fn fields_are_text_edits_with_clipboard_and_a_printable_ascii_filter() {
    let mut context = context();
    let clipboard = MemoryClipboard::with_text("#00ff00");
    context.set_clipboard(clipboard.clone());
    let mut color = Color::rgba(10, 20, 30, 40);
    let id = draw(&mut context, &mut color, ColorPickerType::Internal, true).id;
    let hex = id.with(("field", 3usize));
    let at = region(&context, hex).center();
    click(&mut context, at);
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert!(
        context.probe().text_edits.contains_key(&hex),
        "a real TextEdit"
    );
    // Focus selected the whole field: the paste replaces it.
    press(
        &mut context,
        KeyCode::KeyV,
        winit::keyboard::ModifiersState::CONTROL,
    );
    press(&mut context, KeyCode::Enter, Default::default());
    assert!(draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    assert_eq!(color, Color::rgba(0, 255, 0, 40));
    // Copy takes the committed text, selected whole after the commit.
    press(
        &mut context,
        KeyCode::KeyC,
        winit::keyboard::ModifiersState::CONTROL,
    );
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert_eq!(clipboard.get().as_deref(), Some("#00FF00"));
    // Composed and pasted text outside printable ASCII is dropped.
    let red = id.with(("field", 0usize));
    let at = region(&context, red).center();
    click(&mut context, at);
    context.on_window_event(&WindowEvent::Ime(Ime::Commit("１２ü7".to_owned())));
    press(&mut context, KeyCode::Enter, Default::default());
    assert!(draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    assert_eq!(color, Color::rgba(7, 255, 0, 40));
}

#[test]
fn typing_through_all_four_fields_in_one_frame_commits_each_once() {
    let mut context = context();
    let mut color = Color::rgba(1, 2, 3, 200);
    let id = draw(&mut context, &mut color, ColorPickerType::Internal, true).id;
    let at = region(&context, id.with(("field", 0usize))).center();
    click(&mut context, at);
    for text in ["10", "20", "30"] {
        context.on_text_event(text);
        press(&mut context, KeyCode::Tab, Default::default());
    }
    let r = draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert!(r.changed());
    assert_eq!(color, Color::rgba(10, 20, 30, 200));
    assert!(!draw(&mut context, &mut color, ColorPickerType::Internal, true).changed());
    // The hex field has focus now; an outside change replaces its pending draft.
    context.on_text_event("#123456");
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    color = Color::rgba(200, 100, 50, 200);
    draw(&mut context, &mut color, ColorPickerType::Internal, true);
    press(&mut context, KeyCode::Tab, Default::default());
    let r = draw(&mut context, &mut color, ColorPickerType::Internal, true);
    assert_ne!(
        context.probe().focused_widget,
        Some(id.with(("field", 3usize)))
    );
    assert!(!r.changed(), "the dropped draft is never committed");
    assert_eq!(color, Color::rgba(200, 100, 50, 200));
}
