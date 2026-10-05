use std::{
    io::Cursor,
    sync::{mpsc, Arc, Condvar, Mutex},
    time::Duration,
};
use zaxis::images::{decode, resize, source, *};
use zaxis::Instant;
use zaxis::{vec2, Context, Image, ImageFit, TextureFilter, Window};

fn encoded(format: image::ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        3,
        2,
        image::Rgba([255, 32, 0, 128]),
    ))
    .write_to(&mut out, format)
    .unwrap();
    out.into_inner()
}
fn decode_bytes(bytes: Vec<u8>) -> Result<decode::Document, ImageError> {
    decode::load(
        &source::Source::Encoded(Arc::new(bytes)),
        &ImageLimits::default(),
        &[],
        &mut Vec::new(),
    )
}
const SVG:&[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 20"><defs><linearGradient id="g"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><clipPath id="c"><circle cx="20" cy="10" r="8"/></clipPath></defs><g transform="translate(1 0)" clip-path="url(#c)"><path d="M0 0H40V20H0Z" fill="url(#g)" stroke="white"/></g></svg>"##;

#[test]
fn supported_formats_sniff_content_and_first_frames() {
    let mut formats = vec![
        image::ImageFormat::Png,
        image::ImageFormat::Jpeg,
        image::ImageFormat::WebP,
        image::ImageFormat::Bmp,
    ];
    #[cfg(feature = "image-gif")]
    formats.push(image::ImageFormat::Gif);
    #[cfg(feature = "image-tiff")]
    formats.push(image::ImageFormat::Tiff);
    for format in formats {
        let mut out = Cursor::new(Vec::new());
        let image = if format == image::ImageFormat::Jpeg {
            image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                3,
                2,
                image::Rgb([255, 32, 0]),
            ))
        } else {
            image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                3,
                2,
                image::Rgba([255, 32, 0, 128]),
            ))
        };
        image.write_to(&mut out, format).unwrap();
        let bytes = out.into_inner();
        assert_eq!(
            decode_bytes(bytes.clone()).unwrap().size(),
            [3, 2],
            "{format:?}"
        );
        assert!(
            decode_bytes(bytes[..bytes.len() / 2].to_vec()).is_err(),
            "{format:?} accepted truncated input"
        );
    }
    assert_eq!(decode_bytes(SVG.to_vec()).unwrap().size(), [40, 20]);
    assert!(decode_bytes(b"bad content".to_vec()).is_err());
}
#[test]
fn jpeg_exif_orientation_is_applied() {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(3, 2, image::Rgb([255, 0, 0])))
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .unwrap();
    let jpeg = out.into_inner();
    let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut oriented = jpeg[..2].to_vec();
    oriented.extend([0xff, 0xe1]);
    oriented.extend(((exif.len() + 2) as u16).to_be_bytes());
    oriented.extend(exif);
    oriented.extend(&jpeg[2..]);
    assert_eq!(decode_bytes(oriented).unwrap().size(), [2, 3]);
}
#[test]
fn alpha_conversion_area_reduction_and_limits() {
    let mut pixels = [128, 64, 0, 128, 20, 30, 40, 0];
    decode::unpremultiply(&mut pixels);
    assert_eq!(pixels, [255, 128, 0, 128, 0, 0, 0, 0]);
    let input = DecodedImage {
        size: [2, 2],
        pixels: Arc::new(vec![
            255, 0, 0, 255, 0, 0, 255, 0, 255, 0, 0, 255, 0, 0, 255, 0,
        ]),
    };
    let output = resize::downsample(&input, [1, 1], &ImageLimits::default()).unwrap();
    assert_eq!(&output.pixels[..], [255, 0, 0, 128]);
    let limit = ImageLimits {
        max_decoded_bytes: 12,
        ..Default::default()
    };
    assert!(limit.check_size([2, 2]).is_err());
    assert!(limit.check_size([u32::MAX, u32::MAX]).is_err());
    assert!(decode::validate_rgba([2, 2], &[0; 15], &ImageLimits::default()).is_err());
    let bytes = encoded(image::ImageFormat::Png);
    let small = ImageLimits {
        max_encoded_bytes: 8,
        ..Default::default()
    };
    assert!(decode::load(
        &source::Source::Encoded(Arc::new(bytes)),
        &small,
        &[],
        &mut Vec::new()
    )
    .is_err());
    assert!(decode_bytes(
        br#"<svg xmlns="http://www.w3.org/2000/svg"><filter id="f"/></svg>"#.to_vec()
    )
    .is_err());
}
fn context() -> (Context, mpsc::Receiver<()>) {
    let mut c = Context::new();
    c.set_viewport(zaxis::winit::dpi::PhysicalSize::new(800, 600), 1.0);
    let (tx, rx) = mpsc::channel();
    c.set_image_waker(move || {
        let _ = tx.send(());
    });
    (c, rx)
}
fn build(c: &mut Context, h: ImageHandle, n: usize) {
    c.run(|c| {
        Window::new("images")
            .default_size(vec2(700.0, 500.0))
            .show(c, |ui| {
                for i in 0..n {
                    ui.push_id(i, |ui| {
                        ui.add(Image::new(h).size(vec2(32.0, 32.0)).fit(ImageFit::Stretch));
                    });
                }
            });
    });
}
fn ready(c: &mut Context, rx: &mpsc::Receiver<()>, h: ImageHandle, n: usize) {
    let start = Instant::now();
    loop {
        build(c, h, n);
        match c.image_state(h) {
            ImageState::Ready { .. } => return,
            ImageState::Error(e) => panic!("{e}"),
            _ => {}
        }
        assert!(start.elapsed() < Duration::from_secs(10));
        rx.recv_timeout(Duration::from_secs(10)).unwrap();
    }
}
#[test]
fn shared_source_wakes_idle_host_and_pixels_do_not_rebuild_geometry() {
    let (mut c, rx) = context();
    let source = ImageSource::encoded(encoded(image::ImageFormat::Png));
    let h = c.load_image(&source).unwrap();
    assert_eq!(h, c.load_image(&source).unwrap());
    ready(&mut c, &rx, h, 6);
    let metrics = c.image_metrics();
    assert_eq!(metrics.decodes, 1);
    let revision = c.draw_data().revision;
    build(&mut c, h, 6);
    assert_eq!(c.draw_data().revision, revision);
    assert_eq!(c.image_metrics().decodes, 1);
    assert_eq!(c.image_metrics().pending_jobs, 0);
    assert!(!c.needs_repaint());
    assert!(c.next_repaint().is_none());
    assert_eq!(c.draw_data().texture_options.len(), 1);
    let texture_id = c
        .draw_data()
        .texture_options
        .keys()
        .next()
        .copied()
        .unwrap();
    c.update_image(h, ImageSource::rgba([3, 2], vec![0; 24]))
        .unwrap();
    ready(&mut c, &rx, h, 6);
    assert_eq!(c.draw_data().revision, revision);
    assert!(c.draw_data().texture_options.contains_key(&texture_id));
    assert!(c
        .draw_data()
        .textures
        .iter()
        .find(|t| t.id == texture_id)
        .unwrap()
        .pixels
        .iter()
        .all(|b| *b == 0));
}
#[test]
fn svg_dpi_resize_debounce_and_parsed_cache() {
    let (mut c, rx) = context();
    let h = c.load_image(SVG).unwrap();
    ready(&mut c, &rx, h, 1);
    let start = Instant::now();
    let pass = |c: &mut Context, at, size| {
        c.run_at(at, |c| {
            Window::new("svg").show(c, |ui| {
                ui.add(Image::new(h).size(size).fit(ImageFit::Stretch));
            });
        });
    };
    for i in 0..30 {
        pass(
            &mut c,
            start + Duration::from_millis(i * 3),
            vec2(70.0 + i as f32, 40.0),
        );
    }
    assert_eq!(c.image_metrics().svg_parses, 1);
    assert_eq!(c.image_metrics().rasterizations, 1);
    pass(&mut c, start + Duration::from_millis(300), vec2(99.0, 40.0));
    rx.recv_timeout(Duration::from_secs(10)).unwrap();
    pass(&mut c, start + Duration::from_millis(301), vec2(99.0, 40.0));
    assert_eq!(c.image_metrics().svg_parses, 1);
    assert_eq!(c.image_metrics().rasterizations, 2);
    let id = *c.draw_data().texture_options.keys().next().unwrap();
    assert_eq!(
        c.draw_data()
            .textures
            .iter()
            .find(|t| t.id == id)
            .unwrap()
            .size,
        [99, 40]
    );
    c.set_viewport(zaxis::winit::dpi::PhysicalSize::new(1600, 1200), 2.0);
    pass(&mut c, start + Duration::from_millis(302), vec2(99.0, 40.0));
    pass(&mut c, start + Duration::from_millis(500), vec2(99.0, 40.0));
    rx.recv_timeout(Duration::from_secs(10)).unwrap();
    pass(&mut c, start + Duration::from_millis(501), vec2(99.0, 40.0));
    assert_eq!(
        c.draw_data()
            .textures
            .iter()
            .find(|t| t.id == id)
            .unwrap()
            .size,
        [198, 80]
    );
}
#[test]
fn text_ids_filters_errors_retry_and_eviction_recovery() {
    let (mut c, rx) = context();
    let h = c
        .load_image(ImageSource::rgba([3, 2], vec![255; 24]))
        .unwrap();
    ready(&mut c, &rx, h, 1);
    c.run(|c| {
        Window::new("mix").show(c, |ui| {
            ui.label("glyphs");
            ui.add(Image::new(h));
            ui.add(Image::new(h).filter(TextureFilter::Nearest));
        });
    });
    let data = c.draw_data();
    let ids: std::collections::HashSet<_> = data.textures.iter().map(|t| t.id).collect();
    assert_eq!(ids.len(), data.textures.len());
    assert_eq!(data.texture_options.len(), 2);
    assert!(data
        .textures
        .iter()
        .any(|t| !data.texture_options.contains_key(&t.id)));
    c.clear_image_cache();
    ready(&mut c, &rx, h, 1);
    assert_eq!(c.image_metrics().decodes, 0);
    c.update_image(h, ImageSource::encoded(vec![1, 2, 3]))
        .unwrap();
    build(&mut c, h, 1);
    rx.recv_timeout(Duration::from_secs(10)).unwrap();
    build(&mut c, h, 1);
    assert!(matches!(c.image_state(h), ImageState::Error(_)));
    let attempts = c.image_metrics().pending_jobs;
    for _ in 0..10 {
        build(&mut c, h, 1);
    }
    assert_eq!(attempts, 0);
    assert_eq!(c.image_metrics().pending_jobs, 0);
    c.update_image(h, ImageSource::rgba([1, 1], vec![255; 4]))
        .unwrap();
    ready(&mut c, &rx, h, 1);
    c.release_image(h);
    assert!(c.load_image(h).is_err());
}
struct BlockedDecoder(Arc<(Mutex<bool>, Condvar)>);
impl ImageDecoder for BlockedDecoder {
    fn decode(&self, b: &[u8], _: &ImageLimits) -> Option<Result<DecodedImage, ImageError>> {
        if b != b"wait" {
            return None;
        }
        let (lock, wake) = &*self.0;
        let mut done = lock.lock().unwrap();
        while !*done {
            done = wake.wait(done).unwrap();
        }
        Some(Ok(DecodedImage {
            size: [1, 1],
            pixels: Arc::new(vec![255, 0, 0, 255]),
        }))
    }
}
#[test]
fn stale_completion_cannot_replace_new_generation_and_drop_does_not_wake() {
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let (mut c, rx) = context();
    c.add_image_decoder(Arc::new(BlockedDecoder(gate.clone())));
    let h = c.load_image(ImageSource::bytes(b"wait")).unwrap();
    build(&mut c, h, 1);
    c.update_image(h, ImageSource::rgba([1, 1], vec![0, 255, 0, 255]))
        .unwrap();
    ready(&mut c, &rx, h, 1);
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    rx.recv_timeout(Duration::from_secs(10)).unwrap();
    build(&mut c, h, 1);
    assert_eq!(c.image_metrics().stale_completions, 1);
    let id = *c.draw_data().texture_options.keys().next().unwrap();
    assert_eq!(
        &c.draw_data()
            .textures
            .iter()
            .find(|t| t.id == id)
            .unwrap()
            .pixels[..],
        [0, 255, 0, 255]
    );
    drop(c);
}
#[test]
fn inline_decoding_spends_the_frame_budget_on_the_drawing_thread() {
    let (mut c, rx) = context();
    c.decode_images_inline(Some(Duration::ZERO));
    let handles: Vec<_> = (0..2u8)
        .map(|n| {
            c.load_image(ImageSource::rgba([2, 2], vec![n * 40; 16]))
                .unwrap()
        })
        .collect();
    let draw = |c: &mut Context| {
        c.run(|c| {
            Window::new("inline").show(c, |ui| {
                for (i, h) in handles.iter().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.add(Image::new(*h).size(vec2(8.0, 8.0)));
                    });
                }
            });
        });
    };
    draw(&mut c);
    assert_eq!(c.image_metrics().pending_jobs, 2);
    assert!(rx.try_recv().is_ok(), "queued work wakes the host");
    // No thread ran anything: each frame decodes exactly one job, as a zero budget allows,
    // and queued work keeps asking for frames until it is done.
    for pending in [1, 0] {
        assert!(c.needs_repaint(), "{pending} jobs remain");
        draw(&mut c);
        assert_eq!(c.image_metrics().pending_jobs, pending);
    }
    draw(&mut c);
    assert!(!c.needs_repaint());
    assert!(handles
        .iter()
        .all(|h| matches!(c.image_state(*h), ImageState::Ready { .. })));
}
#[test]
fn inline_decoding_with_room_in_the_budget_finishes_in_one_frame() {
    let (mut c, _rx) = context();
    c.decode_images_inline(Some(Duration::from_secs(5)));
    let h = c
        .load_image(ImageSource::encoded(encoded(image::ImageFormat::Png)))
        .unwrap();
    build(&mut c, h, 1);
    assert_eq!(c.image_metrics().pending_jobs, 1);
    build(&mut c, h, 1);
    assert_eq!(c.image_metrics().pending_jobs, 0);
    assert_eq!(c.image_metrics().decodes, 1);
    assert!(matches!(c.image_state(h), ImageState::Ready { .. }));
}
#[test]
fn fit_crop_math() {
    let rect = zaxis::Rect::from_min_size(vec2(0.0, 0.0), vec2(100.0, 100.0));
    let uv = zaxis::Rect::from_min_size(vec2(0.0, 0.0), vec2(1.0, 1.0));
    let (r, _) = zaxis::components::image::fit(rect, vec2(200.0, 100.0), uv, ImageFit::Contain);
    assert_eq!(r.size(), vec2(100.0, 50.0));
    let (_, crop) = zaxis::components::image::fit(rect, vec2(200.0, 100.0), uv, ImageFit::Cover);
    assert_eq!(crop.min, vec2(0.25, 0.0));
    assert_eq!(crop.max, vec2(0.75, 1.0));
}
