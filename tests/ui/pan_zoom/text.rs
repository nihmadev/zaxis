use super::*;

#[test]
fn text_edit_selection_caret_ime_and_popup_portal_follow_the_camera() {
    for dpi in [1.0, 1.25, 1.5, 2.0] {
        let mut c = setup(dpi);
        let mut state = PanZoomState {
            scale: 2.0,
            translation: vec2(80.0, 60.0),
        };
        let mut text = "abcdef".to_string();
        let mut open = false;
        let make =
            |c: &mut Context, state: &mut PanZoomState, text: &mut String, open: &mut bool| {
                draw(c, state, |ui, _| {
                    let edit = ui.at(
                        "edit",
                        Rect::from_min_size(vec2(-20.0, -10.0), vec2(150.0, 45.0)),
                        |ui| ui.add(TextEdit::new(text).id_source("field").width(140.0)),
                    );
                    Popup::new("popup", edit.rect)
                        .size(vec2(200.0, 160.0))
                        .show(ui, open, |ui| ui.button("Popup action"));
                    edit
                })
            };
        let out = make(&mut c, &mut state, &mut text, &mut open);
        let screen = c.visual_rect(out.inner.id, out.inner.rect);
        pointer(&mut c, screen.min + vec2(20.0, 20.0));
        button(&mut c, ElementState::Pressed);
        pointer(&mut c, screen.min + vec2(75.0, 20.0));
        button(&mut c, ElementState::Released);
        let out = make(&mut c, &mut state, &mut text, &mut open);
        assert!(out.inner.has_focus && !out.panned);
        let selected = &c.probe().text_edits[&out.inner.id];
        assert_ne!(selected.buffer.cursor, selected.buffer.anchor);
        c.on_input(InputEvent::Ime(ImeEvent::Preedit("Ж".into(), Some((0, 2)))));
        make(&mut c, &mut state, &mut text, &mut open);
        let ime = c.probe().ime_area.unwrap();
        assert!(!ime.is_empty() && screen.contains(ime.center()));
        c.on_input(InputEvent::Ime(ImeEvent::Commit("Ж".into())));
        let out = make(&mut c, &mut state, &mut text, &mut open);
        assert!(text.contains('Ж') && out.inner.changed());
        state.translation += vec2(5.0, 10.0);
        state.scale = 1.5;
        let out = make(&mut c, &mut state, &mut text, &mut open);
        assert!(out.inner.has_focus);
        let shown = c.visual_rect(out.inner.id, out.inner.rect);
        assert!(shown.contains(c.probe().ime_area.unwrap().center()));
        open = true;
        let out = make(&mut c, &mut state, &mut text, &mut open);
        let popup = c.probe().popup.as_ref().unwrap();
        near(
            popup.anchor.center(),
            c.visual_rect(out.inner.id, out.inner.rect).center(),
        );
        assert!(popup.rect.min.x >= 0.0 && popup.rect.max.x <= c.viewport().max.x);
        assert!(popup.rect.min.y >= 0.0 && popup.rect.max.y <= c.viewport().max.y);
        let before = state;
        assert!(wheel(
            &mut c,
            vec2(350.0, 250.0),
            -100.0,
            ModifiersState::CONTROL
        ));
        make(&mut c, &mut state, &mut text, &mut open);
        assert_eq!(state, before, "popup owns input over the underlying camera");
    }
}

#[test]
fn image_demand_and_glyph_raster_resolution_include_zoom_while_idle_reuses_geometry() {
    let mut c = setup(1.5);
    c.decode_images_inline(Some(std::time::Duration::from_secs(1)));
    let mut state = PanZoomState::default();
    let make = |c: &mut Context, state: &mut PanZoomState| {
        draw(c, state, |ui, _| {
            ui.at(
                "text",
                Rect::from_min_size(Vec2::ZERO, vec2(150.0, 40.0)),
                |ui| ui.label("Zoom glyphs"),
            );
            ui.at("image", Rect::from_min_size(vec2(0.0, 50.0), vec2(100.0, 100.0)), |ui| {
            ui.add(Image::new(ImageSource::bytes(b"<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100'><circle cx='50' cy='50' r='40' fill='red'/></svg>")).size(vec2(100.0, 100.0)).alt("Circle"))
        });
        })
    };
    make(&mut c, &mut state);
    make(&mut c, &mut state);
    let raster_keys: Vec<_> = c
        .probe()
        .cache
        .iter()
        .map(|(id, cache)| (*id, cache.scale))
        .collect();
    state.scale = 4.0;
    make(&mut c, &mut state);
    make(&mut c, &mut state);
    assert!(raster_keys.iter().any(|(id, scale)| c
        .probe()
        .cache
        .get(id)
        .is_some_and(|cache| cache.scale > *scale)));
    let stats = c.cache_stats();
    let revision = c.draw_data().revision;
    make(&mut c, &mut state);
    assert_eq!(
        stats.tessellated_elements,
        c.cache_stats().tessellated_elements
    );
    assert_eq!(revision, c.draw_data().revision);
    assert!(!c.wants_animation_frame());
    assert!(c
        .draw_data()
        .textures
        .iter()
        .any(|texture| texture.size[0] >= 400));
}

#[cfg(feature = "accesskit")]
#[test]
fn accessibility_group_child_names_focus_actions_and_bounds_follow_displayed_geometry() {
    use zaxis::accesskit::Role;
    let mut c = setup(1.25);
    let mut tree = AccessTree::attach(&mut c);
    let mut state = PanZoomState {
        scale: 1.5,
        translation: vec2(50.0, 40.0),
    };
    let out = draw(&mut c, &mut state, |ui, _| {
        ui.at(
            "apply",
            Rect::from_min_size(vec2(-10.0, -5.0), vec2(130.0, 50.0)),
            |ui| ui.button("Apply"),
        )
    });
    tree.sync(&mut c);
    tree.validate();
    let group = tree.expect(Role::Group, "Pan and zoom");
    let child = tree.expect(Role::Button, "Apply");
    assert_eq!(tree.parent(child), Some(group));
    let rect = c.visual_rect(out.inner.id, out.inner.rect);
    let bounds = tree.node(child).bounds().unwrap();
    assert!(
        (vec2(bounds.x0 as f32, bounds.y0 as f32) - rect.min * 1.25)
            .abs()
            .max_element()
            <= 1.0
    );
    c.request_focus(out.inner.id);
    draw(&mut c, &mut state, |ui, _| {
        ui.at(
            "apply",
            Rect::from_min_size(vec2(-10.0, -5.0), vec2(130.0, 50.0)),
            |ui| ui.button("Apply"),
        )
    });
    tree.sync(&mut c);
    assert_eq!(tree.focus(), child);
}
