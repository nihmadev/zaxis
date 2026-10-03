//! Deterministic image assets and async probes used by the existing CPU/native suites.
use std::{
    io::Cursor,
    sync::mpsc,
    time::{Duration, Instant},
};
use zaxis::{
    vec2, Context, Image, ImageFit, ImageHandle, ImageSource, ImageState, Rect, ScrollArea,
    TextureFilter, Window,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageCase {
    Empty,
    Warm,
    ColdPng,
    ColdJpeg,
    ColdWebp,
    ColdFile,
    Shared,
    Unique,
    Pixels,
    Dimensions,
    SvgSmall,
    SvgLarge,
    SvgDpi,
    SvgResize,
    Downsample,
    Scroll,
    Evict,
    GpuRecovery,
    FirstShow,
    RepeatedShow,
}
impl ImageCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "image_empty",
            Self::Warm => "image_warm",
            Self::ColdPng => "image_cold_png",
            Self::ColdJpeg => "image_cold_jpeg",
            Self::ColdWebp => "image_cold_webp",
            Self::ColdFile => "image_cold_file",
            Self::Shared => "image_shared_128",
            Self::Unique => "image_unique_64",
            Self::Pixels => "image_update_pixels",
            Self::Dimensions => "image_replace_dimensions",
            Self::SvgSmall => "image_svg_small_cold",
            Self::SvgLarge => "image_svg_large_cold",
            Self::SvgDpi => "image_svg_dpi",
            Self::SvgResize => "image_svg_resize",
            Self::Downsample => "image_downsample",
            Self::Scroll => "image_scroll_gallery",
            Self::Evict => "image_eviction_reload",
            Self::GpuRecovery => "image_gpu_recovery",
            Self::FirstShow => "image_first_show",
            Self::RepeatedShow => "image_repeat_show",
        }
    }
    pub fn cold(self) -> bool {
        matches!(
            self,
            Self::ColdPng
                | Self::ColdJpeg
                | Self::ColdWebp
                | Self::ColdFile
                | Self::SvgSmall
                | Self::SvgLarge
                | Self::Downsample
                | Self::Evict
        )
    }
    pub fn stable(self) -> bool {
        matches!(
            self,
            Self::Warm | Self::Shared | Self::Unique | Self::GpuRecovery | Self::FirstShow
        )
    }
    pub fn unsupported(self, side: usize) -> Option<String> {
        if self == Self::ColdWebp && side >= 4096 {
            return Some(
                "deterministic lossless WebP fixture exceeds the 16 MiB encoded limit".into(),
            );
        }
        let resources = if self == Self::Unique {
            64
        } else if self == Self::Scroll {
            9
        } else {
            1
        };
        if side == 0
            || side > 8192
            || side
                .checked_mul(side)
                .and_then(|n| n.checked_mul(4 * resources * 2))
                .map_or(true, |n| n > 128 << 20)
        {
            Some(format!(
                "{resources} sources at {side}² exceed default CPU budget"
            ))
        } else {
            None
        }
    }
}
pub struct Probe {
    pub case: ImageCase,
    pub sources: Vec<ImageSource>,
    alternates: Vec<ImageSource>,
    handle: Option<ImageHandle>,
    pub active: Vec<ImageSource>,
    pub request: Option<Instant>,
    rx: mpsc::Receiver<()>,
    pub step: usize,
    viewport: Rect,
    revision: u64,
    decodes: u64,
    rasters: u64,
    pub measured_ready: bool,
}
impl Probe {
    pub fn new(context: &mut Context, case: ImageCase, side: usize) -> Self {
        if case == ImageCase::Scroll {
            let mut limits = context.image_limits().clone();
            limits.gpu_cache_bytes = 8 << 20;
            limits.max_gpu_bindings = 64;
            context.set_image_limits(limits);
        }
        let (tx, rx) = mpsc::channel();
        context.set_image_waker(move || {
            let _ = tx.send(());
        });
        let vector = matches!(
            case,
            ImageCase::SvgSmall | ImageCase::SvgLarge | ImageCase::SvgDpi | ImageCase::SvgResize
        );
        let raw = std::sync::Arc::new(if vector || case == ImageCase::Empty {
            Vec::new()
        } else {
            rgba(side, 0)
        });
        let source = if case == ImageCase::Empty {
            ImageSource::bytes(b"")
        } else if matches!(
            case,
            ImageCase::SvgSmall | ImageCase::SvgLarge | ImageCase::SvgDpi | ImageCase::SvgResize
        ) {
            ImageSource::encoded(
                svg(side, if case == ImageCase::SvgLarge { 4000 } else { 8 }).into_bytes(),
            )
        } else if matches!(case, ImageCase::Pixels | ImageCase::Dimensions) {
            ImageSource::rgba([side as u32; 2], raw.clone())
        } else {
            let format = match case {
                ImageCase::ColdJpeg => image::ImageFormat::Jpeg,
                ImageCase::ColdWebp => image::ImageFormat::WebP,
                _ => image::ImageFormat::Png,
            };
            let encoded = encode(side, &raw, format);
            if case == ImageCase::ColdFile {
                let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("target/image-benchmark-assets");
                std::fs::create_dir_all(&dir).unwrap();
                let path = dir.join(format!("{side}.wrong-extension"));
                std::fs::write(&path, &encoded).unwrap();
                ImageSource::path(path)
            } else {
                ImageSource::encoded(encoded)
            }
        };
        let mut sources = vec![source];
        if case == ImageCase::Unique || case == ImageCase::Scroll {
            // Different identities share immutable encoded storage; asset preparation must
            // not allocate 512 copies before the cache budget even gets a chance to act.
            let source = sources[0].clone();
            for revision in 1..if case == ImageCase::Unique { 64 } else { 512 } {
                sources.push(source.clone().with_revision(revision));
            }
        }
        let alternates = if matches!(case, ImageCase::Pixels | ImageCase::Dimensions) {
            vec![
                ImageSource::rgba([side as u32; 2], raw),
                ImageSource::rgba([side as u32; 2], rgba(side, 77)),
                ImageSource::rgba([(side / 2).max(1) as u32; 2], rgba((side / 2).max(1), 23)),
            ]
        } else {
            Vec::new()
        };
        let handle = matches!(case, ImageCase::Pixels | ImageCase::Dimensions)
            .then(|| context.load_image(&sources[0]).unwrap());
        Self {
            case,
            sources,
            alternates,
            handle,
            active: Vec::new(),
            request: None,
            rx,
            step: 0,
            viewport: Rect::default(),
            revision: 0,
            decodes: 0,
            rasters: 0,
            measured_ready: false,
        }
    }
    pub fn input(&mut self, c: &mut Context, step: usize) {
        self.step = step;
        self.revision = c.draw_data().revision;
        self.decodes = c.image_metrics().decodes;
        self.rasters = c.image_metrics().rasterizations;
        self.measured_ready = false;
        self.request = Some(Instant::now());
        if self.case.cold() {
            c.clear_image_cache();
        }
        if let Some(h) = self.handle {
            let index = if self.case == ImageCase::Dimensions && step % 2 == 0 {
                2
            } else {
                step % 2
            };
            c.update_image(h, &self.alternates[index]).unwrap();
        }
        if self.case == ImageCase::SvgDpi {
            let scale = [1.0, 1.25, 1.5, 2.0][step % 4];
            c.set_viewport(zaxis::winit::dpi::PhysicalSize::new(1280, 800), scale);
        }
        if self.case == ImageCase::Scroll {
            use zaxis::winit::{
                dpi::PhysicalPosition,
                event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent},
            };
            let p = self.viewport.center() * c.scale_factor();
            c.on_window_event(&WindowEvent::CursorMoved {
                device_id: DeviceId::dummy(),
                position: PhysicalPosition::new(p.x as f64, p.y as f64),
            });
            c.on_window_event(&WindowEvent::MouseWheel {
                device_id: DeviceId::dummy(),
                delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                    0.0,
                    if step % 900 < 450 { -85.0 } else { 85.0 },
                )),
                phase: TouchPhase::Moved,
            });
        }
    }
    pub fn build(&mut self, c: &mut Context) {
        self.active.clear();
        let now = Instant::now();
        c.run_at(now, |c| {
            Window::new("Image benchmark")
                .default_position(vec2(5.0, 5.0))
                .default_size(vec2(1000.0, 700.0))
                .draggable(false)
                .resizable(false)
                .show(c, |ui| {
                    if self.case == ImageCase::Empty
                        || self.case == ImageCase::RepeatedShow && self.step % 2 == 0
                    {
                        return;
                    }
                    if self.case == ImageCase::Scroll {
                        let out = ScrollArea::vertical()
                            .id_source("gallery")
                            .max_height(540.0)
                            .show_rows(ui, 72.0, self.sources.len(), |ui, i| {
                                let response = ui.add(
                                    Image::new(&self.sources[i])
                                        .size(vec2(96.0, 64.0))
                                        .fit(ImageFit::Stretch),
                                );
                                if !response.rect.intersect(ui.clip_rect()).is_empty() {
                                    self.active.push(self.sources[i].clone());
                                }
                            });
                        self.viewport = out.viewport;
                    } else {
                        let count: usize = match self.case {
                            ImageCase::Shared => 128,
                            ImageCase::Unique => 64,
                            _ => 1,
                        };
                        for row in 0..count.div_ceil(16) {
                            ui.horizontal(|ui| {
                                for i in row * 16..((row + 1) * 16).min(count) {
                                    ui.push_id(i, |ui| {
                                        let source = if let Some(h) = self.handle {
                                            ImageSource::from(h)
                                        } else {
                                            self.sources[i % self.sources.len()].clone()
                                        };
                                        self.active.push(source.clone());
                                        let size = if count > 1 {
                                            28.0
                                        } else if self.case == ImageCase::SvgResize {
                                            90.0 + (self.step % 31) as f32 * 3.0
                                        } else {
                                            96.0
                                        };
                                        let filter = if matches!(
                                            self.case,
                                            ImageCase::FirstShow
                                                | ImageCase::GpuRecovery
                                                | ImageCase::Pixels
                                                | ImageCase::Dimensions
                                        ) {
                                            TextureFilter::Nearest
                                        } else {
                                            TextureFilter::Linear
                                        };
                                        ui.add(
                                            Image::new(source)
                                                .size(vec2(size, size))
                                                .fit(ImageFit::Stretch)
                                                .filter(filter),
                                        );
                                    });
                                }
                            });
                        }
                    }
                });
        });
    }
    pub fn ready(&self, c: &Context) -> bool {
        self.active.iter().all(|s| c.image_state(s).is_ready())
            && c.image_metrics().pending_jobs == 0
            && (!matches!(self.case, ImageCase::SvgResize | ImageCase::SvgDpi)
                || c.next_repaint().is_none())
    }
    /// CPU suite waits on completion events; no busy loop or sleep polling.
    pub fn await_ready(&mut self, c: &mut Context) -> Duration {
        let start = Instant::now();
        let mut ui_time = Duration::ZERO;
        while !self.ready(c) {
            for source in &self.active {
                if let ImageState::Error(e) = c.image_state(source) {
                    panic!("{e}");
                }
            }
            if !c.needs_repaint() {
                let timeout = c.next_repaint().map_or(Duration::from_secs(30), |at| {
                    at.saturating_duration_since(Instant::now())
                });
                match self.rx.recv_timeout(timeout) {
                    Ok(()) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) if c.next_repaint().is_some() => {}
                    Err(e) => panic!("image completion wake: {e}"),
                }
            }
            let ui = Instant::now();
            self.build(c);
            ui_time += ui.elapsed();
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "image load never finished"
            );
        }
        ui_time
    }
    pub fn verify(&self, c: &Context) {
        assert!(c.image_metrics().cpu_resident_bytes <= c.image_limits().cpu_cache_bytes);
        if self.ready(c) && self.case.stable() && self.step > 0 {
            assert_eq!(c.image_metrics().decodes, self.decodes);
            assert_eq!(c.image_metrics().rasterizations, self.rasters);
            assert_eq!(
                c.draw_data().revision,
                self.revision,
                "steady images rebuilt geometry"
            );
        }
        if self.case == ImageCase::Shared {
            assert_eq!(c.draw_data().texture_options.len(), 1);
        }
    }
}
fn rgba(side: usize, seed: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(side * side * 4);
    for y in 0..side {
        for x in 0..side {
            data.extend([
                ((x % 64) * 4) as u8 ^ seed,
                ((y % 64) * 4) as u8,
                ((x % 64).wrapping_mul(17) ^ (y % 64).wrapping_mul(13)) as u8,
                if (x + y) % 8 == 0 { 96 } else { 255 },
            ]);
        }
    }
    data
}
fn encode(side: usize, rgba: &[u8], format: image::ImageFormat) -> Vec<u8> {
    if format == image::ImageFormat::Png {
        use image::{
            codecs::png::{CompressionType, FilterType, PngEncoder},
            ImageEncoder,
        };
        let mut output = Vec::new();
        PngEncoder::new_with_quality(&mut output, CompressionType::Best, FilterType::Adaptive)
            .write_image(
                rgba,
                side as u32,
                side as u32,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        assert!(
            output.len() <= 16 << 20,
            "benchmark PNG exceeds encoded budget"
        );
        return output;
    }
    let image = image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(side as u32, side as u32, rgba.to_vec()).unwrap(),
    );
    let image = if format == image::ImageFormat::Jpeg {
        image::DynamicImage::ImageRgb8(image.into_rgb8())
    } else {
        image
    };
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, format).unwrap();
    output.into_inner()
}
fn svg(side: usize, paths: usize) -> String {
    let mut svg=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{side}\" height=\"{side}\" viewBox=\"0 0 100 100\">");
    for i in 0..paths {
        svg.push_str(&format!(
            "<path d=\"M{} {}q20 -15 15 15t-15 15Z\" fill=\"#{:06x}\" fill-opacity=\".5\"/>",
            i % 80,
            (i * 17) % 80,
            (i * 7919) % 0xffffff
        ));
    }
    svg.push_str("</svg>");
    svg
}
