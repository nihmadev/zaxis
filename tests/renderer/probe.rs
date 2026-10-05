//! Dumps zoomable frames of focused/hovered controls to target/probe for visual edge checks:
//! cargo test --lib gpu_dump_control_frames -- --ignored --nocapture | grep -E "^(edit|combo)_" > target/probe/list.txt
//! python scripts/probe_crop.py   (writes target/probe/sheet.png)
use std::{collections::HashMap, io::Write};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, WindowEvent},
};
use zaxis::renderer::{pipeline, textures, viewport};
use zaxis::{vec2, ComboBox, ComboBoxOption, Context, Response, TextEdit, TextureId, Window};

#[test]
#[ignore = "requires a graphics adapter"]
fn gpu_dump_control_frames() {
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
        for (variant, scale) in [
            ("edit", 1.25f32),
            ("edit", 1.5),
            ("combo_focus", 1.25),
            ("combo_focus", 1.5),
            ("combo_hover", 1.25),
            ("combo_hover", 1.5),
            ("combo_open", 1.25),
            ("combo_open", 1.5),
        ] {
            let physical = PhysicalSize::new((512.0 * scale) as u32, (384.0 * scale) as u32);
            let mut context = Context::new();
            context.set_viewport(physical, f64::from(scale));
            let mut style = context.style().clone();
            style.motion.reduced_motion = true;
            style.text_edit_blink_interval = std::time::Duration::ZERO;
            context.set_style(style);
            let options: Vec<_> = ["Work", "Personal", "Archive"]
                .iter()
                .enumerate()
                .map(|(i, n)| ComboBoxOption::new(i as i32, i as i32, n.to_string()))
                .collect();
            let mut selected = Some(0);
            let mut text = String::from("Quarterly report");
            let mut response: Option<Response> = None;
            for frame in 0..4 {
                if frame == 2 && variant != "combo_hover" && variant != "combo_open" {
                    context.request_focus(response.unwrap().id);
                }
                if frame == 2 && variant == "combo_hover" {
                    let r = response.unwrap().rect.center();
                    context.on_window_event(&WindowEvent::CursorMoved {
                        device_id: DeviceId::dummy(),
                        position: PhysicalPosition::new(
                            f64::from(r.x * scale),
                            f64::from(r.y * scale),
                        ),
                    });
                }
                context.run(|context| {
                    Window::new("W")
                        .default_position(vec2(24.0, 24.37))
                        .default_size(vec2(410.0, 330.0))
                        .show(context, |ui| {
                            response = Some(if variant == "edit" {
                                ui.add(TextEdit::new(&mut text).id_source("e"))
                            } else {
                                ui.add(
                                    ComboBox::new(&mut selected, &options)
                                        .label("Category")
                                        .id_source("c")
                                        .default_open(variant == "combo_open")
                                        .width(300.0),
                                )
                            });
                        });
                });
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
            let r = response.unwrap().rect;
            std::fs::create_dir_all("target/probe").unwrap();
            let mut file =
                std::fs::File::create(format!("target/probe/{variant}_{scale}.ppm")).unwrap();
            write!(
                file,
                "P6
{} {}
255
",
                physical.width, physical.height
            )
            .unwrap();
            let rgb: Vec<_> = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| p[..3].iter().copied())
                .collect();
            file.write_all(&rgb).unwrap();
            println!(
                "{variant}_{scale} {} {} {} {}",
                r.min.x * scale,
                r.min.y * scale,
                r.max.x * scale,
                r.max.y * scale
            );
            drop(pixels);
            readback.unmap();
        }
    });
}
