use super::{pipeline, textures, viewport};
use crate::{shapes::Mesh, Color, CornerRadius, Rect, Vec2};
use wgpu::util::DeviceExt;

#[test]
fn image_gpu_alpha_tint_rounding_and_filter_readback() {
    if std::env::var_os("ZAXIS_SKIP_GPU_TESTS").is_some() {
        eprintln!("SKIPPED image GPU readback: ZAXIS_SKIP_GPU_TESTS");
        return;
    }
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let Ok(adapter) = instance.request_adapter(&Default::default()).await else {
            eprintln!("SKIPPED image GPU readback: no adapter");
            return;
        };
        let Ok((device, queue)) = adapter.request_device(&Default::default()).await else {
            eprintln!("SKIPPED image GPU readback: no device");
            return;
        };
        let (viewport_layout, uniform, viewport_group) = viewport::create_bindings(&device);
        let (layout, linear) = textures::create_bindings(&device);
        let nearest = textures::nearest_sampler(&device);
        queue.write_buffer(
            &uniform,
            0,
            bytemuck::cast_slice(&[128.0f32, 128.0, 0.0, 0.0]),
        );
        let translucent = textures::create_texture(
            &device,
            &queue,
            &layout,
            &linear,
            [1, 1],
            &[255, 0, 0, 128],
            1,
        );
        let edge = textures::create_texture(
            &device,
            &queue,
            &layout,
            &linear,
            [2, 1],
            &[255, 0, 0, 255, 0, 0, 0, 0],
            1,
        );
        let edge_nearest = textures::create_texture(
            &device,
            &queue,
            &layout,
            &nearest,
            [2, 1],
            &[255, 0, 0, 255, 0, 0, 0, 0],
            1,
        );
        let opaque = textures::create_texture(
            &device,
            &queue,
            &layout,
            &linear,
            [1, 1],
            &[255, 0, 0, 255],
            1,
        );
        let extent = wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        };
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("image readback"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 128 * 128 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let uv = Rect::from_min_size(Vec2::ZERO, Vec2::ONE);
        for samples in [1, 4] {
            if !adapter
                .get_texture_format_features(wgpu::TextureFormat::Rgba8Unorm)
                .flags
                .sample_count_supported(samples)
            {
                continue;
            }
            let msaa = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let msaa_view = msaa.create_view(&Default::default());
            let smooth = pipeline::create_with_fragment(
                &device,
                &viewport_layout,
                &layout,
                wgpu::TextureFormat::Rgba8Unorm,
                samples,
                "fs_image_linear",
            );
            let pixel = pipeline::create(
                &device,
                &viewport_layout,
                &layout,
                wgpu::TextureFormat::Rgba8Unorm,
                samples,
            );
            let mut mesh = Mesh::default();
            let mut ranges = Vec::new();
            for (position, size, tint, opacity, rounding) in [
                (
                    Vec2::new(4.0, 4.0),
                    Vec2::new(48.0, 48.0),
                    Color::rgba(255, 255, 255, 128),
                    0.5,
                    0.0,
                ),
                (
                    Vec2::new(64.0, 4.0),
                    Vec2::new(48.0, 48.0),
                    Color::WHITE,
                    1.0,
                    0.0,
                ),
                (
                    Vec2::new(4.0, 64.0),
                    Vec2::new(48.0, 48.0),
                    Color::WHITE,
                    1.0,
                    20.0,
                ),
                (
                    Vec2::new(64.0, 64.0),
                    Vec2::new(48.0, 48.0),
                    Color::WHITE,
                    1.0,
                    0.0,
                ),
            ] {
                let start = mesh.indices.len() as u32;
                mesh.image(
                    Rect::from_min_size(position, size),
                    uv,
                    CornerRadius::all(rounding),
                    tint,
                    opacity,
                    crate::TextureId(1),
                    1.0,
                );
                ranges.push(start..mesh.indices.len() as u32);
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
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_bind_group(0, &viewport_group, &[]);
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                for (i, texture) in [&translucent, &edge, &opaque, &edge_nearest]
                    .into_iter()
                    .enumerate()
                {
                    pass.set_pipeline(if i == 3 { &pixel } else { &smooth });
                    pass.set_bind_group(1, &texture.bind_group, &[]);
                    pass.draw_indexed(ranges[i].clone(), 0, 0..1);
                }
            }
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(512),
                        rows_per_image: Some(128),
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
            let bytes = readback.slice(..).get_mapped_range().unwrap();
            let at = |x, y| &bytes[((y * 128 + x) * 4) as usize..][..4];
            assert!((at(20, 20)[0] as i32 - 32).abs() <= 2);
            assert_eq!(at(20, 20)[0], at(20, 20)[3]);
            let middle = at(87, 20);
            assert!(middle[0] > 125 && middle[0] < 140);
            assert!(
                (middle[0] as i32 - middle[3] as i32).abs() <= 1,
                "transparent edge darkened"
            );
            assert_eq!(at(5, 65)[3], 0);
            assert_eq!(at(28, 88)[3], 255);
            assert_eq!(at(86, 88)[3], 255);
            assert_eq!(at(89, 88)[3], 0);
            let coverage: Vec<_> = bytes
                .chunks_exact(4)
                .filter(|p| p[3] > 0 && p[3] < 255)
                .collect();
            assert!(!coverage.is_empty());
            assert!(bytes.chunks_exact(4).all(|p| p[0] == p[3]));
            drop(bytes);
            readback.unmap();
        }
        eprintln!("image GPU readback passed: {:?}", adapter.get_info());
    });
}
