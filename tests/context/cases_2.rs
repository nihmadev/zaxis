use super::*;

#[test]
fn irrelevant_pointer_motion_reuses_every_element_and_frame_buffers() {
    let mut context = context();
    build(&mut context, "Hello");
    let stats = context.cache_stats();
    let revision = context.draw_data().revision;
    move_to(&mut context, vec2(4.0, 4.0));
    build(&mut context, "Hello");
    assert_eq!(context.draw_data().revision, revision);
    assert_eq!(
        context.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    assert!(context.cache_stats().reused_elements > stats.reused_elements);
}

#[test]
fn a_model_update_invalidates_only_the_changed_label() {
    let mut context = context();
    build(&mut context, "First");
    let stats = context.cache_stats();
    build(&mut context, "Second");
    assert_eq!(
        context.cache_stats().tessellated_elements,
        stats.tessellated_elements + 1
    );
    assert_eq!(
        context.cache_stats().geometry_rebuilds,
        stats.geometry_rebuilds + 1
    );
}

#[test]
fn a_late_button_mutation_updates_earlier_controls_without_manual_repaint() {
    let mut context = context();
    let mut style = context.style().clone();
    style.motion.reduced_motion = true;
    context.set_style(style);
    let mut checked = false;
    let mut activations = 0;
    let draw = |context: &mut Context, checked: &mut bool, activations: &mut usize| {
        let mut button = None;
        context.run(|context| {
            Window::new("Test").show(context, |ui| {
                ui.checkbox(checked, "Earlier control");
                let response = ui.button("Change earlier control");
                if response.clicked() {
                    *checked = !*checked;
                    *activations += 1;
                }
                button = Some(response);
            });
        });
        button.unwrap()
    };
    let button = draw(&mut context, &mut checked, &mut activations);
    move_to(&mut context, button.rect.center());
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    draw(&mut context, &mut checked, &mut activations);
    assert!(checked);
    assert_eq!(activations, 1);
    assert!(context.needs_repaint());
    let revision = context.draw_data().revision;
    let stats = context.cache_stats();
    draw(&mut context, &mut checked, &mut activations);
    assert_eq!(activations, 1);
    assert_eq!(context.draw_data().revision, revision + 1);
    assert_eq!(
        context.cache_stats().tessellated_elements,
        stats.tessellated_elements + 1
    );
    assert!(!context.needs_repaint());
    draw(&mut context, &mut checked, &mut activations);
    assert_eq!(context.draw_data().revision, revision + 1);
    assert!(!context.needs_repaint());
}

#[test]
fn a_late_slider_change_updates_an_earlier_label_and_then_settles() {
    let mut context = context();
    let mut style = context.style().clone();
    style.motion.reduced_motion = true;
    context.set_style(style);
    let mut value = -10.0;
    let draw = |context: &mut Context, value: &mut f32| {
        let mut slider = None;
        context.run(|context| {
            Window::new("Test").show(context, |ui| {
                ui.label(format!("Earlier value: {value}"));
                slider = Some(ui.slider(value, -10.0..=10.0));
            });
        });
        slider.unwrap()
    };
    let slider = draw(&mut context, &mut value);
    move_to(&mut context, slider.rect.center());
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    assert!(draw(&mut context, &mut value).changed());
    assert_eq!(value, 0.0);
    assert!(context.needs_repaint());
    let revision = context.draw_data().revision;
    assert!(!draw(&mut context, &mut value).changed());
    assert_eq!(context.draw_data().revision, revision + 1);
    assert!(!context.needs_repaint());
}

#[test]
fn press_and_release_before_redraw_produces_exactly_one_click() {
    let mut context = context();
    let button = build(&mut context, "Hello");
    move_to(&mut context, button.rect.center());
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    assert!(build(&mut context, "Hello").clicked());
    assert!(!context.input().primary_pressed);
    assert!(!context.input().primary_released);
    context.request_repaint();
    assert!(!build(&mut context, "Hello").clicked());
}

#[test]
fn releasing_outside_and_focus_loss_cancel_activation() {
    let mut context = context();
    let button = build(&mut context, "Hello");
    move_to(&mut context, button.rect.center());
    mouse(&mut context, ElementState::Pressed);
    move_to(&mut context, vec2(2.0, 2.0));
    mouse(&mut context, ElementState::Released);
    assert!(!build(&mut context, "Hello").clicked());
    move_to(&mut context, button.rect.center());
    mouse(&mut context, ElementState::Pressed);
    context.on_window_event(&WindowEvent::Focused(false));
    mouse(&mut context, ElementState::Released);
    assert!(!build(&mut context, "Hello").clicked());
    assert!(!context.input().primary_down);
}

#[test]
fn dragging_and_resizing_use_retained_geometry_and_minimum_size() {
    let mut context = context();
    let button = build(&mut context, "Hello");
    move_to(&mut context, vec2(80.0, 55.0));
    mouse(&mut context, ElementState::Pressed);
    move_to(&mut context, vec2(120.0, 85.0));
    mouse(&mut context, ElementState::Released);
    let moved = build(&mut context, "Hello");
    assert_eq!(moved.rect.min - button.rect.min, vec2(40.0, 30.0));
    move_to(&mut context, vec2(456.0, 306.0));
    mouse(&mut context, ElementState::Pressed);
    move_to(&mut context, vec2(100.0, 100.0));
    mouse(&mut context, ElementState::Released);
    build(&mut context, "Hello");
    let window_clip = context.draw_data().commands[0].clip_rect;
    assert_eq!(window_clip.size(), vec2(160.0, 100.0));
}

#[test]
fn a_front_window_blocks_buttons_in_a_back_window() {
    let mut context = context();
    let mut clicked = false;
    let draw = |context: &mut Context, clicked: &mut bool| {
        context.run(|context| {
            Window::new("Back").show(context, |ui| {
                *clicked = ui.button("Covered").clicked();
            });
            Window::new("Front").show(context, |ui| {
                ui.label("Blocks input");
            });
        });
    };
    draw(&mut context, &mut clicked);
    move_to(&mut context, vec2(95.0, 115.0));
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    draw(&mut context, &mut clicked);
    assert!(!clicked);
}

#[test]
fn scoped_ids_support_repeated_captions_and_disabled_buttons() {
    let mut context = context();
    let mut responses = Vec::new();
    context.run(|context| {
        Window::new("Test").show(context, |ui| {
            for i in 0..2 {
                responses.push(ui.push_id(i, |ui| ui.button("Same")));
            }
            responses.push(ui.add(Button::new("Disabled").enabled(false)));
        });
    });
    assert_ne!(responses[0].id, responses[1].id);
    move_to(&mut context, responses[2].rect.center());
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    context.run(|context| {
        Window::new("Test").show(context, |ui| {
            for i in 0..2 {
                ui.push_id(i, |ui| ui.button("Same"));
            }
            assert!(!ui.add(Button::new("Disabled").enabled(false)).clicked());
        });
    });
    assert_eq!(Id::new("stable"), Id::new("stable"));
    assert_ne!(Id::new("stable").with(0), Id::new("stable").with(1));
}

#[test]
fn dpi_changes_rebuild_glyph_geometry_and_scale_pointer_input() {
    let mut context = context();
    build(&mut context, "Hello");
    let before = context.cache_stats();
    context.set_viewport(PhysicalSize::new(1600, 1200), 2.0);
    let button = build(&mut context, "Hello");
    assert_eq!(context.draw_data().logical_size, vec2(800.0, 600.0));
    assert!(context.cache_stats().tessellated_elements > before.tessellated_elements);
    move_to(&mut context, button.rect.center());
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    assert!(build(&mut context, "Hello").clicked());
}

#[test]
fn repaint_deadlines_choose_the_earliest_and_survive_the_ui_pass() {
    let mut context = context();
    build(&mut context, "Hello");
    context.request_repaint_after(Duration::from_secs(10));
    let first = context.next_repaint().unwrap();
    context.request_repaint_after(Duration::from_secs(20));
    assert_eq!(context.next_repaint(), Some(first));
    context.request_repaint_after(Duration::from_secs(1));
    assert!(context.next_repaint().unwrap() < first);
    context.request_repaint();
    context.run(|context| {
        context.request_repaint();
    });
    assert!(context.needs_repaint());
    assert!(context.next_repaint().is_some());
}

#[test]
fn removed_widgets_drop_their_geometry_but_window_positions_are_retained() {
    let mut context = context();
    let initial = build(&mut context, "Hello");
    move_to(&mut context, vec2(80.0, 55.0));
    mouse(&mut context, ElementState::Pressed);
    move_to(&mut context, vec2(110.0, 75.0));
    mouse(&mut context, ElementState::Released);
    let moved = build(&mut context, "Hello");
    context.request_repaint();
    context.run(|_| {});
    assert!(context.draw_data().commands.is_empty());
    assert!(context.draw_data().vertices.is_empty());
    context.request_repaint();
    let reopened = build(&mut context, "Hello");
    assert_eq!(reopened.rect, moved.rect);
    assert_ne!(reopened.rect, initial.rect);
}

#[test]
fn generated_draw_commands_reference_valid_mesh_ranges_and_clips() {
    let mut context = context();
    build(&mut context, "AV kerning\nUnicode: \u{03bb} \u{0416}");
    let data = context.draw_data();
    assert!(!data.vertices.is_empty());
    for command in &data.commands {
        assert!(command.indices.end as usize <= data.indices.len());
        assert!(command.clip_rect.min.is_finite());
        assert!(command.clip_rect.max.is_finite());
    }
    assert!(data
        .indices
        .iter()
        .all(|index| (*index as usize) < data.vertices.len()));
    assert!(Rect::from_min_size(vec2(10.0, 10.0), vec2(5.0, 5.0))
        .intersect(Rect::from_min_size(vec2(50.0, 50.0), vec2(5.0, 5.0)))
        .is_empty());
    let mut style = context.style().clone();
    style.text_color = Color::WHITE;
    context.set_style(style);
    assert!(context.needs_repaint());
}

#[test]
fn checkbox_square_and_label_clicks_toggle_once_and_report_changes() {
    let mut context = context();
    let mut checked = false;
    let response = build_checkbox(&mut context, &mut checked, true);
    assert!(!response.changed());
    let square_center = response.rect.min + vec2(10.0, 10.0);
    let label_center = response.rect.min + vec2(40.0, 10.0);
    for (position, expected) in [(square_center, true), (label_center, false)] {
        move_to(&mut context, position);
        mouse(&mut context, ElementState::Pressed);
        mouse(&mut context, ElementState::Released);
        let response = build_checkbox(&mut context, &mut checked, true);
        assert!(response.clicked());
        assert!(response.changed());
        assert_eq!(checked, expected);
        context.request_repaint();
        let response = build_checkbox(&mut context, &mut checked, true);
        assert!(!response.clicked());
        assert!(!response.changed());
        assert_eq!(checked, expected);
    }
}
