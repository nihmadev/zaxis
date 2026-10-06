//! Pixels of material draws read back from a real device: color, alpha, parameters, texture,
//! multisampling, the pipeline cache and recreation on a new device. Skipped, with a message,
//! when there is no adapter or `ZAXIS_SKIP_GPU_TESTS` is set.

mod dump;
mod harness;

use harness::*;
use zaxis::renderer::materials::MAX_PIPELINES;
use zaxis::{vec2, Color, ImageSource, Material, Params, Rect, TextureFilter};

#[test]
fn premultiplied_output_blends_once_at_every_scale_and_under_msaa() {
    let Some(mut gpu) = gpu("material blending") else {
        return;
    };
    for scale in [1.0, 1.25, 2.0] {
        for samples in [1, 4] {
            if samples > 1
                && !gpu
                    .adapter
                    .get_texture_format_features(FORMAT)
                    .flags
                    .sample_count_supported(samples)
            {
                eprintln!("SKIPPED {samples}x MSAA: unsupported");
                continue;
            }
            let mut c = context(scale);
            let half_red = c.register_material(&flat("return vec4<f32>(0.5, 0.0, 0.0, 0.5);"));
            let opaque_green = c.register_material(&flat("return vec4<f32>(0.0, 1.0, 0.0, 1.0);"));
            let data = scene(&mut c, scale, |ui| {
                ui.material(CELL, half_red).show(ui);
                let second = Rect::from_min_size(vec2(112.0, 64.0), vec2(80.0, 48.0));
                ui.material(second, opaque_green).opacity(0.5).show(ui);
            });
            let pixels = gpu.render(&data, scale, samples, Only::Materials, [0.0, 0.0, 1.0, 1.0]);
            let px = |x: f32, y: f32| at(&pixels, (x * scale) as u32, (y * scale) as u32);
            // Premultiplied (0.5, 0, 0, 0.5) over blue: one source term, one (1 - a) term.
            assert!(
                near(px(60.0, 70.0), [128, 0, 128, 255], 2),
                "{scale} {samples}: {:?}",
                px(60.0, 70.0)
            );
            // An opacity of 0.5 scales an opaque shader result once: green over blue.
            assert!(
                near(px(150.0, 70.0), [0, 128, 128, 255], 2),
                "{scale} {samples}: {:?}",
                px(150.0, 70.0)
            );
            assert!(
                near(px(240.0, 70.0), [0, 0, 255, 255], 0),
                "outside the shapes: {:?}",
                px(240.0, 70.0)
            );
        }
    }
}

#[test]
fn edges_cover_like_the_builtin_shape_pipeline() {
    let Some(mut gpu) = gpu("material coverage") else {
        return;
    };
    for scale in [1.0, 1.25, 1.5] {
        let mut c = context(scale);
        let white = c.register_material(&flat("return vec4<f32>(1.0);"));
        let materials = scene(&mut c, scale, |ui| {
            ui.material(CELL, white).corner_radius(16.0).show(ui);
        });
        let mut b = context(scale);
        b.run(|b| b.paint_background(zaxis::Shape::rect(CELL, Color::WHITE).corner_radius(16.0)));
        let plain = copy(b.draw_data());
        let with = gpu.render(&materials, scale, 1, Only::Materials, [0.0; 4]);
        let without = gpu.render(&plain, scale, 1, Only::Plain, [0.0; 4]);
        let partial = with.iter().filter(|p| p[3] > 8 && p[3] < 247).count();
        assert!(partial > 40, "antialiased contour expected at {scale}");
        let worst = with
            .iter()
            .zip(&without)
            .map(|(a, b)| a[3].abs_diff(b[3]))
            .max()
            .unwrap();
        if worst > 2 {
            let n = with
                .iter()
                .zip(&without)
                .position(|(a, b)| a[3].abs_diff(b[3]) > 2)
                .unwrap() as u32;
            eprintln!(
                "first mismatch at {:?}: {:?} vs {:?}",
                (n % SIZE[0], n / SIZE[0]),
                with[n as usize],
                without[n as usize]
            );
        }
        assert!(
            worst <= 2,
            "coverage of the material differs from the shape pipeline by {worst} at {scale}"
        );
    }
}

#[test]
fn inputs_are_local_uv_logical_size_scale_and_window_position() {
    let Some(mut gpu) = gpu("material inputs") else {
        return;
    };
    for scale in [1.0, 1.25, 2.0] {
        let mut c = context(scale);
        let uv = c.register_material(&flat("return vec4<f32>(in.uv.x, in.uv.y, 0.0, 1.0);"));
        let size = c.register_material(&flat(
            "return vec4<f32>(in.size.x / 255.0, in.size_px.x / 510.0, in.scale / 4.0, 1.0);",
        ));
        let position = c.register_material(&flat(
            "return vec4<f32>(in.position.x / 512.0, in.position.y / 256.0, 0.0, 1.0);",
        ));
        let data = scene(&mut c, scale, |ui| {
            ui.material(CELL, uv).show(ui);
            ui.material(
                Rect::from_min_size(vec2(112.0, 64.0), vec2(80.0, 48.0)),
                size,
            )
            .show(ui);
            ui.material(
                Rect::from_min_size(vec2(200.0, 64.0), vec2(48.0, 48.0)),
                position,
            )
            .show(ui);
        });
        let pixels = gpu.render(&data, scale, 1, Only::Materials, [0.0; 4]);
        let px = |x: f32, y: f32| {
            at(
                &pixels,
                (x * scale).round() as u32,
                (y * scale).round() as u32,
            )
        };
        // Local UV runs 0 to 1 over the shape whatever the DPI.
        let left = px(CELL.min.x + 4.0, CELL.center().y);
        let right = px(CELL.max.x - 4.0, CELL.center().y);
        assert!(
            left[0] < 20 && right[0] > 225,
            "{scale}: {left:?} {right:?}"
        );
        let middle = px(CELL.center().x, CELL.center().y);
        assert!(near(middle, [128, 128, 0, 255], 8), "{scale}: {middle:?}");
        // 80 logical pixels, 80 * scale physical, and the scale itself.
        let inputs = px(152.0, 72.0);
        let expect =
            [80.0 / 255.0, 80.0 * scale / 510.0, scale / 4.0].map(|v| (v * 255.0).round() as u8);
        assert!(
            near(inputs, [expect[0], expect[1], expect[2], 255], 2),
            "{scale}: {inputs:?} vs {expect:?}"
        );
        // The fragment position is in physical window pixels.
        let sample = px(224.0, 72.0);
        let (x, y) = (224.0 * scale, 72.0 * scale);
        assert!(
            near(
                sample,
                [(x / 512.0 * 255.0) as u8, (y / 256.0 * 255.0) as u8, 0, 255],
                3
            ),
            "{scale}: {sample:?}"
        );
    }
}

