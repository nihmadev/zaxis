//! Raw pixel images updated in place, through the public API: validation, what a frame
//! carries, ordering, repaint and memory.

use std::sync::Arc;
use zaxis::{
    vec2, Context, DrawData, Image, ImageFit, ImageHandle, ImageSource, PixelFormat, TextureId,
    TextureImage, TextureRegion, Window,
};

pub const W: u32 = 64;
pub const H: u32 = 32;

pub fn context() -> Context {
    let mut c = Context::new();
    c.set_viewport(zaxis::winit::dpi::PhysicalSize::new(800, 600), 1.0);
    c
}

pub fn frame(fill: [u8; 4]) -> Vec<u8> {
    (0..W * H).flat_map(|_| fill).collect()
}

pub fn show(c: &mut Context, h: ImageHandle) {
    c.run(|c| {
        Window::new("live")
            .default_size(vec2(300.0, 200.0))
            .show(c, |ui| {
                ui.add(
                    Image::new(h)
                        .size(vec2(W as f32, H as f32))
                        .fit(ImageFit::Stretch),
                );
            });
    });
}

/// The payload of the one image the frame draws.
pub fn payload(data: &DrawData) -> &TextureImage {
    let id = *data.texture_options.keys().next().expect("one image");
    data.textures.iter().find(|t| t.id == id).unwrap()
}

pub fn texel(image: &TextureImage, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * image.size[0] + x) * 4) as usize;
    image.pixels[at..at + 4].try_into().unwrap()
}

