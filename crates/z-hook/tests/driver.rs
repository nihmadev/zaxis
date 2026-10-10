//! The frame driver against a scripted backend: call order, rest, panics, budget, threads.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc, Mutex,
    },
    time::Duration,
};
use z_hook::{
    testing::{Call, FakeClock, MockBackend},
    BackendError, FrameOutcome, Overlay, OverlayOptions, SkipReason,
};
use zaxis::{Context, Window};

fn options() -> OverlayOptions {
    OverlayOptions::default().frame_budget(None)
}

fn counting(options: OverlayOptions) -> (Overlay, Arc<AtomicUsize>) {
    let passes = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&passes);
    let overlay = Overlay::detached(options, move |ctx: &mut Context| {
        counter.fetch_add(1, SeqCst);
        Window::new("Test").show(ctx, |ui| {
            ui.label("overlay");
        });
    });
    (overlay, passes)
}

#[test]
fn a_frame_runs_begin_render_end_in_order() {
    let (overlay, passes) = counting(options());
    let mut backend = MockBackend::new([800, 600]);
    assert_eq!(
        overlay.present(&mut backend),
        FrameOutcome::Drawn { ui_pass: true }
    );
    assert_eq!(passes.load(SeqCst), 1);
    assert!(matches!(
        backend.calls.as_slice(),
        [Call::Begin, Call::Render { geometry: true, .. }, Call::End]
    ));
}

#[test]
fn a_resting_interface_is_composited_but_not_rebuilt() {
    let (overlay, passes) = counting(options());
    let mut backend = MockBackend::new([800, 600]);
    overlay.present(&mut backend);
    backend.clear();
    for _ in 0..5 {
        assert_eq!(
            overlay.present(&mut backend),
            FrameOutcome::Drawn { ui_pass: false }
        );
    }
    assert_eq!(passes.load(SeqCst), 1, "no redraw while nothing changes");
    assert_eq!(backend.renders(), 5, "the host's frame is new each time");
    let revisions: Vec<_> = backend
        .calls
        .iter()
        .filter_map(|call| match call {
            Call::Render { revision, .. } => Some(*revision),
            _ => None,
        })
        .collect();
    assert!(
        revisions.windows(2).all(|pair| pair[0] == pair[1]),
        "same draw data, no re-upload"
    );
    let stats = overlay.stats();
    assert_eq!(
        (stats.presents, stats.frames_drawn, stats.ui_passes),
        (6, 6, 1)
    );
}

#[test]
fn a_hidden_overlay_leaves_the_frame_alone() {
    let (overlay, passes) = counting(options().start_visible(false));
    let mut backend = MockBackend::new([800, 600]);
    assert_eq!(overlay.present(&mut backend), FrameOutcome::Hidden);
    assert!(backend.calls.is_empty() && passes.load(SeqCst) == 0);
    overlay.set_visible(true);
    assert_eq!(
        overlay.present(&mut backend),
        FrameOutcome::Drawn { ui_pass: true }
    );
    assert!(!overlay.toggle());
    assert_eq!(overlay.present(&mut backend), FrameOutcome::Hidden);
    assert_eq!(overlay.stats().skipped_hidden, 2);
}

#[test]
fn size_and_dpi_reach_the_interface() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let overlay = Overlay::detached(options(), move |ctx: &mut Context| {
        sink.lock()
            .unwrap()
            .push((ctx.viewport().size(), ctx.scale_factor()));
        Window::new("A").show(ctx, |ui| {
            ui.label("x");
        });
    });
    let mut backend = MockBackend::new([1600, 900]);
    backend.scale_factor = Some(2.0);
    overlay.present(&mut backend);
    backend.size = [1280, 720];
    backend.scale_factor = Some(1.0);
    assert_eq!(
        overlay.present(&mut backend),
        FrameOutcome::Drawn { ui_pass: true }
    );
    assert_eq!(
        *seen.lock().unwrap(),
        [
            (zaxis::vec2(800.0, 450.0), 2.0),
            (zaxis::vec2(1280.0, 720.0), 1.0)
        ]
    );
}

#[test]
fn the_option_overrides_the_platform_scale() {
    let seen = Arc::new(Mutex::new(0.0));
    let sink = Arc::clone(&seen);
    let overlay = Overlay::detached(options().scale_factor(1.5), move |ctx: &mut Context| {
        *sink.lock().unwrap() = ctx.scale_factor();
    });
    let mut backend = MockBackend::new([300, 300]);
    backend.scale_factor = Some(3.0);
    overlay.present(&mut backend);
    assert_eq!(*seen.lock().unwrap(), 1.5);
}

#[test]
fn a_panicking_interface_disables_the_overlay_and_the_host_goes_on() {
    let overlay = Overlay::detached(options(), |_: &mut Context| panic!("boom"));
    let mut backend = MockBackend::new([100, 100]);
    assert_eq!(overlay.present(&mut backend), FrameOutcome::Disabled);
    assert_eq!(
        backend.calls,
        [Call::Begin, Call::End],
        "the backbuffer is restored"
    );
    assert!(overlay.disabled_reason().unwrap().contains("boom"));
    backend.clear();
    assert_eq!(overlay.present(&mut backend), FrameOutcome::Disabled);
    assert!(
        backend.calls.is_empty(),
        "a disabled overlay does not touch the frame"
    );
}

