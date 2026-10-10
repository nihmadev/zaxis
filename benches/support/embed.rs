//! The embedded renderer on the scenes the window renderer is measured on: recorded into a
//! host pass, rendered into the host's texture, and split over four viewports of one device.
//! The host here is headless, so a row's render time is `prepare` or `render_to`, encoding and
//! submission, not presentation; compare it with the `GPU` rows of the same case and size.
//! Assertions on pixels and counters run outside the timed section.
use crate::{
    report::{Counters, FailedCase, Geometry, Report, ResultRow, Samples},
    scene::{Case, Scene},
    BenchResult, Options,
};
use std::time::Duration;
use zaxis::winit::dpi::PhysicalSize;
use zaxis::{EmbedLoad, EmbedOptions, EmbedViewport, EmbeddedRenderer, Instant, PhysicalRect};

/// Cases measured through the embedded renderer: an idle scene, a large static scene, text,
/// an interactive scene that changes every frame, and a backdrop effect.
const CASES: [Case; 5] = [
    Case::Cached,
    Case::Shapes,
    Case::Text,
    Case::Slider,
    Case::Blur24,
];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const CLEAR: wgpu::Color = wgpu::Color {
    r: 0.04,
    g: 0.06,
    b: 0.12,
    a: 1.0,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// `prepare` and `record` into a pass of one sample.
    Pass,
    /// The same into a pass with 4x MSAA and a depth attachment.
    PassMsaa,
    /// `render_to` the host's texture.
    Texture,
    /// Four siblings, each in a quarter of the target, recorded into one pass.
    Viewports,
}

impl Mode {
    const ALL: [Self; 4] = [Self::Pass, Self::PassMsaa, Self::Texture, Self::Viewports];
    fn name(self) -> &'static str {
        match self {
            Self::Pass => "embed_pass",
            Self::PassMsaa => "embed_pass_msaa4_depth",
            Self::Texture => "embed_texture",
            Self::Viewports => "embed_4_viewports",
        }
    }
}

struct Host {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    msaa: Option<wgpu::TextureView>,
    depth: Option<wgpu::TextureView>,
}

pub(super) fn run(options: &Options, cases: &[Case], report: &mut Report) -> BenchResult<()> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let host = Host {
        adapter,
        device,
        queue,
    };
    let msaa = host
        .adapter
        .get_texture_format_features(FORMAT.add_srgb_suffix())
        .flags
        .sample_count_supported(4);
    for &count in &options.sizes {
        for case in CASES.into_iter().filter(|c| cases.contains(c)) {
            for mode in Mode::ALL {
                if mode == Mode::PassMsaa && !msaa {
                    continue;
                }
                report.failed_case =
                    Some(FailedCase::new("Embed", case.name(), count, mode.name()));
                let row = measure(&host, options, case, count, mode)?;
                row.print();
                report.results.push(row);
                report.failed_case = None;
            }
        }
    }
    Ok(())
}

fn target(host: &Host, size: [u32; 2], samples: u32, depth: bool) -> Target {
    let view_format = FORMAT.add_srgb_suffix();
    let make = |format, usage, count| {
        host.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bench target"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: if format == FORMAT {
                std::slice::from_ref(&view_format)
            } else {
                &[]
            },
        })
    };
    let color = wgpu::TextureViewDescriptor {
        format: Some(view_format),
        ..Default::default()
    };
    let texture = make(
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        1,
    );
    Target {
        view: texture.create_view(&color),
        msaa: (samples > 1).then(|| {
            make(FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT, samples).create_view(&color)
        }),
        depth: depth.then(|| {
            make(DEPTH, wgpu::TextureUsages::RENDER_ATTACHMENT, samples)
                .create_view(&Default::default())
        }),
        texture,
    }
}

fn pass<'a>(encoder: &'a mut wgpu::CommandEncoder, target: &'a Target) -> wgpu::RenderPass<'a> {
    let (view, resolve) = match &target.msaa {
        Some(msaa) => (msaa, Some(&target.view)),
        None => (&target.view, None),
    };
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("bench host pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: resolve,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(CLEAR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: target.depth.as_ref().map(|view| {
            wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }
        }),
        ..Default::default()
    })
}

