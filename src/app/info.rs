//! Window state, status, errors and counters reported to the application.

use crate::{RenderError, Vec2};
use std::{error::Error, fmt};
use winit::dpi::PhysicalSize;

/// A snapshot of one native window, taken when the callback that reads it started.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowInfo {
    /// Inner (client area) size in physical pixels.
    pub size: PhysicalSize<u32>,
    /// Inner size in logical pixels, the coordinates widgets use.
    pub logical_size: Vec2,
    /// Physical pixels per logical pixel on the monitor the window is on now.
    pub scale_factor: f64,
    pub focused: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    /// Covered by other windows or otherwise not shown, as reported by the system.
    pub occluded: bool,
    /// Not hidden by [`WindowOptions::with_visible`](super::WindowOptions::with_visible) or
    /// [`Windows::set_visible`](super::Windows::set_visible).
    pub visible: bool,
}

impl WindowInfo {
    /// Whether the runner renders this window: visible, not minimized, not occluded and not empty.
    pub fn is_drawable(&self) -> bool {
        self.visible
            && !self.occluded
            && !self.minimized
            && self.size.width > 0
            && self.size.height > 0
    }
}

/// Where a requested window is in its life.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowStatus {
    /// Requested; the native window is created when the runner next processes requests.
    Pending,
    /// Created and attached to a renderer.
    Open,
    /// The last request for this key failed with this message. Requesting the key again retries.
    Failed(String),
}

/// A secondary window could not be created. The other windows are unaffected.
#[derive(Debug)]
pub enum WindowError {
    /// The system refused to create the native window.
    Create(winit::error::OsError),
    /// The renderer could not use the window's surface.
    Render(RenderError),
    /// The requested parent is not open.
    UnknownParent(super::WindowKey),
    /// The platform cannot show another window. The browser runner draws on one canvas, so
    /// every window after the main one is refused with this error.
    Unsupported(String),
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create(error) => write!(f, "creating the window: {error}"),
            Self::Render(error) => write!(f, "preparing the window surface: {error}"),
            Self::UnknownParent(key) => write!(f, "parent window {key} is not open"),
            Self::Unsupported(reason) => write!(f, "window not opened: {reason}"),
        }
    }
}

impl Error for WindowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Create(error) => Some(error),
            Self::Render(error) => Some(error),
            Self::UnknownParent(_) | Self::Unsupported(_) => None,
        }
    }
}

/// Counters over all windows of the running application.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppStats {
    pub windows_open: usize,
    pub windows_opened: u64,
    pub windows_closed: u64,
    /// Frames presented by all windows together.
    pub frames_presented: u64,
    /// Windows whose creation failed and was reported to the application.
    pub windows_failed: u64,
}
