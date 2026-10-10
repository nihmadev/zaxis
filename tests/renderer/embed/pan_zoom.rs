//! Read back the actual public renderer and compare the colored region with input ownership.
use super::harness::{host, pixel, Spec};
use zaxis::testing::Inspect;
use zaxis::winit::{
    dpi::PhysicalSize,
    event::{ElementState, MouseButton},
};
use zaxis::{
    vec2, Color, Context, EmbedViewport, EmbeddedRenderer, InputEvent, Padding, PanZoom,
    PanZoomState, Rect, Root, Sense, Shape,
};

#[test]
fn magnified_standard_controls_use_display_resolution_glyphs_and_antialiased_edges() {
    use zaxis::{Slider, TextEdit};
    let Some(host) = host("PanZoom resolution") else {
        return;
    };
    let size = [1200, 900];
    let spec = Spec::new(wgpu::TextureFormat::Rgba8UnormSrgb, size);
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(size[0], size[1]), 1.0);
    let mut camera = PanZoomState::default();
    let mut title = "Smooth Zoom".to_owned();
    let mut value = 0.5;
    for scale in [0.1, 0.5, 1.0, 2.0, 4.0, 8.0] {
        camera.scale = scale;
        camera.translation = vec2(35.0, 35.0);
        c.run(|c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                PanZoom::new("scene", vec2(1200.0, 900.0)).show(ui, &mut camera, |ui, _| {
                    ui.at(
                        "panel",
                        Rect::from_min_size(zaxis::Vec2::ZERO, vec2(130.0, 110.0)),
                        |ui| {
                            ui.add(TextEdit::new(&mut title).width(120.0));
                            ui.button("Zoom");
                            ui.add(Slider::new(&mut value, 0.0..=1.0).width(120.0));
                        },
                    );
                });
            })
        });
        let data = c.draw_data();
        let mut checked = 0;
        for cached in c.probe().cache.values() {
            if !cached.paint.iter().any(|p| {
                matches!(
                    p,
                    zaxis::testing::Paint::Text { .. }
                        | zaxis::testing::Paint::Paragraph { .. }
                        | zaxis::testing::Paint::Rich { .. }
                )
            }) {
                continue;
            }
            for (range, texture_id) in &cached.mesh.batches {
                let Some(texture) = data.textures.iter().find(|t| t.id == *texture_id) else {
                    continue;
                };
                for indices in
                    cached.mesh.indices[range.start as usize..range.end as usize].chunks_exact(6)
                {
                    let quad = [
                        cached.mesh.vertices[indices[0] as usize],
                        cached.mesh.vertices[indices[1] as usize],
                    ];
                    let width = (quad[1].position[0] - quad[0].position[0]).abs();
                    let texels = (quad[1].uv[0] - quad[0].uv[0]).abs() * texture.size[0] as f32;
                    if width > 0.0 {
                        assert!(texels / (width * scale) >= 0.99, "glyph stretched below displayed resolution at {scale}: {texels} texels for {}", width * scale);
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 5);
        let mut renderer = EmbeddedRenderer::new_with_adapter(
            host.device.clone(),
            host.queue.clone(),
            &host.adapter,
            spec.options(),
        )
        .unwrap();
        renderer.prepare(data, EmbedViewport::whole(size)).unwrap();
        let pixels = host.in_pass(&spec, false, |pass| {
            renderer.record(pass).unwrap();
        });
        if scale == 8.0 {
            std::fs::create_dir_all("target/pan-zoom-validation").unwrap();
            let rgba: Vec<u8> = pixels.into_iter().flatten().collect();
            image::save_buffer(
                "target/pan-zoom-validation/controls-zoom-8.png",
                &rgba,
                size[0],
                size[1],
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}

#[test]
fn pan_zoom_paint_hit_clip_and_text_share_displayed_geometry_at_multiple_dpi() {
    let Some(host) = host("PanZoom readback") else {
        return;
    };
    for dpi in [1.0, 1.25, 1.5, 2.0] {
        for scale in [0.5, 1.0, 2.0, 8.0] {
            let size = [(512.0 * dpi) as u32, (384.0 * dpi) as u32];
            let spec = Spec::new(wgpu::TextureFormat::Rgba8UnormSrgb, size);
            let mut c = Context::new();
            c.set_viewport(PhysicalSize::new(size[0], size[1]), dpi as f64);
            let mut camera = PanZoomState {
                scale,
                translation: vec2(100.0, 90.0),
            };
            let mut result = None;
            let draw =
                |c: &mut Context,
                 camera: &mut PanZoomState,
                 result: &mut Option<zaxis::PanZoomOutput<zaxis::Response>>| {
                    c.run(|c| {
                        Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                            ui.at(
                                "viewport",
                                Rect::from_min_size(vec2(30.0, 25.0), vec2(300.0, 260.0)),
                                |ui| {
                                    *result = Some(PanZoom::new("scene", vec2(300.0, 260.0)).show(
                                        ui,
                                        camera,
                                        |ui, _| {
                                            let rect = Rect::from_min_size(
                                                vec2(-10.0, -10.0),
                                                vec2(20.0, 20.0),
                                            );
                                            let response = ui.interact(rect, "red", Sense::CLICK);
                                            ui.paint(Shape::rect(rect, Color::rgb(240, 25, 30)));
                                            ui.at(
                                                "text",
                                                Rect::from_min_size(
                                                    vec2(-10.0, 14.0),
                                                    vec2(100.0, 40.0),
                                                ),
                                                |ui| {
                                                    ui.label("Zoom");
                                                },
                                            );
                                            response
                                        },
                                    ));
                                },
                            );
                        })
                    });
                };
            draw(&mut c, &mut camera, &mut result);
            let mut renderer = EmbeddedRenderer::new_with_adapter(
                host.device.clone(),
                host.queue.clone(),
                &host.adapter,
                spec.options(),
            )
            .unwrap();
            renderer
                .prepare(c.draw_data(), EmbedViewport::whole(size))
                .unwrap();
            let pixels = host.in_pass(&spec, false, |pass| {
                renderer.record(pass).unwrap();
            });
            let out = result.as_ref().unwrap();
            let rect = c.visual_rect(out.inner.id, out.inner.rect);
            let expected = out.displayed_viewport(&c).intersect(rect);
            for y in 0..size[1] {
                for x in 0..size[0] {
                    let rgba = pixel(&pixels, size[0], x, y);
                    let p = vec2(x as f32 + 0.5, y as f32 + 0.5) / dpi;
                    let red = rgba[0] > 220 && rgba[1] < 50 && rgba[2] < 55;
                    if red {
                        assert!(
                            expected.contains(p),
                            "red pixel outside hit/clip at DPI {dpi}, scale {scale}"
                        );
                        assert_eq!(c.hit_test(p).unwrap().id, out.inner.id);
                    }
                }
            }
            let point = expected.center();
            assert_eq!(c.hit_test(point).unwrap().id, out.inner.id);
            let rgba = pixel(
                &pixels,
                size[0],
                (point.x * dpi) as u32,
                (point.y * dpi) as u32,
            );
            assert!(rgba[0] > 220 && rgba[1] < 50);
            let p = point * dpi;
            c.on_input(InputEvent::PointerMoved {
                x: p.x as f64,
                y: p.y as f64,
            });
            assert!(
                c.on_input(InputEvent::Button {
                    button: MouseButton::Left,
                    state: ElementState::Pressed
                })
                .consumed
            );
            assert!(
                c.on_input(InputEvent::Button {
                    button: MouseButton::Left,
                    state: ElementState::Released
                })
                .consumed
            );
            draw(&mut c, &mut camera, &mut result);
            assert!(result.as_ref().unwrap().inner.clicked());
            if dpi == 1.5 && scale == 2.0 {
                let rgba: Vec<u8> = pixels.into_iter().flatten().collect();
                std::fs::create_dir_all("target/pan-zoom-validation").unwrap();
                image::save_buffer(
                    "target/pan-zoom-validation/readback.png",
                    &rgba,
                    size[0],
                    size[1],
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
