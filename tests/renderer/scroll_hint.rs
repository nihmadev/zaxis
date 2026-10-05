use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use zaxis::renderer::{pipeline, textures, viewport};
use zaxis::{shapes::Mesh, Color, Rect, TextureId, Vec2};

#[test]
#[ignore = "requires a graphics adapter"]
fn gpu_scroll_hint_is_dim_neutral_smooth_and_clipped_at_multiple_dpi() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let (viewport_layout, uniform, viewport_group) = viewport::create_bindings(&device);
        let (texture_layout, sampler) = textures::create_bindings(&device);
        let white = textures::create_texture(
            &device,
            &queue,
            &texture_layout,
            &sampler,
            [1, 1],
            &[255; 4],
            0,
        );
        let extent = wgpu::Extent3d {
            width: 256,
            height: 64,
            depth_or_array_layers: 1,
        };
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scroll hint pixels"),
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
            size: (extent.width * extent.height * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        for samples in [1, 4] {
            if !adapter
                .get_texture_format_features(format)
                .flags
                .sample_count_supported(samples)
            {
                continue;
            }
            let multisampled = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let msaa_view = multisampled.create_view(&Default::default());
            let pipeline = pipeline::create_with_fragment(
                &device,
                &viewport_layout,
                &texture_layout,
                format,
                samples,
                "fs_scroll_hint",
            );
            for scale in [1.0, 1.25, 2.0] {
                queue.write_buffer(
                    &uniform,
                    0,
                    bytemuck::cast_slice(&[
                        extent.width as f32 / scale,
                        extent.height as f32 / scale,
                        0.0,
                        0.0,
                    ]),
                );
                for axis in [0, 1] {
                    let rect = Rect::from_min_size(
                        Vec2::new(8.0, 4.0),
                        Vec2::new(if axis == 0 { 26.0 } else { 96.0 }, 26.0),
                    );
                    let mut clip = rect;
                    clip.min[1 - axis] += 6.0;
                    clip.max[1 - axis] -= 6.0;
                    let scissor = viewport::scissor(
                        clip,
                        scale,
                        PhysicalSize::new(extent.width, extent.height),
                    )
                    .unwrap();
                    let mut mesh = Mesh::default();
                    mesh.quad(
                        rect,
                        Rect::from_min_size(Vec2::ZERO, Vec2::ONE),
                        Color::rgba(0, 0, 0, 16).linear(),
                        TextureId::WHITE,
                    );
                    if axis == 0 {
                        for vertex in &mut mesh.vertices {
                            vertex.uv.swap(0, 1);
                        }
                    }
                    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&mesh.vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                    let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&mesh.indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });
                    let mut encoder = device.create_command_encoder(&Default::default());
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: if samples == 1 { &view } else { &msaa_view },
                                depth_slice: None,
                                resolve_target: (samples > 1).then_some(&view),
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                        r: 0.5,
                                        g: 0.5,
                                        b: 0.5,
                                        a: 1.0,
                                    }),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                        pass.set_pipeline(&pipeline);
                        pass.set_bind_group(0, &viewport_group, &[]);
                        pass.set_bind_group(1, &white.bind_group, &[]);
                        pass.set_scissor_rect(scissor[0], scissor[1], scissor[2], scissor[3]);
                        pass.set_vertex_buffer(0, vertices.slice(..));
                        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
                    }
                    encoder.copy_texture_to_buffer(
                        target.as_image_copy(),
                        wgpu::TexelCopyBufferInfo {
                            buffer: &readback,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(extent.width * 4),
                                rows_per_image: Some(extent.height),
                            },
                        },
                        extent,
                    );
                    queue.submit([encoder.finish()]);
                    let (tx, rx) = std::sync::mpsc::channel();
                    readback
                        .slice(..)
                        .map_async(wgpu::MapMode::Read, move |result| {
                            tx.send(result).unwrap();
                        });
                    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                    rx.recv().unwrap().unwrap();
                    let pixels = readback.slice(..).get_mapped_range().unwrap();
                    let mut darkened = 0;
                    for y in 0..extent.height {
                        for x in 0..extent.width {
                            let pixel = &pixels[((y * extent.width + x) * 4) as usize..][..4];
                            assert_eq!(pixel[0], pixel[1]);
                            assert_eq!(pixel[1], pixel[2]);
                            assert_eq!(pixel[3], 255);
                            assert!(
                                (119..=128).contains(&pixel[0]),
                                "hint brightens or is too strong: {pixel:?}"
                            );
                            let inside = x >= scissor[0]
                                && x < scissor[0] + scissor[2]
                                && y >= scissor[1]
                                && y < scissor[1] + scissor[3];
                            if !inside {
                                assert_eq!(pixel[0], 128, "hint escaped scissor");
                            }
                            darkened += usize::from(pixel[0] < 127);
                        }
                    }
                    assert!(darkened > 20);
                    let cross = (clip.center()[1 - axis] * scale) as u32;
                    let mut last = 128;
                    for step in 0..(26.0 * scale) as u32 {
                        let pos = (rect.min[axis] * scale) as u32 + step;
                        let (x, y) = if axis == 0 {
                            (pos, cross)
                        } else {
                            (cross, pos)
                        };
                        let value = pixels[((y * extent.width + x) * 4) as usize];
                        assert!(value <= last, "fade must darken monotonically");
                        assert!(last - value <= 1, "hard band in fade");
                        last = value;
                    }
                    assert!((119..=121).contains(&last));
                    drop(pixels);
                    readback.unmap();
                }
            }
        }
    });
}
