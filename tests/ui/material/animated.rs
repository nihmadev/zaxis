use super::*;
use std::time::Duration;

const SPIN: &str = "fn material(in: MaterialInput, p: Params) -> vec4<f32> { return premultiply(vec4<f32>(sin(in.time) * 0.5 + 0.5, 0.0, 0.0, 1.0)); }";

fn tick(c: &mut Context, now: Instant, id: MaterialId) {
    frame(c, now, |ui| {
        ui.material(rect(), id).show(ui);
    });
}

fn block(c: &mut Context) -> Vec<u8> {
    let draw = material_commands(c)[0].material.clone().unwrap();
    c.draw_data().material_uniforms[draw.uniforms.start as usize..draw.uniforms.end as usize]
        .to_vec()
}

#[test]
fn a_static_material_never_asks_for_frames() {
    let mut c = setup(1.0);
    let id = c.register_material(&Material::new("still", SPIN));
    let now = Instant::now();
    tick(&mut c, now, id);
    tick(&mut c, now, id);
    assert!(!c.wants_animation_frame());
    assert!(!c.needs_repaint_at(now + Duration::from_secs(30)));
}

#[test]
fn an_animated_material_asks_for_frames_only_while_visible() {
    let mut c = setup(1.0);
    let id = c.register_material(&Material::new("spin", SPIN).animated());
    let mut now = Instant::now();
    tick(&mut c, now, id);
    assert!(c.wants_animation_frame());
    assert!(c.next_repaint().is_some());
    // Time advances on the pass clock and reaches the uniform block.
    now += Duration::from_millis(500);
    tick(&mut c, now, id);
    let block = block(&mut c);
    let time = f32::from_ne_bytes(block[8..12].try_into().unwrap());
    let delta = f32::from_ne_bytes(block[12..16].try_into().unwrap());
    assert!((time - 0.5).abs() < 1e-3, "time {time}");
    assert!((delta - 0.5).abs() < 1e-3, "delta {delta}");
    // Not drawn any more: no more frames.
    now += Duration::from_millis(16);
    frame(&mut c, now, |_| {});
    assert!(!c.wants_animation_frame());
}

#[test]
fn a_clipped_out_animated_material_stops_asking() {
    let mut c = setup(1.0);
    let id = c.register_material(&Material::new("spin", SPIN).animated());
    let now = Instant::now();
    frame(&mut c, now, |ui| {
        ScrollArea::vertical()
            .max_height(60.0)
            .id_source("s")
            .show(ui, |ui| {
                let slot = ui.allocate_space(vec2(100.0, 400.0));
                let far = Rect::from_min_size(slot.min + vec2(0.0, 300.0), vec2(100.0, 50.0));
                ui.material(far, id).show(ui);
            });
    });
    assert!(!c.wants_animation_frame(), "scrolled out of its clip");
}

#[test]
fn reduced_motion_freezes_time_and_requests_nothing() {
    let mut c = setup(1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    let id = c.register_material(&Material::new("spin", SPIN).animated());
    tick(&mut c, Instant::now(), id);
    assert!(!c.wants_animation_frame());
    assert_eq!(&block(&mut c)[8..16], &[0; 8]);
}

#[test]
fn a_draw_can_opt_out_of_the_clock_of_an_animated_material() {
    let mut c = setup(1.0);
    let id = c.register_material(&Material::new("spin", SPIN).animated());
    let mut now = Instant::now();
    frame(&mut c, now, |ui| {
        ui.material(rect(), id).animated(false).show(ui);
    });
    now += Duration::from_millis(700);
    frame(&mut c, now, |ui| {
        ui.material(rect(), id).animated(false).show(ui);
    });
    // Time stays zero and nothing asks for frames: a still card of an animated material.
    assert!(!c.wants_animation_frame());
    assert!(!c.needs_repaint_at(now + Duration::from_secs(10)));
    assert_eq!(&block(&mut c)[8..16], &[0; 8]);
    frame(&mut c, now, |ui| {
        ui.material(rect(), id).show(ui);
    });
    assert!(c.wants_animation_frame());
}
