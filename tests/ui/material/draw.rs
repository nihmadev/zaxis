use super::*;

fn draw(ui: &mut Ui<'_>, id: MaterialId, x: f32) {
    ui.material(rect(), id).params(params(x)).show(ui);
}

#[test]
fn a_draw_becomes_a_command_with_its_id_and_uniform_block() {
    let mut c = setup(1.0);
    let id = c.register_material(&glow());
    frame(&mut c, Instant::now(), |ui| draw(ui, id, 0.25));
    let commands = material_commands(&mut c);
    assert_eq!(commands.len(), 1);
    let draw = commands[0].material.as_ref().unwrap();
    assert_eq!(draw.id, id);
    let data = c.draw_data();
    assert_eq!(data.materials.len(), 1);
    assert_eq!(data.materials[0].id, id);
    assert!(data.materials[0].wgsl.contains("fn fs_material"));
    let block = &data.material_uniforms[draw.uniforms.start as usize..draw.uniforms.end as usize];
    // Frame data, then the two parameters: size, no time, the whole texture.
    assert_eq!(block.len(), FRAME_BYTES + 32);
    assert_eq!(draw.uniforms.start % 16, 0);
    let f = |at: usize| f32::from_ne_bytes(block[at..at + 4].try_into().unwrap());
    assert_eq!([f(0), f(4), f(8), f(12)], [120.0, 80.0, 0.0, 0.0]);
    assert_eq!([f(16), f(20), f(24), f(28)], [0.0, 0.0, 1.0, 1.0]);
    assert_eq!([f(32), f(36)], [0.25, 0.5]);
}

#[test]
fn equal_neighbours_merge_and_different_ones_do_not() {
    let mut c = setup(1.0);
    let a = c.register_material(&glow());
    let b = c.register_material(&glow().animated());
    frame(&mut c, Instant::now(), |ui| {
        draw(ui, a, 0.25);
        draw(ui, a, 0.25);
        draw(ui, a, 0.75);
        draw(ui, b, 0.75);
    });
    let commands = material_commands(&mut c);
    // Two equal draws merge; another parameter value and another material start new commands.
    assert_eq!(commands.len(), 3, "{commands:?}");
    let ids: Vec<_> = commands
        .iter()
        .map(|c| c.material.as_ref().unwrap().id)
        .collect();
    assert_eq!(ids, [a, a, b]);
    assert_eq!(c.draw_data().materials.len(), 2);
    assert!(
        commands[0].indices.len() >= 2 * 6,
        "the merged command covers both shapes"
    );
}

#[test]
fn changing_a_parameter_keeps_the_mesh() {
    let mut c = setup(1.0);
    let id = c.register_material(&glow());
    let now = Instant::now();
    frame(&mut c, now, |ui| draw(ui, id, 0.1));
    let stats = c.probe().stats;
    let uniforms = c.draw_data().material_uniforms.clone();
    let revision = c.draw_data().revision;
    for n in 1..20 {
        frame(&mut c, now, |ui| draw(ui, id, 0.1 + n as f32 * 0.04));
    }
    let after = c.probe().stats;
    assert_eq!(after.tessellated_elements, stats.tessellated_elements);
    assert_eq!(after.geometry_bytes_copied, stats.geometry_bytes_copied);
    assert!(after.reused_elements > stats.reused_elements);
    assert_ne!(c.draw_data().material_uniforms, uniforms);
    assert!(
        c.draw_data().revision > revision,
        "a changed block is a new revision"
    );
    // The partial update names no vertex or index: nothing is uploaded again.
    let update = c.draw_data().geometry_update.clone().unwrap();
    assert!(update.vertices.is_empty() && update.indices.is_empty());
}

#[test]
fn an_unchanged_frame_changes_nothing() {
    let mut c = setup(1.0);
    let id = c.register_material(&glow());
    let now = Instant::now();
    frame(&mut c, now, |ui| draw(ui, id, 0.5));
    frame(&mut c, now, |ui| draw(ui, id, 0.5));
    let revision = c.draw_data().revision;
    frame(&mut c, now, |ui| draw(ui, id, 0.5));
    assert_eq!(c.draw_data().revision, revision);
}

#[test]
fn the_clip_of_a_scroll_area_reaches_the_command() {
    let mut c = setup(1.0);
    let id = c.register_material(&glow());
    frame(&mut c, Instant::now(), |ui| {
        ScrollArea::vertical()
            .max_height(60.0)
            .id_source("s")
            .show(ui, |ui| {
                let slot = ui.allocate_space(vec2(100.0, 240.0));
                let tall = Rect::from_min_size(slot.min, vec2(100.0, 200.0));
                ui.material(tall, id).params(params(0.5)).show(ui);
            });
    });
    let commands = material_commands(&mut c);
    assert_eq!(commands.len(), 1);
    let clip = commands[0].clip_rect;
    assert!(
        clip.size().y <= 61.0 && clip.size().y > 0.0,
        "clipped to the scroll viewport: {clip:?}"
    );
}

