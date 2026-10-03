use std::{
    sync::{mpsc, Arc},
    time::Duration,
};
use zaxis::{
    vec2, winit, Context, DecodedImage, Image, ImageDecoder, ImageError, ImageLimits, ImageSource,
    ImageState, Window,
};

fn pass(c: &mut Context, source: &ImageSource) {
    c.run(|c| {
        Window::new("memory").show(c, |ui| {
            ui.add(Image::new(source).size(vec2(32.0, 32.0)));
        });
    });
}
fn settle(c: &mut Context, source: &ImageSource, rx: &mpsc::Receiver<()>) {
    loop {
        pass(c, source);
        match c.image_state(source) {
            ImageState::Ready { .. } if c.image_metrics().pending_jobs == 0 => break,
            ImageState::Error(e) => panic!("{e}"),
            _ => {
                rx.recv_timeout(Duration::from_secs(10)).unwrap();
            }
        }
    }
}
#[test]
fn long_gallery_eviction_releases_old_buffers_and_reloads() {
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(800, 600), 1.0);
    let mut limits = c.image_limits().clone();
    limits.cpu_cache_bytes = 512 << 10;
    limits.max_entries = 16;
    c.set_image_limits(limits);
    let (tx, rx) = mpsc::channel();
    c.set_image_waker(move || {
        let _ = tx.send(());
    });
    let pixels = Arc::new(vec![255; 128 * 128 * 4]);
    let old = Arc::downgrade(&pixels);
    let first = ImageSource::rgba([128, 128], pixels);
    settle(&mut c, &first, &rx);
    drop(first);
    for i in 0..512 {
        let source = ImageSource::rgba([128, 128], vec![i as u8; 128 * 128 * 4]);
        settle(&mut c, &source, &rx);
        assert!(c.image_metrics().cpu_resident_bytes <= 512 << 10);
        assert!(!c.needs_repaint());
    }
    assert!(
        old.upgrade().is_none(),
        "old pixels were retained after eviction"
    );
    assert!(c.image_metrics().evictions > 400);
    let source = ImageSource::rgba([128, 128], vec![255; 128 * 128 * 4]);
    settle(&mut c, &source, &rx);
    assert!(c.image_state(&source).is_ready());
}

#[test]
fn repeated_handle_replacement_does_not_retain_previous_generations() {
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(800, 600), 1.0);
    let (tx, rx) = mpsc::channel();
    c.set_image_waker(move || {
        let _ = tx.send(());
    });
    let h = c
        .load_image(ImageSource::rgba([128, 128], vec![255; 128 * 128 * 4]))
        .unwrap();
    let source = ImageSource::from(h);
    settle(&mut c, &source, &rx);
    for i in 0..256 {
        let pixels = Arc::new(vec![i as u8; 128 * 128 * 4]);
        let weak = Arc::downgrade(&pixels);
        c.update_image(h, ImageSource::rgba([128, 128], pixels))
            .unwrap();
        settle(&mut c, &source, &rx);
        c.update_image(h, ImageSource::rgba([128, 128], vec![0; 128 * 128 * 4]))
            .unwrap();
        settle(&mut c, &source, &rx);
        assert!(
            weak.upgrade().is_none(),
            "replaced generation retained its pixels"
        );
        assert!(c.image_metrics().cpu_resident_bytes < 128 << 10);
    }
    c.release_image(h);
    c.run(|_| {});
    assert_eq!(c.image_metrics().cpu_resident_bytes, 0);
}

#[test]
fn explicit_versions_share_storage_but_never_old_decoded_pixels() {
    let mut c = Context::new();
    let source = ImageSource::encoded(vec![1, 2, 3]);
    let h0 = c.load_image(&source).unwrap();
    assert_eq!(h0, c.load_image(&source).unwrap());
    let h1 = c.load_image(source.clone().with_revision(1)).unwrap();
    assert_ne!(h0, h1);
    assert_eq!(h1, c.load_image(source.with_revision(1)).unwrap());
}

#[test]
fn synchronous_limit_error_requests_one_final_state_pass() {
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(800, 600), 1.0);
    let mut limits = c.image_limits().clone();
    limits.max_dimension = 16;
    c.set_image_limits(limits);
    let source = ImageSource::rgba([32, 16], vec![255; 32 * 16 * 4]);
    pass(&mut c, &source);
    assert!(matches!(c.image_state(&source), ImageState::Error(_)));
    assert!(c.needs_repaint());
    assert_eq!(c.image_metrics().pending_jobs, 0);
    pass(&mut c, &source);
    assert!(!c.needs_repaint());
    assert!(c.next_repaint().is_none());
}

#[test]
fn dropping_context_cancels_host_wake_and_releases_running_job() {
    struct Decoder {
        started: mpsc::Sender<()>,
        gate: std::sync::Mutex<mpsc::Receiver<()>>,
        released: mpsc::Sender<()>,
    }
    impl ImageDecoder for Decoder {
        fn decode(
            &self,
            bytes: &[u8],
            _: &ImageLimits,
        ) -> Option<Result<DecodedImage, ImageError>> {
            if bytes != b"drop-test" {
                return None;
            }
            self.started.send(()).unwrap();
            self.gate.lock().unwrap().recv().unwrap();
            Some(Ok(DecodedImage {
                size: [1, 1],
                pixels: Arc::new(vec![255; 4]),
            }))
        }
    }
    impl Drop for Decoder {
        fn drop(&mut self) {
            let _ = self.released.send(());
        }
    }
    let (started_tx, started_rx) = mpsc::channel();
    let (gate_tx, gate_rx) = mpsc::channel();
    let (released_tx, released_rx) = mpsc::channel();
    let (wake_tx, wake_rx) = mpsc::channel();
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(800, 600), 1.0);
    c.set_image_waker(move || {
        let _ = wake_tx.send(());
    });
    c.add_image_decoder(Arc::new(Decoder {
        started: started_tx,
        gate: std::sync::Mutex::new(gate_rx),
        released: released_tx,
    }));
    let source = ImageSource::bytes(b"drop-test");
    pass(&mut c, &source);
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    drop(c); // Must return before the blocked codec is released.
    assert!(matches!(
        wake_rx.try_recv(),
        Err(mpsc::TryRecvError::Disconnected)
    ));
    gate_tx.send(()).unwrap();
    released_rx.recv_timeout(Duration::from_secs(10)).unwrap();
}
