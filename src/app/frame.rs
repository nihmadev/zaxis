//! The per-window view an update receives.

use super::{AppStats, OpenOutcome, WindowInfo, WindowKey, WindowOptions, WindowStatus, Windows};
use std::sync::Arc;
use winit::window::Window;

/// The window an update is building, application-level window management, and shutdown.
///
/// `update` is called once per redraw of one window; [`window_key`](Self::window_key) says
/// which, and the [`Context`](crate::Context) passed alongside is that window's alone. Draw
/// each window's content from your own data keyed by that window.
pub struct Frame<'a> {
    pub(crate) window: &'a Arc<Window>,
    pub(crate) key: &'a WindowKey,
    pub(crate) info: WindowInfo,
    pub(crate) windows: Windows<'a>,
    pub(crate) device_resets: u64,
    pub(crate) device_loss: Option<&'a str>,
}

impl<'a> Frame<'a> {
    /// Native window of this frame for changing the title, requesting redraws, or cloning
    /// its Arc into a background worker that needs to wake the sleeping event loop.
    pub fn window(&self) -> &Arc<Window> {
        self.window
    }

    /// The key of the window being drawn: [`WindowKey::main`] in a single-window application.
    pub fn window_key(&self) -> &WindowKey {
        self.key
    }

    /// Whether this is the main window.
    pub fn is_main(&self) -> bool {
        self.windows.main_key() == self.key
    }

    /// Size, scale factor and state of this window at the start of the frame.
    pub fn info(&self) -> &WindowInfo {
        &self.info
    }

    /// Open, close, focus and inspect other windows; see [`Windows`].
    pub fn windows(&mut self) -> &mut Windows<'a> {
        &mut self.windows
    }

    /// Ask for a secondary window; sugar for [`Windows::open`].
    pub fn open_window(&mut self, key: impl Into<WindowKey>, options: WindowOptions) -> OpenOutcome {
        self.windows.open(key, options)
    }

    /// Close one window now, without asking the application; see [`Windows::close`].
    pub fn close_window(&mut self, key: impl Into<WindowKey>) {
        self.windows.close(key);
    }

    /// Close this window now, without asking: the way to confirm a close you rejected.
    pub fn close_this_window(&mut self) {
        self.windows.close(self.key.clone());
    }

    /// Ask to close this window as the user would, so [`App::close_requested`](super::App::close_requested)
    /// decides. Use it for an in-window close button that must honour the same confirmation.
    pub fn request_close(&mut self) {
        self.windows.request_close(self.key.clone());
    }

    /// Where a requested window is, or `None` for a key that was never requested or has closed.
    pub fn window_status(&self, key: &WindowKey) -> Option<&WindowStatus> {
        self.windows.status(key)
    }

    /// Counters over all windows.
    pub fn stats(&self) -> AppStats {
        self.windows.stats()
    }

    /// How many times the runner has recreated the GPU renderer after a lost device.
    /// Recovery is automatic and covers every window; compare with the previous value to notice it.
    pub fn device_resets(&self) -> u64 {
        self.device_resets
    }

    /// The reason the GPU device was last lost, if it ever was.
    pub fn last_device_loss(&self) -> Option<&str> {
        self.device_loss
    }

    /// End the application after the current frame has been presented successfully,
    /// closing every window. To close only this window use [`close_this_window`](Self::close_this_window).
    pub fn close(&mut self) {
        self.windows.control.exit_after_present = true;
    }
}
