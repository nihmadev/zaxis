use std::time::Duration;

use zaxis::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
};
use zaxis::{
    vec2, Button, Checkbox, Color, Context, Id, Rect, Response, Slider, SliderStatus, Window,
};

#[test]
fn backdrop_effects_preserve_order_and_invalidate_radius_without_retessellation() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let build = |context: &mut Context, radius| {
        context.run(|context| {
            context.paint_background(zaxis::Shape::rect(
                Rect::from_min_size(vec2(0.0, 0.0), vec2(200.0, 200.0)),
                Color::WHITE,
            ));
            Window::new("Glass").blur(radius).show(context, |ui| {
                ui.add(Button::new("Inherited"));
                ui.add(Button::new("Override").blur(3.0));
                let rect = ui.allocate_space(vec2(100.0, 30.0));
                zaxis::Blur::new(rect)
                    .radius(radius)
                    .corner_radius(8.0)
                    .show(ui);
                ui.label("Sharp foreground");
            });
        });
    };
    build(&mut context, 8.0);
    let data = context.draw_data();
    let effects: Vec<_> = data
        .commands
        .iter()
        .enumerate()
        .filter(|(_, c)| c.blur.is_some())
        .collect();
    assert_eq!(effects.len(), 3);
    assert!(
        effects[0].0 > 0,
        "background must be drawn before window blur"
    );
    assert!(
        effects[2].0 < data.commands.len() - 1,
        "foreground must follow blur"
    );
    assert_eq!(
        effects
            .iter()
            .map(|(_, c)| c.blur.unwrap())
            .collect::<Vec<_>>(),
        [8.0, 3.0, 8.0]
    );
    let before = context.cache_stats();
    let revision = data.revision;
    build(&mut context, 8.0);
    assert_eq!(context.draw_data().revision, revision);
    build(&mut context, 16.0);
    assert!(context.draw_data().revision > revision);
    assert_eq!(
        context.cache_stats().tessellated_elements,
        before.tessellated_elements
    );
    build(&mut context, 0.0);
    assert_eq!(
        context
            .draw_data()
            .commands
            .iter()
            .filter(|c| c.blur.is_some())
            .count(),
        1
    );
}

#[test]
fn theme_blur_reaches_controls_and_local_zero_disables_it() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 2.0);
    context.set_style(context.style().clone().blur(10.0));
    let mut checked = true;
    let mut value = 0.5;
    context.run(|context| {
        Window::new("Glass").show(context, |ui| {
            ui.button("Inherited");
            ui.add(Button::new("Opaque").blur(0.0));
            ui.checkbox(&mut checked, "Inherited checkbox");
            ui.add(Slider::new(&mut value, 0.0..=1.0));
        });
        Window::new("Opaque window")
            .blur(f32::NAN)
            .show(context, |_| {});
    });
    let effects: Vec<_> = context
        .draw_data()
        .commands
        .iter()
        .filter_map(|c| c.blur)
        .collect();
    assert_eq!(
        effects,
        [10.0],
        "controls inherit the panel's filtered backdrop without extra GPU passes"
    );
}

fn context() -> Context {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    context
}

fn build(context: &mut Context, label: &str) -> Response {
    let mut response = None;
    context.run(|context| {
        Window::new("Test").show(context, |ui| {
            ui.label(label);
            response = Some(ui.button("Click"));
        });
    });
    response.expect("the UI pass must run")
}

fn move_to(context: &mut Context, position: zaxis::Vec2) {
    let scale = context.scale_factor();
    context.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(
            f64::from(position.x * scale),
            f64::from(position.y * scale),
        ),
    });
}

fn build_checkbox(context: &mut Context, checked: &mut bool, enabled: bool) -> Response {
    let mut response = None;
    context.run(|context| {
        Window::new("Test").show(context, |ui| {
            response = Some(ui.add(Checkbox::new(checked, "Checked").enabled(enabled)));
        });
    });
    response.expect("the checkbox UI pass must run")
}

