//! The public handle: install the overlay, show and hide it, feed it input, read its counters.

use crate::{
    backend::PresentBackend,
    core::{Clock, Core},
    driver::{self, FrameOutcome},
    error::HookError,
    input::Delivery,
    options::{Api, OverlayOptions},
    registry,
    stats::OverlayStats,
};
use std::{sync::Arc, time::Instant};
use zaxis::{Context, InputEvent};

/// An interface drawn over the frames of the process it lives in.
///
/// [`install`](Self::install) registers the interface and arms the hooks for the requested
/// graphics APIs; from then on every frame the host presents goes through the overlay,
/// which draws the interface over it and passes input to it according to
/// [`OverlayInput`](crate::OverlayInput). Dropping the value, or
/// [`uninstall`](Self::uninstall), removes the hooks and stops all work.
///
/// Nothing here can fail the host: errors while running disable the overlay and are logged
/// (see [`disabled_reason`](Self::disabled_reason)).
pub struct Overlay {
    core: Arc<Core>,
    installed: bool,
}

impl Overlay {
    /// Install the overlay in this process. `ui` runs on the host's render thread, once per
    /// interface pass, and builds the interface like an `App::update` does.
    ///
    /// Call it from a thread you own, never from `DllMain`: placing hooks takes locks and
    /// loads libraries, which the loader lock forbids there.
    pub fn install(
        options: OverlayOptions,
        ui: impl FnMut(&mut Context) + Send + 'static,
    ) -> Result<Self, HookError> {
        validate(&options)?;
        let overlay = Self::detached(options, ui);
        registry::register(&overlay.core)?;
        let mut overlay = overlay;
        overlay.installed = true;
        if let Err(error) = registry::arm(&overlay.core) {
            overlay.uninstall();
            return Err(error);
        }
        Ok(overlay)
    }

    /// An overlay that hooks nothing: the owner of a custom [`PresentBackend`] drives it with
    /// [`present`](Self::present). Also what tests use.
    pub fn detached(
        options: OverlayOptions,
        ui: impl FnMut(&mut Context) + Send + 'static,
    ) -> Self {
        Self::with_clock(options, ui, Arc::new(Instant::now))
    }

    #[doc(hidden)]
    pub fn with_clock(
        options: OverlayOptions,
        ui: impl FnMut(&mut Context) + Send + 'static,
        clock: Clock,
    ) -> Self {
        Self {
            core: Core::new(options, Box::new(ui), clock),
            installed: false,
        }
    }

    /// Switch the overlay off for good, as if it had failed: frames pass through untouched.
    /// Safe to call from the interface closure or a backend, mid-frame.
    pub fn disable(&self, reason: &str) {
        self.core.disable(reason);
    }

    /// Draw the interface over one frame through `backend`. Hooks call this on every present;
    /// call it yourself only with a [`detached`](Self::detached) overlay.
    pub fn present(&self, backend: &mut dyn PresentBackend) -> FrameOutcome {
        driver::run_frame(&self.core, backend)
    }

    pub fn options(&self) -> &OverlayOptions {
        &self.core.options
    }

    pub fn is_visible(&self) -> bool {
        self.core.is_visible()
    }

    pub fn set_visible(&self, visible: bool) {
        self.core.set_visible(visible);
    }

    /// Flip visibility; returns the new state.
    pub fn toggle(&self) -> bool {
        let visible = !self.core.is_visible();
        self.core.set_visible(visible);
        visible
    }

    /// Why the overlay switched itself off, if it did.
    pub fn disabled_reason(&self) -> Option<String> {
        self.core.disabled_reason()
    }

    pub fn stats(&self) -> OverlayStats {
        self.core.stats()
    }

    /// A cloneable, `Send` handle that feeds input to the overlay from any thread: the
    /// source for hosts whose window events the hooks cannot see.
    pub fn input(&self) -> InputSink {
        InputSink(Arc::clone(&self.core))
    }

    /// Remove the hooks and disable the overlay. The host keeps running; frames in flight
    /// finish. Called by `Drop`.
    pub fn uninstall(&mut self) {
        if !std::mem::take(&mut self.installed) {
            self.core.disable("uninstalled");
            return;
        }
        self.core.disable("uninstalled");
        registry::disarm(&self.core);
        driver::release_thread(&self.core);
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        self.uninstall();
    }
}

/// Feeds input events to the overlay; see [`Overlay::input`].
#[derive(Clone)]
pub struct InputSink(Arc<Core>);

impl InputSink {
    /// Give the overlay an event, in the host window's physical pixels. The answer says
    /// whether the host should also get the event, per the overlay's
    /// [`OverlayInput`](crate::OverlayInput).
    pub fn send(&self, event: InputEvent) -> Delivery {
        self.0.route(event)
    }
}

fn validate(options: &OverlayOptions) -> Result<(), HookError> {
    if options.apis.is_empty() {
        return Err(HookError::InvalidOptions(
            "no graphics API requested".into(),
        ));
    }
    if !options.apis.iter().any(Api::available) {
        return Err(HookError::NoSupportedApi);
    }
    if options
        .scale_factor
        .is_some_and(|s| !s.is_finite() || s <= 0.0)
    {
        return Err(HookError::InvalidOptions(
            "scale factor must be positive".into(),
        ));
    }
    Ok(())
}

/// The input handle of the overlay installed in this process, if any: the way to feed input to
/// an overlay that a layer or a hook created (it never returns its `Overlay`).
pub fn input_sink() -> Option<InputSink> {
    registry::current().map(InputSink)
}