#[test]
fn a_panicking_backend_is_restored_and_disables_the_overlay() {
    let (overlay, _) = counting(options());
    let mut backend = MockBackend::new([100, 100]);
    backend.panic_in_render = true;
    assert_eq!(overlay.present(&mut backend), FrameOutcome::Disabled);
    assert_eq!(backend.calls.last(), Some(&Call::End));
    assert!(overlay.disabled_reason().is_some());
}

#[test]
fn backend_errors_decide_between_skipping_and_disabling() {
    let (overlay, _) = counting(options());
    let mut backend = MockBackend::new([100, 100]);
    backend.begin_error = Some(BackendError::NotReady);
    assert_eq!(
        overlay.present(&mut backend),
        FrameOutcome::Skipped(SkipReason::Backend)
    );
    assert_eq!(
        overlay.stats().backend_errors,
        0,
        "not ready is not an error"
    );
    backend.begin_error = None;
    backend.render_error = Some(BackendError::Frame("busy".into()));
    assert_eq!(
        overlay.present(&mut backend),
        FrameOutcome::Skipped(SkipReason::Backend)
    );
    assert_eq!(
        backend.calls.last(),
        Some(&Call::End),
        "end follows a failed render"
    );
    backend.render_error = Some(BackendError::Lost("resized".into()));
    overlay.present(&mut backend);
    backend.render_error = None;
    assert_eq!(
        overlay.present(&mut backend),
        FrameOutcome::Drawn { ui_pass: true },
        "a lost target is rebuilt, not fatal"
    );
    backend.render_error = Some(BackendError::Unsupported("no format".into()));
    assert_eq!(overlay.present(&mut backend), FrameOutcome::Disabled);
    assert_eq!(overlay.disabled_reason().as_deref(), Some("no format"));
}

#[test]
fn a_frame_over_budget_makes_the_next_ones_pass_through() {
    let clock = FakeClock::new();
    let options = OverlayOptions::default().frame_budget(Some(Duration::from_millis(4)));
    let overlay = Overlay::with_clock(options, |_: &mut Context| {}, clock.as_clock());
    let mut backend = MockBackend::new([100, 100]);
    backend.clock = Some(clock);
    backend.render_cost = Duration::from_millis(14);
    assert!(matches!(
        overlay.present(&mut backend),
        FrameOutcome::Drawn { .. }
    ));
    backend.render_cost = Duration::ZERO;
    for _ in 0..3 {
        assert_eq!(
            overlay.present(&mut backend),
            FrameOutcome::Skipped(SkipReason::Budget)
        );
    }
    assert!(matches!(
        overlay.present(&mut backend),
        FrameOutcome::Drawn { .. }
    ));
    let stats = overlay.stats();
    assert_eq!((stats.over_budget, stats.skipped_budget), (1, 3));
}

#[test]
fn a_present_inside_a_frame_is_skipped() {
    let (overlay, _) = counting(options());
    let overlay = Arc::new(overlay);
    let inner = Arc::clone(&overlay);
    let outcome = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&outcome);
    let mut backend = MockBackend::new([100, 100]);
    backend.during_render = Some(Box::new(move || {
        let mut nested = MockBackend::new([100, 100]);
        *sink.lock().unwrap() = Some(inner.present(&mut nested));
        assert!(nested.calls.is_empty());
    }));
    assert!(matches!(
        overlay.present(&mut backend),
        FrameOutcome::Drawn { .. }
    ));
    assert_eq!(
        *outcome.lock().unwrap(),
        Some(FrameOutcome::Skipped(SkipReason::Reentrant))
    );
}

#[test]
fn another_thread_cannot_present_the_interface_of_this_one() {
    let (overlay, _) = counting(options());
    let overlay = Arc::new(overlay);
    overlay.present(&mut MockBackend::new([100, 100]));
    let other = Arc::clone(&overlay);
    let outcome = std::thread::spawn(move || other.present(&mut MockBackend::new([100, 100])))
        .join()
        .unwrap();
    assert_eq!(outcome, FrameOutcome::Skipped(SkipReason::ForeignThread));
    assert!(matches!(
        overlay.present(&mut MockBackend::new([100, 100])),
        FrameOutcome::Drawn { .. }
    ));
}

#[test]
fn removing_the_overlay_during_a_frame_finishes_the_frame() {
    let (overlay, _) = counting(options());
    let overlay = Arc::new(overlay);
    let inner = Arc::clone(&overlay);
    let mut backend = MockBackend::new([100, 100]);
    backend.during_render = Some(Box::new(move || inner.disable("removed")));
    overlay.present(&mut backend);
    assert_eq!(backend.calls.last(), Some(&Call::End));
    backend.clear();
    assert_eq!(overlay.present(&mut backend), FrameOutcome::Disabled);
    assert!(backend.calls.is_empty());
}
