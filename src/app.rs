//! Desktop runner: one or several native windows, each with its own context and surface.
//! The low-level Context and Renderer remain available for custom hosts.

mod callbacks;
mod commands;
mod frame;
mod hub;
#[cfg(test)]
mod hub_tests;
mod info;
mod key;
mod options;
mod registry;
#[cfg(test)]
mod registry_tests;
mod runner;
mod schedule;
#[cfg(test)]
mod shared_tests;
#[cfg(test)]
mod tests;

pub use callbacks::{CloseRequested, CloseSource, GlobalShortcut, WindowPlan};
pub use commands::{OpenOutcome, Windows};
pub use frame::Frame;
pub use info::{AppStats, WindowError, WindowInfo, WindowStatus};
pub use key::WindowKey;
pub use options::{ExitPolicy, WindowOptions};

use crate::{Context, PresentationMode, RenderError, SharedResources};
use runner::Runner;
#[cfg(test)]
use schedule::repaint_schedule;
#[cfg(test)]
use std::time::{Duration, Instant};
use std::{error::Error, fmt};
use winit::{
    dpi::LogicalSize,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

/// Application state lives here, independently of the native windows and GPU.
///
/// Only [`update`](Self::update) is required; an application that implements nothing else
/// is a single-window application: closing its window ends it, as before. The other
/// methods are for applications with several native windows.
pub trait App {
    /// Build the UI of one window on each of its redraws. Do not call `Context::run` yourself.
    /// `frame.window_key()` names the window and `context` is that window's alone, so draw
    /// each window from your own data keyed by it. Widgets request follow-up frames
    /// automatically. For timers and animations, use `Context::request_repaint_after`;
    /// the runner sleeps between redraws and redraws only the windows that asked.
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>);

    /// Declare the secondary windows that should be open. Called at startup and after every
    /// frame; see [`WindowPlan`]. Skip it to open windows imperatively with
    /// [`Frame::open_window`] instead. Both styles can be mixed.
    fn windows(&mut self, _plan: &mut WindowPlan) {}

    /// The user (or [`Windows::request_close`]) asked to close a window. The default closes it;
    /// call [`CloseRequested::reject`] to keep it open, for example to ask "Save changes?"
    /// in a [`Modal`](crate::Modal). Closing the main window follows [`RunOptions::exit_policy`].
    fn close_requested(&mut self, _request: &mut CloseRequested<'_>) {}

    /// A secondary window could not be created or recovered. The other windows are unaffected
    /// and the failed key can be requested again.
    fn window_failed(&mut self, _key: &WindowKey, _error: &WindowError) {}

    /// A key was pressed in the focused window. Return `true` to take it: the window's
    /// context then never sees it. The default leaves every shortcut with its window.
    fn global_shortcut(
        &mut self,
        _shortcut: &GlobalShortcut<'_>,
        _windows: &mut Windows<'_>,
    ) -> bool {
        false
    }
}

/// Native window configuration. Sizes in `WindowAttributes` may be logical or physical.
/// On macOS the runner always enables system decorations for native controls and resizing.
///
/// `window_attributes` and `presentation_mode` describe the main window. Other windows use
/// [`WindowOptions`].
#[derive(Debug, Clone)]
pub struct RunOptions {
    pub window_attributes: WindowAttributes,
    pub presentation_mode: PresentationMode,
    /// Replaces the bundled Inter family for every window. System fonts and color emoji
    /// still supply scripts the family lacks.
    pub font_family: Option<crate::FontFamily>,
    /// Replaces the bundled JetBrains Mono used by monospace text, independently of
    /// `font_family`. Without the `bundled-monospace` feature and without this, monospace
    /// text uses the system's generic monospace font.
    pub monospace_family: Option<crate::FontFamily>,
    /// Key of the first window. Default: [`WindowKey::main`].
    pub main_window: WindowKey,
    /// What closing the main window does. Default: [`ExitPolicy::MainWindow`].
    pub exit_policy: ExitPolicy,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            window_attributes: Window::default_attributes()
                .with_title("zaxis")
                .with_inner_size(LogicalSize::new(860.0, 560.0)),
            presentation_mode: PresentationMode::default(),
            font_family: None,
            monospace_family: None,
            main_window: WindowKey::main(),
            exit_policy: ExitPolicy::default(),
        }
    }
}