fn mouse(context: &mut Context, state: ElementState) {
    context.on_window_event(&WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state,
        button: MouseButton::Left,
    });
}

#[test]
fn hover_preserves_layout_caches_and_pointer_focus_without_a_ring() {
    use zaxis::{winit::keyboard::KeyCode, WidgetState};
    let mut context = context();
    let mut style = context.style().clone();
    style.motion.reduced_motion = true;
    context.set_style(style);
    let idle = build(&mut context, "Hover test");
    let idle_vertices = context.draw_data().vertices.clone();
    move_to(&mut context, idle.rect.center());
    let hovered = build(&mut context, "Hover test");
    assert_eq!(hovered.state(), WidgetState::Hovered);
    assert_eq!(hovered.rect, idle.rect);
    assert!(context.draw_data().vertices.len() > idle_vertices.len());
    let stats = context.cache_stats();
    let revision = context.draw_data().revision;
    build(&mut context, "Hover test");
    assert_eq!(context.draw_data().revision, revision);
    assert_eq!(
        context.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    mouse(&mut context, ElementState::Pressed);
    let pressed = build(&mut context, "Hover test");
    assert_eq!(pressed.state(), WidgetState::Pressed);
    assert!(pressed.has_focus && !pressed.focus_visible);
    assert_eq!(context.draw_data().vertices.len(), idle_vertices.len());
    mouse(&mut context, ElementState::Released);
    assert!(build(&mut context, "Hover test").clicked());
    move_to(&mut context, vec2(790.0, 590.0));
    let focused = build(&mut context, "Hover test");
    assert!(focused.has_focus && !focused.focus_visible);
    assert_eq!(context.draw_data().vertices, idle_vertices);
    context.on_key_event(KeyCode::Tab, ElementState::Pressed, false);
    assert!(build(&mut context, "Hover test").focus_visible);
}

#[test]
fn hover_wrapper_custom_paint_disabled_priority_and_override_scope() {
    use zaxis::{Gradient, Hover, HoverStyle, WidgetState};
    let mut context = context();
    let gradient = HoverStyle::gradient(Gradient::new(
        Color::rgb(20, 40, 180),
        Color::rgb(180, 40, 20),
    ));
    let mut calls = 0;
    let mut draw = |context: &mut Context, enabled, preset| {
        let mut responses = Vec::new();
        context.run(|context| {
            Window::new("Test").show(context, |ui| {
                responses.push(
                    ui.add(
                        Hover::new(Button::new("Wrapped").enabled(enabled))
                            .style(preset)
                            .on_hover(|ui, response| {
                                calls += 1;
                                ui.paint(zaxis::Shape::rect(
                                    response.rect.shrink(8.0),
                                    Color::rgba(10, 200, 70, 20),
                                ));
                            }),
                    ),
                );
                responses.push(ui.horizontal(|ui| ui.button("Sibling")));
            });
        });
        responses
    };
    let idle = draw(&mut context, true, gradient);
    move_to(&mut context, idle[0].rect.center());
    let hovered = draw(&mut context, true, gradient);
    assert_eq!(hovered[0].id, idle[0].id);
    assert_eq!(hovered[0].rect, idle[0].rect);
    assert_eq!(
        hovered[1].id, idle[1].id,
        "conditional custom paint must not shift subsequent layout IDs"
    );
    assert_eq!(hovered[0].state(), WidgetState::Hovered);
    assert!(context
        .draw_data()
        .vertices
        .iter()
        .any(|v| v.color[2] > v.color[0] * 2.0 && v.color[3] > 0.0));
    let disabled = draw(&mut context, false, gradient);
    assert_eq!(disabled[0].state(), WidgetState::Disabled);
    assert!(!context
        .draw_data()
        .vertices
        .iter()
        .any(|v| v.color[2] > v.color[0] * 2.0 && v.color[3] > 0.0));
    move_to(&mut context, idle[1].rect.center());
    draw(&mut context, true, gradient);
    let sibling = context.draw_data().vertices.clone();
    draw(&mut context, true, HoverStyle::NONE);
    assert_eq!(
        context.draw_data().vertices,
        sibling,
        "wrapper style must not leak to siblings"
    );
    assert_eq!(
        calls, 1,
        "custom paint skips disabled controls and non-hovered passes"
    );
}

fn build_slider(context: &mut Context, value: &mut f32, enabled: bool) -> Response {
    let mut response = None;
    context.run(|context| {
        Window::new("Test").show(context, |ui| {
            response = Some(ui.add(Slider::new(value, -10.0..=10.0).step(2.0).enabled(enabled)));
        });
    });
    response.expect("the slider UI pass must run")
}

#[test]
fn slider_snaps_on_press_and_captures_motion_and_release_outside() {
    let mut context = context();
    let mut value = -10.0;
    let response = build_slider(&mut context, &mut value, true);
    move_to(&mut context, response.rect.center() + vec2(22.0, 0.0));
    mouse(&mut context, ElementState::Pressed);
    let response = build_slider(&mut context, &mut value, true);
    assert_eq!(value, 2.0);
    assert!(response.pressed && response.has_focus && response.changed());
    move_to(&mut context, vec2(900.0, 700.0));
    let response = build_slider(&mut context, &mut value, true);
    assert_eq!(value, 10.0);
    assert!(response.pressed && response.changed());
    move_to(&mut context, vec2(0.0, 0.0));
    mouse(&mut context, ElementState::Released);
    let response = build_slider(&mut context, &mut value, true);
    assert_eq!(value, -10.0);
    assert!(!response.pressed && response.changed());
    context.request_repaint();
    assert!(!build_slider(&mut context, &mut value, true).changed());
}

#[test]
fn slider_fast_drag_is_retained_and_disabling_cancels_capture() {
    let mut context = context();
    let mut value = -10.0;
    let response = build_slider(&mut context, &mut value, true);
    move_to(&mut context, response.rect.center());
    mouse(&mut context, ElementState::Pressed);
    move_to(&mut context, vec2(900.0, 700.0));
    mouse(&mut context, ElementState::Released);
    let response = build_slider(&mut context, &mut value, true);
    assert_eq!(value, 10.0);
    assert!(response.changed());
    move_to(&mut context, response.rect.center());
    mouse(&mut context, ElementState::Pressed);
    let response = build_slider(&mut context, &mut value, false);
    assert!(!response.changed() && !response.has_focus && !response.pressed);
    assert_eq!(value, 10.0);
    move_to(&mut context, vec2(0.0, 0.0));
    context.request_repaint();
    let response = build_slider(&mut context, &mut value, true);
    assert!(!response.changed() && !response.pressed);
    assert_eq!(value, 10.0);
    mouse(&mut context, ElementState::Released);
}

#[test]
fn slider_normalization_disabled_state_color_and_cache() {
    let mut context = context();
    let mut style = context.style().clone();
    style.motion.reduced_motion = true;
    context.set_style(style);
    let mut value = 99.0;
    assert!(!build_slider(&mut context, &mut value, false).changed());
    assert_eq!(value, 99.0);
    context.request_repaint();
    assert!(build_slider(&mut context, &mut value, true).changed());
    assert_eq!(value, 10.0);
    let stats = context.cache_stats();
    context.request_repaint();
    assert!(!build_slider(&mut context, &mut value, true).changed());
    assert_eq!(
        stats.tessellated_elements,
        context.cache_stats().tessellated_elements
    );
    let revision = context.draw_data().revision;
    assert!(!build_slider(&mut context, &mut value, true).changed());
    assert_eq!(context.draw_data().revision, revision);
    assert!(!context.needs_repaint());

    value = f32::NAN;
    context.request_repaint();
    assert!(build_slider(&mut context, &mut value, true).changed());
    assert_eq!(value, -10.0);
    let draw = |context: &mut Context, status, color| {
        context.request_repaint();
        context.run(|context| {
            Window::new("Test").show(context, |ui| {
                ui.add(Slider::new(&mut 0.5, 0.0..=1.0).status(status).color(color));
            });
        });
    };
    draw(&mut context, SliderStatus::Error, Color::rgb(0, 255, 0));
    let vertices = context.draw_data().vertices.clone();
    draw(&mut context, SliderStatus::Warning, Color::rgb(0, 255, 0));
    assert_eq!(vertices, context.draw_data().vertices);
    draw(&mut context, SliderStatus::Warning, Color::rgb(0, 0, 255));
    assert_ne!(vertices, context.draw_data().vertices);
}

#[test]
fn unchanged_redraw_evaluates_ui_and_preserves_geometry_allocations() {
    let mut context = context();
    build(&mut context, "Hello");
    let stats = context.cache_stats();
    let revision = context.draw_data().revision;
    let vertices = context.draw_data().vertices.as_ptr();
    let indices = context.draw_data().indices.as_ptr();
    build(&mut context, "Hello");
    assert_eq!(context.cache_stats().ui_passes, stats.ui_passes + 1);
    assert_eq!(
        context.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    assert_eq!(
        context.cache_stats().geometry_rebuilds,
        stats.geometry_rebuilds
    );
    assert!(!context.needs_repaint());
    assert_eq!(context.draw_data().revision, revision);
    assert_eq!(context.draw_data().vertices.as_ptr(), vertices);
    assert_eq!(context.draw_data().indices.as_ptr(), indices);
}

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

#[test]
fn checkbox_external_state_change_adds_the_checkmark_and_reuses_other_geometry() {
    let mut context = context();
    let mut checked = false;
    build_checkbox(&mut context, &mut checked, true);
    let stats = context.cache_stats();
    let vertices = context.draw_data().vertices.len();
    let indices = context.draw_data().indices.len();
    checked = true;
    let response = build_checkbox(&mut context, &mut checked, true);
    assert!(!response.changed());
    assert!(context.draw_data().vertices.len() > vertices);
    assert!(context.draw_data().indices.len() > indices);
    assert_eq!(
        context.cache_stats().tessellated_elements,
        stats.tessellated_elements + 1
    );
    let stats = context.cache_stats();
    build_checkbox(&mut context, &mut checked, true);
    assert_eq!(
        context.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    assert_eq!(
        context.cache_stats().geometry_rebuilds,
        stats.geometry_rebuilds
    );
    assert!(!context.needs_repaint());
}

#[test]
fn disabled_checkbox_ignores_clicks_and_release_outside_cancels_a_toggle() {
    let mut context = context();
    let mut checked = true;
    let response = build_checkbox(&mut context, &mut checked, false);
    move_to(&mut context, response.rect.center());
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    let response = build_checkbox(&mut context, &mut checked, false);
    assert!(checked);
    assert!(!response.clicked());
    assert!(!response.changed());
    context.request_repaint();
    let response = build_checkbox(&mut context, &mut checked, true);
    move_to(&mut context, response.rect.center());
    mouse(&mut context, ElementState::Pressed);
    move_to(&mut context, vec2(2.0, 2.0));
    mouse(&mut context, ElementState::Released);
    assert!(!build_checkbox(&mut context, &mut checked, true).changed());
    assert!(checked);
}

#[test]
fn disabled_groups_inherit_through_rows_and_restore_the_parent() {
    let mut context = context();
    let mut checked = false;
    let mut value = 99.0;
    let draw = |context: &mut Context, checked: &mut bool, value: &mut f32| {
        let mut responses = Vec::new();
        context.run(|context| {
            Window::new("Test")
                .default_size(vec2(600.0, 400.0))
                .show(context, |ui| {
                    ui.add_enabled_ui(false, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_enabled_ui(true, |ui| {
                                assert!(!ui.is_enabled());
                                responses.push(ui.checkbox(checked, "Disabled checkbox"));
                                responses.push(ui.slider(value, 0.0..=10.0));
                            });
                        });
                        responses.push(ui.button_enabled(true, "Disabled button"));
                    });
                    assert!(ui.is_enabled());
                    responses.push(ui.button("Outside"));
                });
        });
        responses
    };
    let responses = draw(&mut context, &mut checked, &mut value);
    for response in &responses[..3] {
        move_to(&mut context, response.rect.center());
        mouse(&mut context, ElementState::Pressed);
        mouse(&mut context, ElementState::Released);
        let responses = draw(&mut context, &mut checked, &mut value);
        assert!(responses[..3]
            .iter()
            .all(|r| !r.clicked() && !r.changed() && !r.has_focus));
        assert!(!checked);
        assert_eq!(value, 99.0);
    }
    move_to(&mut context, responses[3].rect.center());
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    assert!(draw(&mut context, &mut checked, &mut value)[3].clicked());
}

