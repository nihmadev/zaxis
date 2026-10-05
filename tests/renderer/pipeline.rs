use wgpu::util::DeviceExt;
use zaxis::renderer::{pipeline, textures, viewport};
use zaxis::{shapes::Mesh, Border, Color, CornerRadius, Rect, Shape, Vec2};

// Read actual GPU pixels, including the single-sample fallback. Kept opt-in so
// CPU-only CI does not need a graphics adapter.
#[test]
#[ignore = "requires a graphics adapter"]
fn gpu_contour_coverage_and_backdrop_alpha() {
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
            height: 128,
            depth_or_array_layers: 1,
        };
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("contour test output"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        // The backdrop shader uses exact framebuffer texel loads, so its input
        // must have the same extent as the attachment.
        let original_pixels = [0, 0, 0, 64].repeat((extent.width * extent.height) as usize);
        let original = textures::create_texture(
            &device,
            &queue,
            &texture_layout,
            &sampler,
            [extent.width, extent.height],
            &original_pixels,
            0,
        );
        let view = target.create_view(&Default::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("contour test readback"),
            size: (extent.width * extent.height * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let rect = Rect::from_min_size(Vec2::new(8.25, 8.25), Vec2::new(112.0, 48.0));
        for samples in [1, 4] {
            if !adapter
                .get_texture_format_features(wgpu::TextureFormat::Rgba8Unorm)
                .flags
                .sample_count_supported(samples)
            {
                continue;
            }
            let multisampled = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("contour test MSAA"),
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let msaa_view = multisampled.create_view(&Default::default());
            for scale in [1.0, 1.25, 1.5, 2.0] {
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
                for backdrop in [false, true] {
                    let mut mesh = Mesh::default();
                    mesh.shape(
                        &Shape::Rect {
                            rect,
                            rounding: CornerRadius::all(24.0),
                            fill: Color::WHITE,
                            border: Border::NONE,
                        },
                        scale,
                    );
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
                    let pipeline = pipeline::create_with_fragment(
                        &device,
                        &viewport_layout,
                        &texture_layout,
                        wgpu::TextureFormat::Rgba8Unorm,
                        samples,
                        if backdrop { "fs_backdrop" } else { "fs_main" },
                    );
                    let mut encoder = device.create_command_encoder(&Default::default());
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: if samples == 1 { &view } else { &msaa_view },
                                resolve_target: (samples > 1).then_some(&view),
                                depth_slice: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                        r: 0.0,
                                        g: 0.0,
                                        b: 0.0,
                                        a: if backdrop { 64.0 / 255.0 } else { 0.0 },
                                    }),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                        pass.set_pipeline(&pipeline);
                        pass.set_bind_group(0, &viewport_group, &[]);
                        pass.set_bind_group(1, &white.bind_group, &[]);
                        if backdrop {
                            pass.set_bind_group(2, &original.bind_group, &[]);
                        }
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
                    let mut edge_pixels = 0;
                    let mut worst_error = 0.0_f32;
                    let mut squared_error = 0.0_f32;
                    let mut compared_pixels = 0;
                    for y in 0..extent.height {
                        for x in 0..extent.width {
                            let pixel = &pixels[((y * extent.width + x) * 4) as usize..][..4];
                            let coverage = pixel[0] as f32 / 255.0;
                            let expected_alpha = if backdrop {
                                64.0 / 255.0 + coverage * (1.0 - 64.0 / 255.0)
                            } else {
                                coverage
                            };
                            assert!((pixel[3] as f32 / 255.0 - expected_alpha).abs() < 0.012);
                            let center = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) / scale;
                            // Distance to the capsule centerline, used only to
                            // select pixels near the ideal silhouette.
                            let nearest = Vec2::new(
                                center.x.clamp(rect.min.x + 24.0, rect.max.x - 24.0),
                                rect.center().y,
                            );
                            if ((center - nearest).length() - 24.0).abs() * scale > 1.5 {
                                continue;
                            }
                            let mut inside = 0;
                            for sy in 0..16 {
                                for sx in 0..16 {
                                    let point = Vec2::new(
                                        x as f32 + (sx as f32 + 0.5) / 16.0,
                                        y as f32 + (sy as f32 + 0.5) / 16.0,
                                    ) / scale;
                                    let nearest = Vec2::new(
                                        point.x.clamp(rect.min.x + 24.0, rect.max.x - 24.0),
                                        rect.center().y,
                                    );
                                    inside += usize::from(
                                        (point - nearest).length_squared() <= 24.0 * 24.0,
                                    );
                                }
                            }
                            let error = (coverage - inside as f32 / 256.0).abs();
                            worst_error = worst_error.max(error);
                            squared_error += error * error;
                            compared_pixels += 1;
                            edge_pixels += usize::from(coverage > 0.02 && coverage < 0.98);
                        }
                    }
                    assert!(edge_pixels > 80, "missing smooth edge coverage");
                    let max_error = if samples == 1 { 0.06 } else { 0.13 };
                    assert!(
                        worst_error < max_error,
                        "{samples}x MSAA, DPI {scale}, backdrop {backdrop}: {worst_error}"
                    );
                    let rms = (squared_error / compared_pixels as f32).sqrt();
                    println!("{samples}x MSAA, DPI {scale}, backdrop {backdrop}: max coverage error {worst_error:.3}, RMS {rms:.3}");
                    drop(pixels);
                    readback.unmap();
                }
            }
        }
    });
}