#[test]
fn draws_with_different_parameters_read_their_own_blocks() {
    let Some(mut gpu) = gpu("material parameters") else {
        return;
    };
    let mut c = context(1.0);
    let id = c.register_material(&flat("return vec4<f32>(p.level, 1.0 - p.level, 0.0, 1.0);"));
    let data = scene(&mut c, 1.0, |ui| {
        for n in 0..4 {
            let rect = Rect::from_min_size(vec2(16.0 + 90.0 * n as f32, 64.0), vec2(80.0, 48.0));
            ui.material(rect, id)
                .id_source(n)
                .params(Params::new().f32("level", n as f32 / 3.0))
                .show(ui);
        }
    });
    // Neighbours with other values are separate commands, each with its own dynamic offset.
    assert_eq!(
        data.commands
            .iter()
            .filter(|c| c.material.is_some())
            .count(),
        4
    );
    let pixels = gpu.render(&data, 1.0, 1, Only::Materials, [0.0; 4]);
    for n in 0..4u32 {
        let level = n as f32 / 3.0;
        let got = at(&pixels, 56 + 90 * n, 72);
        let want = [
            (level * 255.0).round() as u8,
            ((1.0 - level) * 255.0).round() as u8,
            0,
            255,
        ];
        assert!(near(got, want, 2), "draw {n}: {got:?} vs {want:?}");
    }
}

#[test]
fn the_widget_texture_is_sampled_through_its_own_sampler() {
    let Some(mut gpu) = gpu("material texture") else {
        return;
    };
    let mut c = context(1.0);
    c.decode_images_inline(Some(std::time::Duration::from_millis(50)));
    let id = c.register_material(&Material::new(
        "texture",
        "fn material(in: MaterialInput, p: Params) -> vec4<f32> { return widget_color(widget_uv(in)); }",
    ).reads_texture());
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let source = ImageSource::rgba([2, 2], pixels.to_vec());
    let mut data = None;
    for _ in 0..6 {
        let source = source.clone();
        data = Some(scene(&mut c, 1.0, |ui| {
            ui.material(CELL, id)
                .image(source)
                .filter(TextureFilter::Nearest)
                .show(ui);
        }));
    }
    let data = data.unwrap();
    assert!(
        !data.textures.is_empty(),
        "the image payload reaches the draw data"
    );
    let rendered = gpu.render(&data, 1.0, 1, Only::Materials, [0.0; 4]);
    let quadrant = |fx: f32, fy: f32| {
        at(
            &rendered,
            (CELL.min.x + CELL.size().x * fx) as u32,
            (CELL.min.y + CELL.size().y * fy) as u32,
        )
    };
    assert!(near(quadrant(0.25, 0.25), [255, 0, 0, 255], 2));
    assert!(near(quadrant(0.75, 0.25), [0, 255, 0, 255], 2));
    assert!(near(quadrant(0.25, 0.75), [0, 0, 255, 255], 2));
    assert!(near(quadrant(0.75, 0.75), [255, 255, 255, 255], 2));
}

#[test]
fn pipelines_are_cached_bounded_and_rebuilt_on_a_new_device() {
    let Some(mut gpu) = gpu("material pipeline cache") else {
        return;
    };
    let mut c = context(1.0);
    let id = c.register_material(&flat("return vec4<f32>(p.level);"));
    let data = scene(&mut c, 1.0, |ui| {
        ui.material(CELL, id).show(ui);
    });
    for _ in 0..30 {
        gpu.render(&data, 1.0, 1, Only::Materials, [0.0; 4]);
    }
    assert_eq!(
        gpu.pipelines.counters(),
        (1, 0),
        "one build however many frames"
    );
    // Many distinct materials: at most MAX_PIPELINES stay between frames.
    for n in 0..MAX_PIPELINES + 20 {
        let id = c.register_material(&flat(&format!("return vec4<f32>({n}.0 / 255.0);")));
        let data = scene(&mut c, 1.0, |ui| {
            ui.material(CELL, id).show(ui);
        });
        gpu.render(&data, 1.0, 1, Only::Materials, [0.0; 4]);
    }
    assert!(
        gpu.pipelines.len() <= MAX_PIPELINES,
        "{}",
        gpu.pipelines.len()
    );
    // A device that replaces a lost one compiles from the sources carried by the draw data.
    let Some(mut fresh) = self::gpu("material recreation") else {
        return;
    };
    let pixels = fresh.render(&data, 1.0, 1, Only::Materials, [0.0; 4]);
    assert!(near(at(&pixels, 60, 70), [0, 0, 0, 0], 0));
    assert_eq!(fresh.pipelines.counters(), (1, 0));
}
