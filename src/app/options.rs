//! Native window attributes and the exit policy.

use super::WindowKey;
use crate::PresentationMode;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    window::{Icon, Window, WindowAttributes, WindowLevel},
};

/// What ends the application.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExitPolicy {
    /// Closing the main window ends the application and closes every other window with it.
    /// Closing any other window closes only that window and its children.
    #[default]
    MainWindow,
    /// The application runs until the last window closes. Closing the main window closes
    /// its children but leaves unrelated windows open.
    LastWindow,
}

/// Configuration of one secondary native window.
///
/// Sizes and positions are logical pixels. A window's `parent` must already be open; the
/// child closes with it, and on Windows stays above it in the Z-order (the owner relation).
/// Other platforms keep the close-with-parent rule but leave the Z-order to the system.
///
/// ```
/// use zaxis::{WindowKey, WindowOptions};
/// let palette = WindowOptions::new("Tools")
///     .with_inner_size(260.0, 420.0)
///     .with_always_on_top(true)
///     .with_resizable(false)
///     .with_parent(WindowKey::main());
/// assert_eq!(palette.parent(), Some(&WindowKey::main()));
/// ```
#[derive(Debug, Clone)]
pub struct WindowOptions {
    pub attributes: WindowAttributes,
    pub(crate) parent: Option<WindowKey>,
    pub(crate) presentation_mode: Option<PresentationMode>,
}

impl WindowOptions {
    /// A decorated, resizable window of 640 × 480 logical pixels.
    pub fn new(title: impl Into<String>) -> Self {
        Self::from_attributes(
            Window::default_attributes()
                .with_title(title)
                .with_inner_size(LogicalSize::new(640.0, 480.0)),
        )
    }

    /// Start from complete winit attributes for options this builder does not cover.
    pub fn from_attributes(attributes: WindowAttributes) -> Self {
        Self {
            attributes,
            parent: None,
            presentation_mode: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.attributes = self.attributes.with_title(title);
        self
    }

    pub fn with_inner_size(mut self, width: f64, height: f64) -> Self {
        self.attributes = self
            .attributes
            .with_inner_size(LogicalSize::new(width, height));
        self
    }

    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.attributes = self.attributes.with_position(LogicalPosition::new(x, y));
        self
    }

    pub fn with_min_inner_size(mut self, width: f64, height: f64) -> Self {
        self.attributes = self
            .attributes
            .with_min_inner_size(LogicalSize::new(width, height));
        self
    }

    pub fn with_max_inner_size(mut self, width: f64, height: f64) -> Self {
        self.attributes = self
            .attributes
            .with_max_inner_size(LogicalSize::new(width, height));
        self
    }

    /// System title bar and borders. Turn off to draw a [`TitleBar`](crate::TitleBar).
    /// macOS always keeps system decorations.
    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.attributes = self.attributes.with_decorations(decorations);
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.attributes = self.attributes.with_resizable(resizable);
        self
    }

    pub fn with_always_on_top(mut self, on_top: bool) -> Self {
        self.attributes = self.attributes.with_window_level(if on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        });
        self
    }

    /// A surface that composites with the desktop where the platform supports it. The
    /// style background's alpha then decides how much of the desktop shows through.
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.attributes = self.attributes.with_transparent(transparent);
        self
    }

    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.attributes = self.attributes.with_window_icon(Some(icon));
        self
    }

    /// A window created hidden is not rendered and requests no frames until shown with
    /// [`Windows::set_visible`](super::Windows::set_visible).
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.attributes = self.attributes.with_visible(visible);
        self
    }

    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.attributes = self.attributes.with_maximized(maximized);
        self
    }

    /// Close this window with `parent`, which must be open when this window is requested.
    pub fn with_parent(mut self, parent: impl Into<WindowKey>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    /// Present with `mode` instead of the [`RunOptions`](crate::RunOptions) default.
    pub fn with_presentation_mode(mut self, mode: PresentationMode) -> Self {
        self.presentation_mode = Some(mode);
        self
    }

    /// Native rounded corners and shadow on Windows 11; see [`RunOptions::with_rounded_corners`](crate::RunOptions::with_rounded_corners).
    pub fn with_rounded_corners(mut self, rounded: bool) -> Self {
        self.attributes = super::round_corners(self.attributes, rounded);
        self
    }

    pub fn parent(&self) -> Option<&WindowKey> {
        self.parent.as_ref()
    }
}
