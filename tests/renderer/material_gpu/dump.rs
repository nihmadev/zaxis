//! Frames of the pixel-card material written to `target/materials/` for visual inspection:
//! `cargo test --test renderer dump_pixel_cards -- --ignored --nocapture`.

use super::*;

#[path = "../../../examples/pixel_cards/shader.rs"]
mod shader;

use shader::Look;

#[test]
#[ignore = "writes images for visual inspection"]
fn dump_pixel_cards() {
    let Some(mut gpu) = gpu("pixel card dump") else {
        return;
    };
    std::fs::create_dir_all("target/materials").unwrap();
    let tints = [Color::rgb(18, 52, 86), Color::rgb(110, 230, 180)];
    for scale in [1.0_f32, 1.25, 1.5] {
        for (name, pointer, hover, textured) in [
            ("rest", vec2(0.5, 0.5), 0.0, true),
            ("left", vec2(0.1, 0.5), 1.0, true),
            ("right", vec2(0.9, 0.5), 1.0, true),
            ("proc_left", vec2(0.1, 0.5), 1.0, false),
            ("proc_right", vec2(0.9, 0.5), 1.0, false),
        ] {
            let mut c = context(scale);
            c.decode_images_inline(Some(std::time::Duration::from_millis(100)));
            let id = c.register_material(&shader::material());
            let handle = c
                .load_image(ImageSource::from(
                    &include_bytes!("../../../assets/images/landscape.jpg")[..],
                ))
                .unwrap();
            let look = Look {
                pointer,
                hover,
                block: 8.0,
                depth: 14.0,
                seed: 3.0,
                textured,
                drift: false,
                tints,
            };
            let rect = Rect::from_min_size(vec2(16.0, 56.0), vec2(220.0, 100.0));
            let mut data = None;
            for _ in 0..8 {
                data = Some(scene(&mut c, scale, |ui| {
                    let mut paint = ui
                        .material(rect, id)
                        .params(look.params())
                        .corner_radius(14.0);
                    if textured {
                        paint = paint.image(handle).fit(zaxis::ImageFit::Cover);
                    }
                    paint.show(ui);
                }));
            }
            let pixels = gpu.render(
                &data.unwrap(),
                scale,
                1,
                Only::Materials,
                [0.08, 0.08, 0.1, 1.0],
            );
            // The attachment is not sRGB here: encode for the file.
            let encoded: Vec<u8> = pixels
                .iter()
                .flat_map(|p| {
                    let enc = |v: u8| {
                        let l = f32::from(v) / 255.0;
                        let s = if l <= 0.003_130_8 {
                            l * 12.92
                        } else {
                            1.055 * l.powf(1.0 / 2.4) - 0.055
                        };
                        (s * 255.0).round() as u8
                    };
                    [enc(p[0]), enc(p[1]), enc(p[2]), 255]
                })
                .collect();
            image::save_buffer(
                format!("target/materials/pixel_{name}_{scale}.png"),
                &encoded,
                SIZE[0],
                SIZE[1],
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}
