use super::*;

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
                        responses.push(ui.button("Disabled button"));
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
