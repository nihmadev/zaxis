//! Live frames, as an embedded browser or a video player shows them: one image the size of
//! the target, a changed part of it every step. The same updates go through
//! `update_image(ImageSource::rgba(..))` (the whole frame, as before) and
//! `update_image_region` in RGBA, BGRA and opaque BGRA, into a host pass, a host texture
//! and two windows sharing one image. Timed: the update call, the interface pass and
//! `prepare` + `record`/`render_to` + submission. Pixels, write counts, bytes and the absence
//! of texture and bind group creation are verified outside the timed sections. GPU
//! execution time is not measured: CPU time of the calls is not GPU time.
#[path = "live_image/frames.rs"]
mod frames;
#[path = "live_image/gpu.rs"]
mod gpu;

use crate::{
    report::{Counters, FailedCase, Geometry, Report, ResultRow, Samples},
    BenchResult, Options,
};
use frames::{Layout, Model, Shape};
use gpu::{present, read, target, write_texture, Host};
use std::{sync::Arc, time::Duration};
use zaxis::{
    vec2, Context, EmbedOptions, EmbeddedRenderer, Image, ImageFit, ImageHandle, Instant, Rect,
    Root, SharedResources,
};

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const FRAMES: [(&str, [u32; 2]); 2] = [("1440p", [2560, 1440]), ("4k", [3840, 2160])];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Path {
    /// `update_image` with a new RGBA source holding the whole frame.
    Full,
    Region(Layout),
    /// The host writes the rectangle into a texture of its own, registered with the renderer.
    External,
}

impl Path {
    const ALL: [Self; 5] = [
        Self::Full,
        Self::Region(Layout::Rgba),
        Self::Region(Layout::Bgra),
        Self::Region(Layout::BgraOpaque),
        Self::External,
    ];
    fn name(self) -> String {
        match self {
            Self::Full => "full_rgba".into(),
            Self::Region(layout) => format!("region_{}", layout.name()),
            Self::External => "external_bgra_opaque".into(),
        }
    }
    fn layout(self) -> Layout {
        match self {
            Self::Full => Layout::Rgba,
            Self::Region(layout) => layout,
            Self::External => Layout::BgraOpaque,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Context only: the update and the interface pass, no renderer.
    Context,
    Pass,
    Texture,
    TwoWindows,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::Context => "ctx",
            Self::Pass => "embed_pass",
            Self::Texture => "embed_texture",
            Self::TwoWindows => "embed_2_windows",
        }
    }
}

/// Whether a `--filter` asks for these rows (or for everything).
pub fn selected(filter: &str) -> bool {
    filter
        .split(',')
        .any(|f| f.is_empty() || f.contains("live"))
}

fn wanted(filter: &str, name: &str) -> bool {
    filter
        .split(',')
        .any(|f| f.is_empty() || f.contains("live") && name.contains(f) || "live".contains(f))
}

pub fn run(options: &Options, report: &mut Report) -> BenchResult<()> {
    if !selected(&options.filter) {
        return Ok(());
    }
    let host = if options.gpu {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
        Some(Host {
            device,
            queue,
            adapter,
        })
    } else {
        None
    };
    for (frame_name, size) in FRAMES {
        for shape in Shape::ALL {
            for path in Path::ALL {
                let case = format!("live_{frame_name}_{}_{}", shape.name(), path.name());
                if !wanted(&options.filter, &case) {
                    continue;
                }
                let mut modes = Vec::new();
                if options.cpu && matches!(path, Path::Full | Path::Region(Layout::BgraOpaque)) {
                    modes.push(Mode::Context);
                }
                if options.gpu {
                    modes.extend([Mode::Pass, Mode::Texture]);
                    if shape == Shape::Rows64 && size[0] == 2560 {
                        modes.push(Mode::TwoWindows);
                    }
                }
                for mode in modes {
                    report.failed_case = Some(FailedCase::new(
                        "Live",
                        &case,
                        size[0] as usize,
                        mode.name(),
                    ));
                    let row = measure(host.as_ref(), options, &case, size, shape, path, mode)?;
                    row.print();
                    report.results.push(row);
                    report.failed_case = None;
                }
            }
        }
    }
    Ok(())
}

pub struct Window {
    pub ctx: Context,
    pub renderer: Option<EmbeddedRenderer>,
    pub target: Option<wgpu::Texture>,
}

fn build(ctx: &mut Context, handle: ImageHandle, size: [u32; 2]) {
    ctx.run(|ctx| {
        Root::new().padding(Default::default()).show(ctx, |ui| {
            let rect = Rect::from_min_size(vec2(0.0, 0.0), vec2(size[0] as f32, size[1] as f32));
            ui.at("frame", rect, |ui| {
                ui.add(Image::new(handle).size(rect.size()).fit(ImageFit::Stretch));
            });
        });
    });
}

