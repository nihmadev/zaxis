//! The frame driver: runs the interface and draws it through a backend, once per host present.
//!
//! A [`Context`] is not `Send`, so it lives in a thread-local of the thread that presents (the
//! host's render thread) and never leaves it. Other threads reach it only through the input
//! queue and atomics in [`Core`](crate::core::Core).

use crate::{
    backend::{BackendError, PresentBackend, SurfaceInfo},
    budget::Budget,
    core::{Core, UiFn},
    stats::Counters,
};
use std::{
    cell::RefCell,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{atomic::Ordering::Relaxed, Arc},
    time::Duration,
};
use zaxis::{winit::event::ElementState, Context, FontFamily, InputEvent};

/// What one present of the host came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameOutcome {
    /// The interface was drawn into the frame; `ui_pass` says whether it was rebuilt first.
    Drawn { ui_pass: bool },
    /// The overlay is hidden; the frame was left alone.
    Hidden,
    /// The frame was left alone for the given reason.
    Skipped(SkipReason),
    /// The overlay is disabled; see [`Overlay::disabled_reason`](crate::Overlay::disabled_reason).
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// A frame over budget is being paid back.
    Budget,
    /// The backend is not ready yet or could not draw this frame.
    Backend,
    /// A present arrived while a frame was running on this thread.
    Reentrant,
    /// The interface belongs to another thread.
    ForeignThread,
}

pub(crate) struct Driver {
    core: Arc<Core>,
    context: Context,
    ui: Option<UiFn>,
    in_frame: bool,
    shown: bool,
    viewport: Option<([u32; 2], f64)>,
    force_pass: bool,
    budget: Budget,
    pointer_consumed: bool,
}

thread_local! {
    static DRIVER: RefCell<Option<Driver>> = const { RefCell::new(None) };
}

fn new_context(core: &Core) -> Context {
    let options = &core.options;
    match (&options.fonts, &options.monospace) {
        (Some(fonts), monospace) => Context::with_font_families(fonts.clone(), monospace.clone()),
        (None, Some(monospace)) => {
            Context::with_font_families(FontFamily::inter(), Some(monospace.clone()))
        }
        (None, None) => Context::new(),
    }
}

impl Driver {
    fn new(core: Arc<Core>, ui: UiFn) -> Self {
        Self {
            context: new_context(&core),
            budget: Budget::new(core.options.frame_budget),
            core,
            ui: Some(ui),
            in_frame: false,
            shown: false,
            viewport: None,
            force_pass: true,
            pointer_consumed: false,
        }
    }

    /// Visibility changes reach the interface as focus: hiding releases held keys and
    /// buttons and closes popups, showing restores the modifiers and the pointer.
    fn sync_visibility(&mut self) {
        let visible = self.core.is_visible();
        if visible == self.shown {
            return;
        }
        self.shown = visible;
        self.force_pass = true;
        if visible {
            self.context
                .on_input(InputEvent::Modifiers(self.core.hub.modifiers()));
            self.context.on_input(InputEvent::Focus(true));
            if let Some((x, y)) = self.core.hub.pointer() {
                self.context.on_input(InputEvent::PointerMoved { x, y });
            }
        } else {
            self.core.hub.drain();
            self.core.hub.forget_held();
            self.context.on_input(InputEvent::Focus(false));
        }
    }

    fn apply(&mut self, event: InputEvent) -> bool {
        let response = self.context.on_input(event.clone());
        self.force_pass |= response.repaint;
        match event {
            InputEvent::PointerMoved { .. } => self.pointer_consumed = response.consumed,
            InputEvent::Button {
                state: ElementState::Released,
                ..
            } if !response.consumed => {
                self.pointer_consumed = false;
            }
            _ => {}
        }
        response.consumed
    }

    fn apply_queue(&mut self) {
        for event in self.core.hub.drain() {
            self.apply(event);
        }
    }

    fn publish_policy(&self) {
        let policy = &self.core.hub.policy;
        policy.pointer_over_ui.store(self.pointer_consumed, Relaxed);
        let area = self.context.ime_cursor_area();
        policy.keyboard_focus.store(area.is_some(), Relaxed);
        policy.cursor.store(
            crate::win32::cursor_resource(self.context.cursor_icon()),
            Relaxed,
        );
        let scale = self.context.scale_factor();
        *crate::core::lock(&policy.ime_area) = area.map(|r| {
            [
                r.min.x * scale,
                r.min.y * scale,
                r.size().x * scale,
                r.size().y * scale,
            ]
        });
    }

    fn update_viewport(&mut self, info: SurfaceInfo) {
        let scale = self
            .core
            .options
            .scale_factor
            .or(info.scale_factor)
            .filter(|scale| scale.is_finite() && *scale > 0.0)
            .unwrap_or(1.0);
        if self.viewport == Some((info.size, scale)) {
            return;
        }
        let scale_changed = self.viewport.is_none_or(|(_, old)| old != scale);
        if scale_changed {
            self.context.on_input(InputEvent::ScaleFactor(scale));
        }
        self.context.on_input(InputEvent::Resized {
            width: info.size[0],
            height: info.size[1],
        });
        self.viewport = Some((info.size, scale));
        self.force_pass = true;
    }

