use super::{pipeline, textures, viewport};
use crate::{vec2, Context, DragValue, NumberInput, Rect, Response, Slider, TextureId, Window};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

/// Exercise real glyph atlases, shader blending and scissors, not only UI layout.
#[test]
#[ignore = "requires a graphics adapter"]
fn gpu_numeric_fields_fit_at_multiple_dpi() {
    pollster::block_on(async {
        let adapter = wgpu::Instance::default()
            .request_adapter(&Default::default())
            .await
            .unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let (viewport_layout, uniform, viewport_group) = viewport::create_bindings(&device);
        let (texture_layout, sampler) = textures::create_bindings(&device);
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let pipeline = pipeline::create(&device, &viewport_layout, &texture_layout, format, 1);
        for scale in [1.0, 1.25, 2.0] {
            let physical = PhysicalSize::new((512.0 * scale) as u32, (384.0 * scale) as u32);
            let mut context = Context::new();
            context.set_viewport(physical, f64::from(scale));
            let mut style = context.style().clone();
            style.motion.reduced_motion = true;
            style.text_edit_blink_interval = std::time::Duration::ZERO;
            context.set_style(style);
            let mut gain = 0.65_f32;
            let mut offset = -12.5_f64;
            let mut count = 9_007_199_254_740_993_u64;
            let mut invalid = 4.0_f64;
            let mut responses: Vec<Response> = Vec::new();
            for frame in 0..2 {
                context.run(|context| {
                    Window::new("NumberInput & DragValue")
                        .default_position(vec2(24.0, 24.0))
                        .default_size(vec2(464.0, 330.0))
                        .show(context, |ui| {
                            responses.clear();
                            ui.muted("Gain");
                            ui.horizontal_aligned(crate::Align::Center, |ui| {
                                ui.add(Slider::new(&mut gain, 0.0..=1.0).width(260.0));
                                responses.push(
                                    ui.add(
                                        NumberInput::new(&mut gain)
                                            .precision(2)
                                            .suffix(" ×")
                                            .width(120.0),
                                    ),
                                );
                            });
                            ui.muted("Offset · drag or click to type");
                            responses.push(
                                ui.add(
                                    DragValue::new(&mut offset)
                                        .precision(2)
                                        .suffix(" px")
                                        .width(160.0),
                                ),
                            );
                            responses.push(ui.add(NumberInput::new(&mut count).width(260.0)));
                            responses.push(
                                ui.add(
                                    NumberInput::new(&mut offset)
                                        .prefix("$ ")
                                        .suffix(" kg")
                                        .enabled(false)
                                        .width(180.0),
                                ),
                            );
                            responses.push(
                                ui.add(
                                    NumberInput::new(&mut invalid)
                                        .id_source("invalid")
                                        .width(160.0),
                                ),
                            );
                        });
                });
                if frame == 0 {
                    context.request_focus(responses[4].id);

                    // Focus selects the exact number; replace it with an incomplete draft.
                    context.on_text_event("-");
                }
            }
            let data = context.draw_data();
            let mut atlas = HashMap::new();
            atlas.insert(
                TextureId::WHITE,
                textures::create_texture(
                    &device,
                    &queue,
                    &texture_layout,
                    &sampler,
                    [1, 1],
                    &[255; 4],
                    0,
                ),
            );
            for image in &data.textures {
                atlas.insert(
                    image.id,
                    textures::create_texture(
                        &device,
                        &queue,
                        &texture_layout,
                        &sampler,
                        image.size,
                        &image.pixels,
                        image.revision,
                    ),
                );
            }
            queue.write_buffer(
                &uniform,
                0,
                bytemuck::cast_slice(&[512.0_f32, 384.0, 0.0, 0.0]),
            );
            let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&data.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&data.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            let extent = wgpu::Extent3d {
                width: physical.width,
                height: physical.height,
                depth_or_array_layers: 1,
            };
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = target.create_view(&Default::default());
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: u64::from(physical.width * physical.height * 4),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.015,
                                g: 0.015,
                                b: 0.015,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &viewport_group, &[]);
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                for command in &data.commands {
                    let Some([x, y, w, h]) = viewport::scissor(command.clip_rect, scale, physical)
                    else {
                        continue;
                    };
                    pass.set_scissor_rect(x, y, w, h);
                    pass.set_bind_group(1, &atlas[&command.texture].bind_group, &[]);
                    pass.draw_indexed(command.indices.clone(), 0, 0..1);
                }
            }
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(physical.width * 4),
                        rows_per_image: Some(physical.height),
                    },
                },
                extent,
            );
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                tx.send(r).unwrap();
            });
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            rx.recv().unwrap().unwrap();
            let pixels = readback.slice(..).get_mapped_range().unwrap();
            for response in &responses[..4] {
                assert_eq!(response.rect.size().y, 28.0);
                check_band(&pixels, physical.width, response.rect, scale);
            }
            if scale == 1.25 {
                image::save_buffer(
                    "target/numbers.png",
                    &pixels,
                    physical.width,
                    physical.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            drop(pixels);
            readback.unmap();
        }
    });
}
fn check_band(pixels: &[u8], width: u32, rect: Rect, scale: f32) {
    let mut first = u32::MAX;
    let mut last = 0;
    for y in (rect.min.y * scale).ceil() as u32..(rect.max.y * scale).floor() as u32 {
        for x in
            ((rect.min.x + 6.0) * scale).ceil() as u32..((rect.max.x - 28.0) * scale).floor() as u32
        {
            if pixels[((y * width + x) * 4) as usize] > 105 {
                first = first.min(y);
                last = last.max(y);
            }
        }
    }
    assert!(first < last, "missing text in {rect:?}");
    assert!(
        first as f32 >= rect.min.y * scale + 1.0 && (last as f32) < rect.max.y * scale - 1.0,
        "clipped text in {rect:?}: {first}..{last} at {scale}"
    );
    assert!(
        ((first + last) as f32 * 0.5 - rect.center().y * scale).abs() <= 2.0 * scale,
        "uncentered glyph pixels in {rect:?}: {first}..{last} at {scale}"
    );
}