/// Frames until the image is ready, no decode is in flight and nothing wants a repaint.
fn settle(c: &mut Context, h: ImageHandle) {
    let start = std::time::Instant::now();
    loop {
        show(c, h);
        if c.image_state(h).is_ready() && c.image_metrics().pending_jobs == 0 && !c.needs_repaint()
        {
            return;
        }
        assert!(start.elapsed().as_secs() < 10);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

fn live(c: &mut Context, fill: [u8; 4]) -> ImageHandle {
    let h = c
        .load_image(ImageSource::rgba([W, H], frame(fill)))
        .unwrap();
    settle(c, h);
    h
}

fn rejected(
    c: &mut Context,
    h: ImageHandle,
    region: TextureRegion,
    len: usize,
    stride: usize,
) -> String {
    let before = c.draw_data().revision;
    let error = c
        .update_image_region(h, region, &vec![0; len], stride)
        .expect_err(&format!(
            "{region:?} with {len} bytes, stride {stride} must be rejected"
        ))
        .to_string();
    assert_eq!(c.image_metrics().region_updates, 0, "{error}");
    assert!(!c.needs_repaint(), "{error}");
    show(c, h);
    assert_eq!(c.draw_data().revision, before, "{error}");
    error
}

#[test]
fn invalid_regions_are_errors_that_change_nothing() {
    let mut c = context();
    let h = live(&mut c, [10, 20, 30, 255]);
    settle(&mut c, h);
    let ok = vec![0; 16 * 4 * 4];
    let say = |c: &mut Context, region, len, stride| rejected(c, h, region, len, stride);
    let region = TextureRegion::new;
    assert!(say(&mut c, region(0, 0, 0, 4), ok.len(), 64).contains("empty or outside"));
    assert!(say(&mut c, region(0, 0, 4, 0), ok.len(), 64).contains("empty or outside"));
    assert!(say(&mut c, region(W - 3, 0, 4, 4), ok.len(), 64).contains("outside"));
    assert!(say(&mut c, region(0, H - 3, 4, 4), ok.len(), 64).contains("outside"));
    assert!(say(&mut c, region(u32::MAX, 0, 2, 2), ok.len(), 64).contains("outside"));
    assert!(say(&mut c, region(0, 0, u32::MAX, 1), ok.len(), 64).contains("outside"));
    assert!(say(&mut c, region(0, 0, 4, 4), 63, 16).contains("pixels hold"));
    assert!(say(&mut c, region(0, 0, 4, 4), ok.len(), 15).contains("stride"));
    assert!(say(&mut c, region(0, 0, 4, 2), 1 << 20, usize::MAX).contains("pixels hold"));
    // A handle of another Context, and one that was released.
    let mut other = context();
    let foreign = other
        .load_image(ImageSource::rgba([2, 2], vec![0; 16]))
        .unwrap();
    let error = c.update_image_region(foreign, region(0, 0, 1, 1), &[0; 4], 4);
    assert!(error.unwrap_err().to_string().contains("foreign"));
    let gone = c
        .load_image(ImageSource::rgba([2, 2], vec![0; 16]))
        .unwrap();
    c.release_image(gone);
    settle(&mut c, h);
    let error = c.update_image_region(gone, region(0, 0, 1, 1), &[0; 4], 4);
    assert!(error.unwrap_err().to_string().contains("released"));
    // Decoded files have no pixels of their own to patch.
    let file = c.load_image(ImageSource::encoded(vec![1, 2, 3])).unwrap();
    let error = c.update_image_region(file, region(0, 0, 1, 1), &[0; 4], 4);
    assert!(error.unwrap_err().to_string().contains("raw pixel"));
    // A source that broke a limit stays an error.
    let bad = c.load_image(ImageSource::rgba([2, 2], vec![0; 3])).unwrap();
    let error = c.update_image_region(bad, region(0, 0, 1, 1), &[0; 4], 4);
    assert!(error.is_err());
    assert_eq!(c.image_metrics().region_updates, 0);
}

#[test]
fn a_region_becomes_one_revision_that_names_it_and_keeps_identities() {
    let mut c = context();
    let h = live(&mut c, [10, 20, 30, 255]);
    settle(&mut c, h);
    let first = payload(c.draw_data()).clone();
    let id = first.id;
    assert!(c.draw_data().texture_updates.is_empty());
    // Rows of 256 bytes, as a readback has them: only the first 16 bytes of each are pixels.
    let (w, rows) = (4u32, 3u32);
    let mut padded = vec![0xee; 256 * rows as usize];
    for y in 0..rows as usize {
        padded[y * 256..][..16].copy_from_slice(&[1, 2, 3, 255].repeat(w as usize));
    }
    c.update_image_region(h, TextureRegion::new(10, 5, w, rows), &padded, 256)
        .unwrap();
    assert!(c.needs_repaint());
    show(&mut c, h);
    let data = c.draw_data();
    let now = payload(data);
    assert_eq!(now.id, id);
    assert_eq!(now.revision, first.revision + 1);
    assert_eq!(
        data.texture_updates,
        [zaxis::TextureUpdate {
            id,
            base_revision: first.revision,
            to_revision: now.revision,
            regions: vec![TextureRegion::new(10, 5, w, rows)],
        }]
    );
    assert_eq!(texel(now, 10, 5), [1, 2, 3, 255]);
    assert_eq!(texel(now, 13, 7), [1, 2, 3, 255]);
    assert_eq!(texel(now, 9, 5), [10, 20, 30, 255]);
    assert_eq!(texel(now, 14, 5), [10, 20, 30, 255]);
    assert_eq!(texel(now, 10, 8), [10, 20, 30, 255]);
    // The frame the previous draw data shared is untouched.
    assert_eq!(texel(&first, 10, 5), [10, 20, 30, 255]);
    assert_eq!(c.image_state(h).size(), Some(vec2(W as f32, H as f32)));
    assert_eq!(
        h,
        c.load_image(ImageSource::rgba([W, H], frame([0; 4])))
            .map(|_| h)
            .unwrap()
    );
}

#[test]
fn updates_of_one_frame_apply_in_order_and_publish_one_revision() {
    let mut c = context();
    let h = live(&mut c, [0, 0, 0, 255]);
    settle(&mut c, h);
    let revision = payload(c.draw_data()).revision;
    let paint = |c: &mut Context, region, byte| {
        let r: TextureRegion = region;
        let pixels = vec![byte; r.width as usize * r.height as usize * 4];
        c.update_image_region(h, r, &pixels, r.width as usize * 4)
            .unwrap();
    };
    paint(&mut c, TextureRegion::new(0, 0, 8, 8), 1);
    paint(&mut c, TextureRegion::new(4, 4, 8, 8), 2);
    paint(&mut c, TextureRegion::new(12, 0, 4, 4), 3);
    paint(&mut c, TextureRegion::new(40, 20, 2, 2), 4);
    paint(&mut c, TextureRegion::new(4, 4, 2, 2), 5);
    show(&mut c, h);
    let data = c.draw_data();
    let now = payload(data);
    assert_eq!(now.revision, revision + 1);
    assert_eq!(texel(now, 0, 0), [1; 4]);
    assert_eq!(texel(now, 6, 6), [2; 4]);
    assert_eq!(texel(now, 4, 4), [5; 4], "the last update wins");
    assert_eq!(texel(now, 12, 0), [3; 4]);
    assert_eq!(texel(now, 41, 21), [4; 4]);
    assert_eq!(texel(now, 30, 30), [0, 0, 0, 255]);
    let update = &data.texture_updates[0];
    assert_eq!(
        (update.base_revision, update.to_revision),
        (revision, revision + 1)
    );
    // Overlapping and touching rectangles are joined; the far one stays apart.
    assert_eq!(update.regions.len(), 2, "{update:?}");
    let covered = |x, y| {
        update.regions.iter().any(|r| {
            r.fits([W, H]) && x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
        })
    };
    assert!(covered(0, 0) && covered(11, 11) && covered(41, 21));
    // An update after the frame is a new revision.
    paint(&mut c, TextureRegion::new(0, 0, 1, 1), 9);
    show(&mut c, h);
    assert_eq!(payload(c.draw_data()).revision, revision + 2);
    assert_eq!(c.draw_data().texture_updates[0].base_revision, revision + 1);
}

#[test]
fn a_full_update_discards_queued_regions_and_idle_frames_repeat_nothing() {
    let mut c = context();
    let h = live(&mut c, [0, 0, 0, 255]);
    settle(&mut c, h);
    c.update_image_region(h, TextureRegion::new(0, 0, 4, 4), &[7; 64], 16)
        .unwrap();
    c.update_image(h, ImageSource::rgba([W, H], frame([9, 9, 9, 255])))
        .unwrap();
    settle(&mut c, h);
    let data = c.draw_data();
    assert_eq!(texel(payload(data), 0, 0), [9, 9, 9, 255]);
    assert!(data.texture_updates.is_empty());
    // Nothing is left to draw: no repaint, and the revision stays put.
    assert!(!c.needs_repaint());
    let revision = payload(data).revision;
    show(&mut c, h);
    assert_eq!(payload(c.draw_data()).revision, revision);
    assert!(!c.needs_repaint());
}

#[test]
fn the_frame_is_never_copied_in_steady_state_and_memory_is_bounded() {
    let mut c = context();
    let host = Arc::new(frame([0, 0, 0, 255]));
    let h = c
        .load_image(ImageSource::rgba([W, H], host.clone()))
        .unwrap();
    show(&mut c, h);
    let mut model = frame([0, 0, 0, 255]);
    let mut held: Vec<TextureImage> = Vec::new();
    for step in 0..40u32 {
        let y = step % (H - 4);
        let pixels = vec![(step + 1) as u8; 4 * 64 * 4];
        c.update_image_region(h, TextureRegion::new(0, y, 64, 4), &pixels, 256)
            .unwrap();
        model[(y * W * 4) as usize..][..4 * 64 * 4].copy_from_slice(&pixels);
        show(&mut c, h);
        let image = payload(c.draw_data());
        assert_eq!(&image.pixels[..], &model[..], "step {step}");
        // The host's own buffer is never written.
        assert!(host.iter().all(|b| *b == 0 || *b == 255));
        held.push(image.clone());
        if held.len() > 1 {
            held.remove(0);
        }
    }
    let metrics = c.image_metrics();
    assert_eq!(metrics.live_flushes, 40);
    // The first update leaves the host's buffer alone; the second has no spare yet.
    assert!(metrics.live_full_copies <= 2, "{metrics:?}");
    assert!(
        metrics.cpu_resident_bytes <= 3 * (W * H * 4) as usize,
        "{metrics:?}"
    );
    // Without a frame, updates pile up only to a bound.
    for step in 0..500u32 {
        c.update_image_region(h, TextureRegion::new(0, 0, 64, 1), &[step as u8; 256], 256)
            .unwrap();
    }
    let metrics = c.image_metrics();
    assert!(
        metrics.cpu_resident_bytes <= 6 * (W * H * 4) as usize,
        "{metrics:?}"
    );
    assert!(metrics.live_flushes < 40 + 500 / 32);
}

#[test]
fn layouts_reach_the_frame_as_given() {
    let mut c = context();
    for (source, format) in [
        (
            ImageSource::bgra([W, H], frame([1, 2, 3, 4])),
            PixelFormat::Bgra8Srgb,
        ),
        (
            ImageSource::rgba_opaque([W, H], frame([1, 2, 3, 4])),
            PixelFormat::Rgbx8Srgb,
        ),
        (
            ImageSource::bgra_opaque([W, H], frame([1, 2, 3, 4])),
            PixelFormat::Bgrx8Srgb,
        ),
    ] {
        let h = c.load_image(source).unwrap();
        // Ready in the first frame: raw pixels need no worker.
        show(&mut c, h);
        let data = c.draw_data();
        let image = payload(data);
        assert_eq!(
            texel(image, 5, 5),
            [1, 2, 3, 4],
            "uploaded without swapping channels"
        );
        assert_eq!(data.texture_formats.get(&image.id), Some(&format));
        c.update_image_region(h, TextureRegion::new(1, 1, 1, 1), &[9, 8, 7, 6], 4)
            .unwrap();
        show(&mut c, h);
        assert_eq!(texel(payload(c.draw_data()), 1, 1), [9, 8, 7, 6]);
        c.release_image(h);
        show(&mut c, h);
    }
    // Plain RGBA keeps the protocol's default: no entry in the table.
    let h = c
        .load_image(ImageSource::rgba([W, H], frame([1, 2, 3, 4])))
        .unwrap();
    show(&mut c, h);
    while c.image_metrics().pending_jobs > 0 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        show(&mut c, h);
    }
    assert!(c.draw_data().texture_formats.is_empty());
    let _ = TextureId::WHITE;
}

#[test]
fn regions_race_with_loading_and_clearing_the_cache() {
    let mut c = context();
    // Before the first frame, and while the load of a plain RGBA source is in flight.
    let h = c
        .load_image(ImageSource::rgba([W, H], frame([0, 0, 0, 255])))
        .unwrap();
    c.update_image_region(h, TextureRegion::new(2, 2, 2, 2), &[5; 16], 8)
        .unwrap();
    show(&mut c, h);
    for _ in 0..3 {
        show(&mut c, h);
    }
    let image = payload(c.draw_data());
    assert_eq!(texel(image, 2, 2), [5; 4]);
    assert_eq!(texel(image, 0, 0), [0, 0, 0, 255]);
    // Clearing the cache keeps what the updates made, and the next update works again.
    c.clear_image_cache();
    show(&mut c, h);
    show(&mut c, h);
    assert_eq!(texel(payload(c.draw_data()), 2, 2), [5; 4]);
    assert!(
        c.draw_data().texture_updates.is_empty(),
        "no base for the renderer to build on"
    );
    c.update_image_region(h, TextureRegion::new(3, 3, 1, 1), &[6; 4], 4)
        .unwrap();
    show(&mut c, h);
    assert_eq!(texel(payload(c.draw_data()), 3, 3), [6; 4]);
    assert_eq!(texel(payload(c.draw_data()), 2, 2), [5; 4]);
    // Released: a region has nothing to land in.
    c.release_image(h);
    assert!(c
        .update_image_region(h, TextureRegion::new(0, 0, 1, 1), &[0; 4], 4)
        .is_err());
}

#[test]
fn a_region_that_is_the_whole_frame_is_adopted_without_another_copy() {
    let mut c = context();
    let h = c
        .load_image(ImageSource::rgba([W, H], frame([0, 0, 0, 255])).live())
        .unwrap();
    settle(&mut c, h);
    let whole = Arc::new(frame([4, 5, 6, 255]));
    c.update_image_region_shared(
        h,
        TextureRegion::whole([W, H]),
        whole.clone(),
        0,
        (W * 4) as usize,
    )
    .unwrap();
    show(&mut c, h);
    let data = c.draw_data();
    assert!(
        Arc::ptr_eq(&payload(data).pixels, &whole),
        "the host's buffer is the frame"
    );
    assert_eq!(texel(payload(data), 7, 7), [4, 5, 6, 255]);
    assert_eq!(c.image_metrics().live_full_copies, 0);
    // Patches after it land on a copy; the host's buffer is never written.
    c.update_image_region_shared(
        h,
        TextureRegion::whole([W, H]),
        whole.clone(),
        0,
        (W * 4) as usize,
    )
    .unwrap();
    c.update_image_region(h, TextureRegion::new(1, 1, 1, 1), &[9, 9, 9, 9], 4)
        .unwrap();
    show(&mut c, h);
    assert_eq!(texel(payload(c.draw_data()), 1, 1), [9; 4]);
    assert_eq!(whole[(W * 4 + 4) as usize], 4);
}