impl RunOptions {
    /// Use `family` for all text instead of the bundled Inter, without building
    /// a custom host around [`Context::with_fonts`].
    pub fn with_font_family(mut self, family: crate::FontFamily) -> Self {
        self.font_family = Some(family);
        self
    }

    /// Use `family` for monospace text instead of the bundled JetBrains Mono.
    pub fn with_monospace_family(mut self, family: crate::FontFamily) -> Self {
        self.monospace_family = Some(family);
        self
    }

    /// Choose whether the application ends with the main window or with the last window.
    pub fn with_exit_policy(mut self, policy: ExitPolicy) -> Self {
        self.exit_policy = policy;
        self
    }

    /// Name the main window, for applications that address it by a key of their own.
    pub fn with_main_window(mut self, key: impl Into<WindowKey>) -> Self {
        self.main_window = key.into();
        self
    }

    /// Request native rounded corners and the system shadow on Windows 11.
    /// Applies to undecorated windows as well. Maximized/snapped windows follow
    /// the OS policy. On other platforms this leaves window attributes unchanged.
    /// Call after setting `window_attributes`. Secondary windows choose their own with
    /// [`WindowOptions::with_rounded_corners`].
    pub fn with_rounded_corners(mut self, rounded: bool) -> Self {
        self.window_attributes = round_corners(self.window_attributes, rounded);
        self
    }
}

#[cfg(target_os = "windows")]
fn round_corners(attributes: WindowAttributes, rounded: bool) -> WindowAttributes {
    use winit::platform::windows::{CornerPreference, WindowAttributesExtWindows};
    attributes
        .with_undecorated_shadow(rounded)
        .with_corner_preference(if rounded {
            CornerPreference::Round
        } else {
            CornerPreference::DoNotRound
        })
}

#[cfg(not(target_os = "windows"))]
fn round_corners(attributes: WindowAttributes, _rounded: bool) -> WindowAttributes {
    attributes
}

/// Open a desktop window and run an application on the main thread until closed.
/// Returns event-loop, window creation, and unrecoverable renderer errors.
/// winit generally permits only one event loop per process.
///
/// ```no_run
/// use zaxis::{App, Context, Frame, Window};
/// struct MyApp;
/// impl App for MyApp {
///     fn update(&mut self, context: &mut Context, _frame: &mut Frame<'_>) {
///         Window::new("Tools").show(context, |ui| { ui.label("Hello"); });
///     }
/// }
/// fn main() -> Result<(), zaxis::RunError> {
///     zaxis::run(MyApp)
/// }
/// ```
pub fn run(app: impl App) -> Result<(), RunError> {
    run_with_options(app, RunOptions::default())
}

/// Run with explicit window attributes and presentation mode. See [`run`].
pub fn run_with_options(app: impl App, options: RunOptions) -> Result<(), RunError> {
    let event_loop = EventLoop::<()>::with_user_event()
        .build()
        .map_err(RunError::EventLoop)?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let resources = match (&options.font_family, &options.monospace_family) {
        (None, None) => SharedResources::new(),
        (family, monospace) => SharedResources::with_font_families(
            family.clone().unwrap_or_default(),
            monospace
                .clone()
                .or_else(crate::FontFamily::default_monospace),
        ),
    };
    resources.set_image_waker(move || {
        let _ = proxy.send_event(());
    });
    let mut runner = Runner::new(app, options, resources);
    let result = event_loop.run_app(&mut runner).map_err(RunError::EventLoop);
    runner.error.take().map_or(result, Err)
}

/// An unrecoverable desktop runner failure.
#[derive(Debug)]
pub enum RunError {
    EventLoop(winit::error::EventLoopError),
    Window(winit::error::OsError),
    Render(RenderError),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventLoop(error) => write!(f, "running the event loop: {error}"),
            Self::Window(error) => write!(f, "creating the window: {error}"),
            Self::Render(error) => error.fmt(f),
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::EventLoop(error) => error,
            Self::Window(error) => error,
            Self::Render(error) => error,
        })
    }
}
