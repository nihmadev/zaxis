use super::*;

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
fn theme_blur_only_filters_background_surfaces() {
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
        "background blur must not add filtering passes to controls"
    );
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