    fn frame(&mut self, backend: &mut dyn PresentBackend) -> FrameOutcome {
        let core = Arc::clone(&self.core);
        let counters = &core.counters;
        Counters::bump(&counters.presents);
        if core.is_disabled() {
            return FrameOutcome::Disabled;
        }
        self.sync_visibility();
        if !self.shown {
            Counters::bump(&counters.skipped_hidden);
            return FrameOutcome::Hidden;
        }
        self.apply_queue();
        if self.budget.should_skip() {
            Counters::bump(&counters.skipped_budget);
            return FrameOutcome::Skipped(SkipReason::Budget);
        }
        let started = core.now();
        let info = match backend.begin() {
            Ok(info) => info,
            Err(error) => return self.backend_failed(error),
        };
        self.update_viewport(info);
        let mut ui_pass = false;
        if self.force_pass
            || self.context.needs_repaint_at(started)
            || self.context.wants_animation_frame()
        {
            self.force_pass = false;
            ui_pass = true;
            let Some(ui) = self.ui.as_mut() else {
                let _ = backend.end();
                core.disable("the interface closure is gone");
                return FrameOutcome::Disabled;
            };
            let context = &mut self.context;
            let result = catch_unwind(AssertUnwindSafe(|| context.run_at(started, |c| ui(c))));
            if let Err(payload) = result {
                let _ = backend.end();
                core.disable(format!(
                    "the interface panicked: {}",
                    panic_message(&payload)
                ));
                return FrameOutcome::Disabled;
            }
            Counters::bump(&counters.ui_passes);
        }
        let ui_done = core.now();
        let rendered = catch_unwind(AssertUnwindSafe(|| {
            backend.render(self.context.draw_data())
        }));
        let rendered = match rendered {
            Ok(rendered) => rendered,
            Err(payload) => {
                let _ = catch_unwind(AssertUnwindSafe(|| backend.end()));
                core.disable(format!("drawing panicked: {}", panic_message(&payload)));
                return FrameOutcome::Disabled;
            }
        };
        let ended = backend.end();
        let finished = core.now();
        counters
            .last_ui_micros
            .store(micros(ui_done.saturating_duration_since(started)), Relaxed);
        counters
            .last_render_micros
            .store(micros(finished.saturating_duration_since(ui_done)), Relaxed);
        self.publish_policy();
        if self
            .budget
            .record(finished.saturating_duration_since(started))
        {
            Counters::bump(&counters.over_budget);
        }
        if let Err(error) = rendered.and(ended) {
            return self.backend_failed(error);
        }
        Counters::bump(&counters.frames_drawn);
        FrameOutcome::Drawn { ui_pass }
    }

    fn backend_failed(&mut self, error: BackendError) -> FrameOutcome {
        let counters = &self.core.counters;
        match error {
            BackendError::NotReady => {}
            BackendError::Frame(message) => {
                Counters::bump(&counters.backend_errors);
                log::debug!("z-hook: {message}");
            }
            BackendError::Lost(message) => {
                Counters::bump(&counters.backend_errors);
                self.viewport = None;
                log::info!("z-hook: target lost: {message}");
            }
            BackendError::Unsupported(message) => {
                Counters::bump(&counters.backend_errors);
                self.core.disable(message);
                return FrameOutcome::Disabled;
            }
        }
        Counters::bump(&counters.skipped_backend);
        FrameOutcome::Skipped(SkipReason::Backend)
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        if let Some(ui) = self.ui.take() {
            self.core.return_ui(ui);
        }
    }
}

fn micros(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

pub(crate) fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
}

/// Run one host present through the thread's driver, creating it on the first present.
pub(crate) fn run_frame(core: &Arc<Core>, backend: &mut dyn PresentBackend) -> FrameOutcome {
    DRIVER.with(|cell| {
        let Ok(mut slot) = cell.try_borrow_mut() else {
            Counters::bump(&core.counters.skipped_reentrant);
            return FrameOutcome::Skipped(SkipReason::Reentrant);
        };
        if slot
            .as_ref()
            .is_some_and(|driver| !Arc::ptr_eq(&driver.core, core))
        {
            *slot = None;
        }
        if slot.is_none() {
            let Some(ui) = core.take_ui() else {
                Counters::bump(&core.counters.presents);
                Counters::bump(&core.counters.skipped_foreign_thread);
                return FrameOutcome::Skipped(SkipReason::ForeignThread);
            };
            *slot = Some(Driver::new(Arc::clone(core), ui));
        }
        let driver = slot.as_mut().expect("driver was just created");
        driver.in_frame = true;
        let outcome = catch_unwind(AssertUnwindSafe(|| driver.frame(backend)));
        driver.in_frame = false;
        match outcome {
            Ok(outcome) => outcome,
            Err(payload) => {
                core.disable(format!("the overlay panicked: {}", panic_message(&payload)));
                FrameOutcome::Disabled
            }
        }
    })
}

/// If this thread owns the interface and is not inside a frame, give it `event` now and say
/// whether it consumed the event.
pub(crate) fn dispatch_now(core: &Arc<Core>, event: &InputEvent) -> Option<bool> {
    DRIVER.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        let driver = slot
            .as_mut()
            .filter(|d| Arc::ptr_eq(&d.core, core) && !d.in_frame)?;
        driver.sync_visibility();
        driver.apply_queue();
        let consumed = driver.apply(event.clone());
        driver.publish_policy();
        Some(consumed)
    })
}

/// Drop this thread's driver, if it belongs to `core` (the overlay was uninstalled).
pub(crate) fn release_thread(core: &Arc<Core>) {
    DRIVER.with(|cell| {
        if let Ok(mut slot) = cell.try_borrow_mut() {
            if slot
                .as_ref()
                .is_some_and(|driver| Arc::ptr_eq(&driver.core, core))
            {
                *slot = None;
            }
        }
    });
}

/// Run the overlay's part of a hooked call so that nothing unwinds into the host: a panic
/// disables the overlay and is logged.
#[cfg(windows)]
pub(crate) fn guard_frame(what: &str, body: impl FnOnce()) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(body)) {
        let message = format!("panic in {what}: {}", panic_message(&payload));
        match crate::registry::current() {
            Some(core) => core.disable(message),
            None => log::error!("z-hook: {message}"),
        }
    }
}
