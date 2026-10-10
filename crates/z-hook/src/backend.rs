//! The boundary between the frame driver and a graphics API.

use std::fmt;
use zaxis::DrawData;

/// How a call into a backend failed, which decides what the driver does about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendError {
    /// Not ready yet (resources are being created off the render thread): pass this frame
    /// through, quietly.
    NotReady,
    /// This frame cannot be drawn (a swapchain image in an unexpected state, a full queue);
    /// the next one may be.
    Frame(String),
    /// The swapchain, window or device went away or changed. Drop everything built for it and
    /// try again with the next present.
    Lost(String),
    /// The overlay cannot work with this host (a missing extension, an unusable format).
    /// Disable it for the rest of the process.
    Unsupported(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotReady => f.write_str("backend is not ready"),
            Self::Frame(message) => write!(f, "frame skipped: {message}"),
            Self::Lost(message) => write!(f, "target lost: {message}"),
            Self::Unsupported(message) => write!(f, "unsupported: {message}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// The backbuffer a frame is drawn into.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceInfo {
    /// Physical pixels.
    pub size: [u32; 2],
    /// DPI scale of the host's window, when the backend knows it.
    pub scale_factor: Option<f64>,
}

/// One graphics API's way of drawing into the frame the host is about to present.
///
/// The driver calls them in a fixed order on the host's render thread, inside the hooked
/// present call: `begin` (acquire the backbuffer and learn its size), then `render` (draw the
/// interface over it) and `end` (restore whatever state `begin` changed), always paired: after
/// a successful `begin`, `end` is called even when `render` fails or the interface panics.
/// A backend does not block: waiting for the GPU or building resources goes off-thread, and
/// `begin` says [`BackendError::NotReady`] meanwhile.
pub trait PresentBackend {
    fn begin(&mut self) -> Result<SurfaceInfo, BackendError>;

    fn render(&mut self, data: &DrawData) -> Result<(), BackendError>;

    fn end(&mut self) -> Result<(), BackendError>;
}