#[test]
fn the_draw_is_released_with_its_widget() {
    let mut c = setup(1.0);
    let id = c.register_material(&glow());
    let now = Instant::now();
    frame(&mut c, now, |ui| draw(ui, id, 0.5));
    assert!(!c.draw_data().material_uniforms.is_empty());
    frame(&mut c, now, |_| {});
    let data = c.draw_data();
    assert!(data.material_uniforms.is_empty());
    assert!(data.materials.is_empty());
    assert!(data
        .commands
        .iter()
        .all(|command| command.material.is_none()));
    // The registration itself stays: it is shared and idempotent.
    assert_eq!(c.shared_resources().material_count(), 1);
}

#[test]
fn an_unusable_material_paints_its_fallback_and_reports_once() {
    let mut c = setup(1.0);
    let broken = c.register_material(&Material::new(
        "broken",
        "fn material(in: MaterialInput, p: Params) -> vec4<f32> { return 1.0; }",
    ));
    let now = Instant::now();
    for _ in 0..3 {
        frame(&mut c, now, |ui| {
            let fallback = Color::rgb(200, 30, 30);
            assert!(!ui.material(rect(), broken).fallback(fallback).show(ui));
        });
    }
    assert!(material_commands(&mut c).is_empty());
    assert!(c.draw_data().materials.is_empty());
    // Unknown ids and wrong parameters fall back as well.
    frame(&mut c, now, |ui| {
        assert!(!ui.material(rect(), MaterialId(12345)).show(ui));
        let id = ui.context().register_material(&glow());
        let wrong = Params::new().f32("nope", 1.0);
        assert!(!ui.material(rect(), id).params(wrong).show(ui));
        assert!(ui.material(rect(), id).params(params(0.5)).show(ui));
    });
    assert_eq!(material_commands(&mut c).len(), 1);
}

#[test]
fn material_inside_a_scroll_area_follows_the_scroll_without_tessellating() {
    let mut c = setup(1.0);
    let id = c.register_material(&glow());
    let now = Instant::now();
    let build = |ui: &mut Ui<'_>| {
        ScrollArea::vertical()
            .max_height(120.0)
            .id_source("s")
            .show(ui, |ui| {
                for n in 0..3 {
                    let slot = ui.allocate_space(vec2(100.0, 130.0));
                    let shape = Rect::from_min_size(slot.min, vec2(100.0, 100.0));
                    ui.material(shape, id)
                        .id_source(n)
                        .params(params(0.5))
                        .show(ui);
                }
            });
    };
    frame(&mut c, now, build);
    assert!(!material_commands(&mut c).is_empty());
    let tessellated = c.probe().stats.tessellated_elements;
    c.scroll_wheel(vec2(0.0, -40.0));
    frame(&mut c, now, build);
    assert_eq!(c.probe().stats.tessellated_elements, tessellated);
}

#[test]
fn visual_scale_and_opacity_reach_the_block_and_the_mesh() {
    let mut c = setup(1.0);
    let id = c.register_material(&glow());
    frame(&mut c, Instant::now(), |ui| {
        let transform = Transform::around(vec2(0.0, 0.0), 2.0, vec2(0.0, 0.0));
        ui.visual("zoom", transform, 0.5, |ui| draw(ui, id, 0.5));
    });
    let commands = material_commands(&mut c);
    assert_eq!(commands.len(), 1);
    let data = c.draw_data();
    let command = &commands[0];
    let draw = command.material.as_ref().unwrap();
    let block = &data.material_uniforms[draw.uniforms.start as usize..draw.uniforms.end as usize];
    let f = |at: usize| f32::from_ne_bytes(block[at..at + 4].try_into().unwrap());
    // The shape is shown twice as large: the shader sees its displayed size.
    assert_eq!([f(0), f(4)], [240.0, 160.0]);
    let alphas = data.indices[command.indices.start as usize..command.indices.end as usize]
        .iter()
        .map(|i| data.vertices[*i as usize].color[3])
        .fold(0.0_f32, f32::max);
    assert!(
        (alphas - 0.5).abs() < 1e-3,
        "opacity folded into the vertex color: {alphas}"
    );
}

#[test]
fn a_backdrop_material_is_a_backdrop_effect_and_never_merges() {
    let mut c = setup(1.0);
    let id = c.register_material(
        &Material::new(
            "glass",
            "fn material(in: MaterialInput, p: Params) -> vec4<f32> { return backdrop_blurred(in); }",
        )
        .reads_backdrop(),
    );
    frame(&mut c, Instant::now(), |ui| {
        ui.material(rect(), id).backdrop_blur(6.0).show(ui);
        ui.material(rect(), id).backdrop_blur(6.0).show(ui);
    });
    let commands = material_commands(&mut c);
    assert_eq!(
        commands.len(),
        2,
        "each backdrop draw reads what is behind it"
    );
    assert!(commands.iter().all(|c| c.blur == Some(6.0)));
}