fn context(resources: &SharedResources, size: [u32; 2]) -> Context {
    let mut ctx = Context::with_shared(resources);
    ctx.set_viewport(zaxis::winit::dpi::PhysicalSize::new(size[0], size[1]), 1.0);
    ctx
}

/// Passes until a replaced source has been decoded and shown, and the time of those passes
/// (waiting for the worker is not counted).
fn settle(window: &mut Window, handle: ImageHandle, size: [u32; 2]) -> Duration {
    let start = Instant::now();
    let mut passes = Duration::ZERO;
    loop {
        let pass = Instant::now();
        build(&mut window.ctx, handle, size);
        passes += pass.elapsed();
        if window.ctx.image_state(handle).is_ready() && window.ctx.image_metrics().pending_jobs == 0
        {
            return passes;
        }
        assert!(start.elapsed().as_secs() < 30, "image never became ready");
        std::thread::sleep(Duration::from_micros(200));
    }
}

fn measure(
    host: Option<&Host>,
    options: &Options,
    case: &str,
    size: [u32; 2],
    shape: Shape,
    path: Path,
    mode: Mode,
) -> BenchResult<ResultRow> {
    // The context-only rows have no renderer, whether or not a device exists.
    let host = host.filter(|_| mode != Mode::Context);
    let resources = SharedResources::new();
    let count = if mode == Mode::TwoWindows { 2 } else { 1 };
    let embed_options = EmbedOptions::new(FORMAT);
    let mut windows: Vec<Window> = Vec::new();
    for n in 0..count {
        let renderer = match (host, mode) {
            (Some(host), m) if m != Mode::Context => Some(if let Some(first) = windows.first() {
                first
                    .renderer
                    .as_ref()
                    .expect("renderer")
                    .create_sibling(embed_options)?
            } else {
                EmbeddedRenderer::new_with_adapter(
                    host.device.clone(),
                    host.queue.clone(),
                    &host.adapter,
                    embed_options,
                )?
            }),
            _ => None,
        };
        windows.push(Window {
            ctx: context(&resources, size),
            target: host.filter(|_| renderer.is_some()).map(|h| target(h, size)),
            renderer,
        });
        let _ = n;
    }
    let mut model = Model::new(size);
    let layout = path.layout();
    // The host's own texture, written by the host's queue, for the external rows.
    let host_texture = host.filter(|_| path == Path::External).map(|h| {
        let texture = h.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("live bench host texture"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        write_texture(
            h,
            &texture,
            [0, 0],
            size,
            &model.bytes,
            0,
            size[0] as usize * 4,
        );
        texture
    });
    let registration = match (&host_texture, windows[0].renderer.as_mut()) {
        (Some(texture), Some(renderer)) => Some(renderer.register_external_texture(
            texture,
            zaxis::ExternalTextureOptions {
                alpha: zaxis::ExternalAlpha::Opaque,
                ..Default::default()
            },
        )?),
        _ => None,
    };
    let handle = match &registration {
        Some(external) => windows[0]
            .ctx
            .load_image(zaxis::ImageSource::external(external.image()))?,
        None => windows[0]
            .ctx
            .load_image(layout.source(size, model.bytes.clone()))?,
    };
    let settle_all = |windows: &mut Vec<Window>| {
        windows
            .iter_mut()
            .map(|w| settle(w, handle, size))
            .sum::<Duration>()
    };
    settle_all(&mut windows);
    if let Some(host) = host {
        for w in &mut windows {
            present(host, w, size, mode, options.gpu_wait)?;
        }
    }
    let total = options.warmup + options.iterations;
    let mut samples = Samples::with_capacity(options.iterations);
    let mut counters_before = None;
    let mut region_bytes = 0u64;
    for step in 0..total {
        let region = model.region(shape, step);
        let patch = model.paint(region, step);
        let full = matches!(path, Path::Full).then(|| Arc::new(model.bytes.clone()));
        if step == options.warmup {
            counters_before = Some((
                windows[0].ctx.cache_stats(),
                windows[0].ctx.image_metrics(),
                windows[0].renderer.as_ref().map(|r| r.stats()),
            ));
            region_bytes = 0;
        }
        region_bytes += region.area() * 4;
        let input = Instant::now();
        match path {
            Path::Full => windows[0]
                .ctx
                .update_image(handle, layout.source(size, full.expect("frame")))?,
            Path::External => {
                if let (Some(host), Some(texture)) = (host, &host_texture) {
                    let at = (region.y as usize * size[0] as usize + region.x as usize) * 4;
                    write_texture(
                        host,
                        texture,
                        [region.x, region.y],
                        [region.width, region.height],
                        &model.bytes,
                        at,
                        size[0] as usize * 4,
                    );
                }
                windows[0].ctx.request_repaint();
            }
            Path::Region(_) => windows[0].ctx.update_image_region(
                handle,
                region,
                &patch,
                region.width as usize * 4,
            )?,
        }
        let input = input.elapsed();
        // A replaced source is shown only after a worker has taken it: the passes that wait
        // for it are interface passes of this update and count; the waiting does not.
        let mut ui = if path == Path::Full {
            settle_all(&mut windows)
        } else {
            Duration::ZERO
        };
        let pass = Instant::now();
        for w in &mut windows {
            build(&mut w.ctx, handle, size);
        }
        ui += pass.elapsed();
        let render = Instant::now();
        if let Some(host) = host {
            for w in &mut windows {
                present(host, w, size, mode, options.gpu_wait)?;
            }
        }
        let render = render.elapsed();
        if step >= options.warmup {
            samples.input.push(input);
            samples.ui.push(ui);
            samples.render.push(render);
            samples.total.push(input + ui + render);
        }
    }
    let elapsed: Duration = samples.total.iter().sum();
    let (cache_before, images_before, gpu_before) = counters_before.expect("measured steps");
    let mut counters = Counters::cpu(cache_before, windows[0].ctx.cache_stats());
    counters.images(images_before, windows[0].ctx.image_metrics());
    if let (Some(before), Some(r)) = (gpu_before, &windows[0].renderer) {
        counters.gpu(before, r.stats());
    }
    if let Some(host) = host {
        host.device.poll(wgpu::PollType::wait_indefinitely())?;
        verify(
            host,
            &windows,
            &model,
            layout,
            path,
            shape,
            &counters,
            options,
            region_bytes,
        );
    } else {
        // No renderer: the frame the context publishes is the model, byte for byte.
        let data = windows[0].ctx.draw_data();
        let image = data
            .textures
            .iter()
            .find(|t| t.size == size)
            .expect("the image payload");
        assert_eq!(&image.pixels[..], &model.bytes[..], "published frame");
        if matches!(path, Path::Region(_)) {
            assert!(counters.image_live_full_copies <= 2);
            assert_eq!(counters.image_region_updates, options.iterations as u64);
        }
    }
    let data = windows[0].ctx.draw_data();
    Ok(ResultRow::new(
        "Live",
        case,
        size[0] as usize,
        mode.name(),
        samples,
        elapsed.max(Duration::from_nanos(1)),
        counters,
        Geometry::new(data),
    ))
}

/// Pixels of every window, and counters that show what the path wrote.
#[allow(clippy::too_many_arguments)]
fn verify(
    host: &Host,
    windows: &[Window],
    model: &Model,
    layout: Layout,
    path: Path,
    shape: Shape,
    counters: &Counters,
    options: &Options,
    region_bytes: u64,
) {
    let want = model.expected(layout);
    for (n, w) in windows.iter().enumerate() {
        let got = read(host, w.target.as_ref().expect("target"), model.size);
        let worst = got
            .iter()
            .zip(&want)
            .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)))
            .max()
            .unwrap_or(0);
        // Interpolated coordinates across thousands of pixels are a fraction of a texel off, which
        // linear filtering turns into a step or two on noisy data.
        let allowed = if model.size[0] > 2560 { 3 } else { 1 };
        assert!(worst <= allowed, "window {n}: pixels differ by {worst}");
    }
    let frame = u64::from(model.size[0]) * u64::from(model.size[1]) * 4;
    let steps = options.iterations as u64;
    match path {
        Path::Region(_) => {
            assert_eq!(counters.texture_creations, 0);
            assert_eq!(counters.texture_bind_group_creations, 0);
            assert_eq!(counters.texture_upload_bytes, 0);
            assert_eq!(counters.texture_update_fallbacks, 0);
            assert_eq!(counters.texture_region_updates, steps);
            assert_eq!(counters.texture_region_bytes, region_bytes);
            assert_eq!(counters.image_region_updates, steps);
            assert!(counters.texture_region_bytes <= steps * frame);
            let _ = shape;
        }
        Path::External => {
            // The host wrote the pixels; the renderer wrote and created nothing.
            assert_eq!(counters.texture_writes, 0);
            assert_eq!(counters.texture_creations, 0);
            assert_eq!(counters.texture_bind_group_creations, 0);
            assert_eq!(counters.texture_region_updates, 0);
        }
        Path::Full => {
            assert_eq!(counters.texture_creations, 0);
            assert_eq!(counters.texture_region_updates, 0);
            assert_eq!(counters.texture_upload_bytes, steps * frame);
        }
    }
}