#[test]
fn automatic_slider_ids_are_distinct_and_unaffected_by_inserted_text() {
    let mut context = context();
    let mut values = [0.0, 5.0];
    let draw = |context: &mut Context, values: &mut [f32; 2], insert_text: bool| {
        let mut responses = Vec::new();
        context.run(|context| {
            Window::new("Test").show(context, |ui| {
                if insert_text {
                    ui.label("Inserted before sliders");
                }
                for value in values {
                    responses.push(ui.slider_labeled(value, 0.0..=10.0, "Repeated caption"));
                }
            });
        });
        responses
    };
    let first = draw(&mut context, &mut values, false);
    assert_ne!(first[0].id, first[1].id);
    move_to(&mut context, first[1].rect.center());
    mouse(&mut context, ElementState::Pressed);
    draw(&mut context, &mut values, false);
    let second = draw(&mut context, &mut values, true);
    assert_eq!(first[0].id, second[0].id);
    assert_eq!(first[1].id, second[1].id);
    assert!(!second[0].has_focus && second[1].has_focus && second[1].pressed);
    assert_eq!(values[0], 0.0);
}

#[test]
fn tabs_bind_selection_update_earlier_tabs_and_keep_ids_when_captions_change() {
    let mut context = context();
    let mut style = context.style().clone();
    style.motion.reduced_motion = true;
    context.set_style(style);
    let mut selected = 0;
    let draw = |context: &mut Context, selected: &mut i32, renamed: bool, enabled: bool| {
        let mut responses = Vec::new();
        context.run(|context| {
            Window::new("Test").show(context, |ui| {
                responses = ui.add_enabled_ui(enabled, |ui| {
                    ui.tab_bar(
                        selected,
                        [
                            (0, if renamed { "Renamed" } else { "First" }),
                            (1, "Second"),
                        ],
                    )
                });
            });
        });
        responses
    };
    let first = draw(&mut context, &mut selected, false, true);
    move_to(&mut context, first[1].rect.center());
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    assert!(draw(&mut context, &mut selected, false, true)[1].changed());
    assert_eq!(selected, 1);
    assert!(context.needs_repaint());
    let revision = context.draw_data().revision;
    assert!(!draw(&mut context, &mut selected, false, true)[1].changed());
    assert!(context.draw_data().revision > revision);
    assert!(!context.needs_repaint());
    let renamed = draw(&mut context, &mut selected, true, false);
    assert_eq!(first[0].id, renamed[0].id);
    move_to(&mut context, renamed[0].rect.center());
    mouse(&mut context, ElementState::Pressed);
    mouse(&mut context, ElementState::Released);
    assert!(!draw(&mut context, &mut selected, true, false)[0].clicked());
    assert_eq!(selected, 1);
}
