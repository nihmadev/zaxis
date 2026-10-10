//! `--smoke-test`: both modes off screen on a device with no window, pixels read back.

use crate::{
    frame,
    gpu::Gpu,
    interface::{Interface, Mode},
    model::Model,
    scene::Scene,
    targets::{Targets, DEPTH},
};
use std::error::Error;
use zaxis::{winit::dpi::PhysicalSize, EmbedOptions};

const SIZE: [u32; 2] = [640, 400];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

pub fn run() -> Result<(), Box<dyn Error>> {
    let (gpu, _) = Gpu::new(None)?;
    let view_format = FORMAT.add_srgb_suffix();
    let samples = gpu.samples(view_format, DEPTH);
    let scene = Scene::new(&gpu, view_format, samples);
    let targets = Targets::new(&gpu, SIZE, samples, view_format);
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("host target"),
        size: wgpu::Extent3d {
            width: SIZE[0],
            height: SIZE[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[view_format],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        format: Some(view_format),
        ..Default::default()
    });
    let options = EmbedOptions::new(FORMAT)
        .sample_count(samples)
        .depth_format(DEPTH);
    let mut images = Vec::new();
    for mode in [Mode::Pass, Mode::Texture] {
        let mut interface = Interface::new(&gpu, options, mode)?;
        // Layout settles over a few passes; after that an unchanged interface uploads nothing.
        frames(&gpu, &scene, &targets, &mut interface, &view, 4)?;
        let settled = interface.renderer.stats();
        frames(&gpu, &scene, &targets, &mut interface, &view, 3)?;
        let after = interface.renderer.stats();
        assert_eq!(after.geometry_uploads, settled.geometry_uploads);
        assert_eq!(after.texture_uploads, settled.texture_uploads);
        assert_eq!(after.presented_frames, settled.presented_frames + 3);
        assert!(after.draw_calls > settled.draw_calls);
        // Input forwarded from the host reaches the model through the interface.
        click(&mut interface, 48.0, 82.0);
        frames(&gpu, &scene, &targets, &mut interface, &view, 2)?;
        assert!(!interface.model.rotate, "the checkbox did not toggle");
        let speed = interface.model.speed;
        click(&mut interface, 230.0, 141.0);
        frames(&gpu, &scene, &targets, &mut interface, &view, 2)?;
        assert!(interface.model.speed > speed, "the slider did not move");
        interface.model.rotate = true;
        interface.model.speed = speed;
        images.push(read(&gpu, &texture));
    }
    let scene_only = {
        scene.update(&gpu, &Model::default(), SIZE);
        frame::draw(&gpu, &scene, &targets, None, &view)?;
        read(&gpu, &texture)
    };
    let blurred = images[0]
        .iter()
        .zip(&images[1])
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        blurred > 1_000,
        "glass must differ between the modes: {blurred} pixels"
    );
    std::fs::create_dir_all("target/embed")?;
    for (name, image) in ["pass", "texture"].iter().zip(&images) {
        let bytes: Vec<u8> = image
            .iter()
            .flat_map(|[b, g, r, a]| [*r, *g, *b, *a])
            .collect();
        image::save_buffer(
            format!("target/embed/{name}.png"),
            &bytes,
            SIZE[0],
            SIZE[1],
            image::ExtendedColorType::Rgba8,
        )?;
        let differing = image
            .iter()
            .zip(&scene_only)
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            differing > 10_000,
            "{name}: the interface changed {differing} pixels"
        );
        println!("embed smoke: {name} mode draws {differing} pixels over the scene");
    }
    Ok(())
}

/// A left click at logical `(x, y)` as the host's window would report it.
fn click(interface: &mut Interface, x: f64, y: f64) {
    use zaxis::winit::{
        dpi::PhysicalPosition,
        event::{DeviceId, ElementState, MouseButton, WindowEvent},
    };
    let id = DeviceId::dummy();
    interface.input(&WindowEvent::CursorMoved {
        device_id: id,
        position: PhysicalPosition::new(x, y),
    });
    for state in [ElementState::Pressed, ElementState::Released] {
        interface.input(&WindowEvent::MouseInput {
            device_id: id,
            state,
            button: MouseButton::Left,
        });
    }
}

fn frames(
    gpu: &Gpu,
    scene: &Scene,
    targets: &Targets,
    interface: &mut Interface,
    view: &wgpu::TextureView,
    count: usize,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..count {
        interface.build(PhysicalSize::new(SIZE[0], SIZE[1]), 1.0);
        scene.update(gpu, &interface.model, SIZE);
        frame::draw(gpu, scene, targets, Some(&mut *interface), view)?;
    }
    Ok(())
}

fn read(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<[u8; 4]> {
    let row = (SIZE[0] * 4).next_multiple_of(256);
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * SIZE[1]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(SIZE[1]),
            },
        },
        texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap()
        });
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let data = buffer.slice(..).get_mapped_range().unwrap();
    (0..SIZE[1])
        .flat_map(|y| {
            let line = &data[(y * row) as usize..][..(SIZE[0] * 4) as usize];
            line.as_chunks::<4>().0.to_vec()
        })
        .collect()
}