fn measure(
    host: &Host,
    options: &Options,
    case: Case,
    count: usize,
    mode: Mode,
) -> BenchResult<ResultRow> {
    let size = options.size;
    let whole = [size.width, size.height];
    // Four viewports: the scene is built for a quarter of the target.
    let (scene_size, quarter) = match mode {
        Mode::Viewports => (PhysicalSize::new(size.width / 2, size.height / 2), true),
        _ => (size, false),
    };
    let samples = if mode == Mode::PassMsaa { 4 } else { 1 };
    let embed_options = if mode == Mode::PassMsaa {
        EmbedOptions::new(FORMAT)
            .sample_count(samples)
            .depth_format(DEPTH)
    } else {
        EmbedOptions::new(FORMAT)
    };
    let new = || {
        EmbeddedRenderer::new_with_adapter(
            host.device.clone(),
            host.queue.clone(),
            &host.adapter,
            embed_options,
        )
    };
    let mut renderers = vec![new()?];
    if quarter {
        for _ in 1..4 {
            let sibling = renderers[0].create_sibling(embed_options)?;
            renderers.push(sibling);
        }
    }
    let target = target(host, whole, samples, mode == Mode::PassMsaa);
    let mut scene = Scene::new(case, count, scene_size, 1.0);
    let regions = [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(x, y)| {
        PhysicalRect::new(
            x * scene_size.width,
            y * scene_size.height,
            scene_size.width,
            scene_size.height,
        )
    });
    let mut samples_out = Samples::with_capacity(options.iterations);
    let mut before = None;
    let mut started = Instant::now();
    let mut degraded = 0;
    for step in 0..options.warmup + options.iterations {
        if step == options.warmup {
            host.device.poll(wgpu::PollType::wait_indefinitely())?;
            before = Some((renderers[0].stats(), scene.context.cache_stats()));
            started = Instant::now();
        }
        let frame = Instant::now();
        scene.input(step);
        let input = frame.elapsed();
        let ui = Instant::now();
        scene.build();
        let ui = ui.elapsed();
        let render = Instant::now();
        let data = scene.draw_data();
        let mut encoder = host.device.create_command_encoder(&Default::default());
        match mode {
            Mode::Pass | Mode::PassMsaa => {
                renderers[0].prepare(data, EmbedViewport::whole(whole))?;
                let mut pass = pass(&mut encoder, &target);
                degraded = renderers[0].record(&mut pass)?.skipped_backdrop.len();
            }
            Mode::Texture => {
                drop(pass(&mut encoder, &target));
                renderers[0].render_to(&mut encoder, &target.view, None, EmbedLoad::Keep, data)?;
            }
            Mode::Viewports => {
                for (renderer, region) in renderers.iter_mut().zip(regions) {
                    renderer.prepare(data, EmbedViewport::region(whole, region))?;
                }
                let mut pass = pass(&mut encoder, &target);
                for renderer in &renderers {
                    renderer.record(&mut pass)?;
                }
            }
        }
        host.queue.submit([encoder.finish()]);
        // Keep the queue from growing without waiting for the GPU.
        host.device.poll(wgpu::PollType::Poll)?;
        let render = render.elapsed();
        if step >= options.warmup {
            samples_out.input.push(input);
            samples_out.ui.push(ui);
            samples_out.render.push(render);
            samples_out.total.push(frame.elapsed());
            if case.interactive() {
                samples_out.response.push(frame.elapsed());
            }
        }
        scene.verify();
    }
    host.device.poll(wgpu::PollType::wait_indefinitely())?;
    let elapsed = started.elapsed();
    let (gpu_before, cpu_before) = before.unwrap();
    let mut counters = Counters::cpu(cpu_before, scene.context.cache_stats());
    counters.gpu(gpu_before, renderers[0].stats());
    verify(
        host, &target, case, mode, &counters, options, degraded, whole,
    );
    Ok(ResultRow::new(
        "Embed",
        case.name(),
        count,
        mode.name(),
        samples_out,
        elapsed.max(Duration::from_nanos(1)),
        counters,
        Geometry::new(scene.draw_data()),
    ))
}

/// What a row must show to count as verified: pixels the interface drew, uploads only where
/// the scene changed, and the documented degradation of backdrop effects in a pass.
#[allow(clippy::too_many_arguments)]
fn verify(
    host: &Host,
    target: &Target,
    case: Case,
    mode: Mode,
    counters: &Counters,
    options: &Options,
    degraded: usize,
    size: [u32; 2],
) {
    let pixels = read(host, &target.texture, size);
    let clear = pixels[0];
    let drawn = pixels.iter().filter(|p| **p != clear).count();
    assert!(
        drawn > 1000,
        "{} {}: the interface drew {drawn} pixels",
        case.name(),
        mode.name()
    );
    if case == Case::Cached {
        assert_eq!(
            counters.geometry_uploads, 0,
            "an idle scene uploaded geometry"
        );
        assert_eq!(
            counters.texture_uploads, 0,
            "an idle scene uploaded textures"
        );
    }
    if mode != Mode::Viewports {
        assert!(counters.presents as usize >= options.iterations);
    }
    let blurred = matches!(case, Case::Blur24);
    match mode {
        Mode::Pass | Mode::PassMsaa => assert_eq!(blurred, degraded > 0),
        _ => {}
    }
}

fn read(host: &Host, texture: &wgpu::Texture, [width, height]: [u32; 2]) -> Vec<[u8; 4]> {
    let row = (width * 4).next_multiple_of(256);
    let buffer = host.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = host.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    host.queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap()
        });
    host.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let data = buffer.slice(..).get_mapped_range().unwrap();
    (0..height)
        .flat_map(|y| {
            data[(y * row) as usize..][..(width * 4) as usize]
                .as_chunks::<4>()
                .0
                .to_vec()
        })
        .collect()
}
