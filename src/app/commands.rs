//! Application-level window requests. They are queued and executed by the runner, which is
//! the only code that touches the event loop, so `update` never needs `ActiveEventLoop`.

use super::{AppStats, WindowInfo, WindowKey, WindowOptions, WindowStatus};
use crate::{SharedResources, Style, Theme};
use std::collections::HashMap;

pub enum Command {
    Open {
        key: WindowKey,
        options: Box<WindowOptions>,
        declared: bool,
    },
    Close(WindowKey),
    RequestClose(WindowKey),
    Focus(WindowKey),
    Title(WindowKey, String),
    Size(WindowKey, f64, f64),
    Visible(WindowKey, bool),
    Repaint(Option<WindowKey>),
    Exit,
    /// A request the platform cannot honour, with the reason; the application is told through
    /// [`App::window_failed`](super::App::window_failed).
    Refused(WindowKey, String),
}

/// State the runner publishes to callbacks and the requests they queue.
pub struct Control {
    pub commands: Vec<Command>,
    pub status: HashMap<WindowKey, WindowStatus>,
    pub infos: HashMap<WindowKey, WindowInfo>,
    pub main: WindowKey,
    pub resources: SharedResources,
    /// Exit after the frame being built is presented ([`Frame::close`](super::Frame::close)).
    pub exit_after_present: bool,
    pub stats: AppStats,
    /// How many windows the platform can show at once; `None` for no limit. The browser
    /// has one canvas, so its runner sets `Some(1)`.
    pub max_windows: Option<usize>,
}

impl Control {
    pub fn new(main: WindowKey, resources: SharedResources) -> Self {
        Self {
            commands: Vec::new(),
            status: HashMap::new(),
            infos: HashMap::new(),
            main,
            resources,
            exit_after_present: false,
            stats: AppStats::default(),
            max_windows: None,
        }
    }
}

/// What [`Windows::open`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenOutcome {
    /// The window will be created when the runner next processes requests.
    Requested,
    /// The key is already open or already requested: nothing was created or recreated.
    AlreadyOpen,
    /// The platform cannot show another window (the browser runner has a single canvas).
    /// Nothing was queued; the key's [`status`](Windows::status) is `Failed` and
    /// [`App::window_failed`](super::App::window_failed) receives
    /// [`WindowError::Unsupported`](super::WindowError::Unsupported).
    Unsupported,
}

/// Open, close, focus and inspect native windows by [`WindowKey`].
///
/// Reached through [`Frame::windows`](super::Frame::windows) and the other callbacks of
/// [`App`](super::App). Requests take effect after the callback returns, in order. Every
/// request is addressed to a key, so none of them can reach the wrong window.
pub struct Windows<'a> {
    pub(crate) control: &'a mut Control,
}

impl<'a> Windows<'a> {
    #[doc(hidden)]
    pub fn from_control(control: &'a mut Control) -> Self {
        Self { control }
    }
}

impl Windows<'_> {
    /// Ask for a window. A key that is already open or already requested is left alone: no
    /// duplicate window, no new surface. A key whose last attempt failed is retried; the
    /// outcome of a failed creation arrives in [`App::window_failed`](super::App::window_failed)
    /// and in [`status`](Self::status).
    pub fn open(&mut self, key: impl Into<WindowKey>, options: WindowOptions) -> OpenOutcome {
        self.request(key.into(), options, false)
    }

    pub(crate) fn request(
        &mut self,
        key: WindowKey,
        options: WindowOptions,
        declared: bool,
    ) -> OpenOutcome {
        if matches!(
            self.control.status.get(&key),
            Some(WindowStatus::Pending | WindowStatus::Open)
        ) {
            return OpenOutcome::AlreadyOpen;
        }
        if let Some(limit) = self.control.max_windows {
            let taken = self
                .control
                .status
                .values()
                .filter(|status| matches!(status, WindowStatus::Pending | WindowStatus::Open))
                .count();
            if taken >= limit {
                let reason = format!("this platform shows at most {limit} window(s) at a time");
                self.control
                    .status
                    .insert(key.clone(), WindowStatus::Failed(reason.clone()));
                self.control.commands.push(Command::Refused(key, reason));
                return OpenOutcome::Unsupported;
            }
        }
        self.control
            .status
            .insert(key.clone(), WindowStatus::Pending);
        self.control.commands.push(Command::Open {
            key,
            options: Box::new(options),
            declared,
        });
        OpenOutcome::Requested
    }

    /// Close one window and its children now, without asking the application. Closing the
    /// main window follows the [`ExitPolicy`](super::ExitPolicy). A window that is not open
    /// is ignored, so closing twice is harmless. Its retained UI state is discarded.
    pub fn close(&mut self, key: impl Into<WindowKey>) {
        self.control.commands.push(Command::Close(key.into()));
    }

    /// Behave as if the user asked to close the window: the application receives
    /// [`App::close_requested`](super::App::close_requested) and may veto it.
    pub fn request_close(&mut self, key: impl Into<WindowKey>) {
        self.control
            .commands
            .push(Command::RequestClose(key.into()));
    }

    /// Bring the window forward and give it keyboard focus, restoring it if minimized.
    /// Systems may refuse to steal focus from another application.
    pub fn focus(&mut self, key: impl Into<WindowKey>) {
        self.control.commands.push(Command::Focus(key.into()));
    }

    pub fn set_title(&mut self, key: impl Into<WindowKey>, title: impl Into<String>) {
        self.control
            .commands
            .push(Command::Title(key.into(), title.into()));
    }

    /// Request a new inner size in logical pixels; the system may adjust it.
    pub fn set_inner_size(&mut self, key: impl Into<WindowKey>, width: f64, height: f64) {
        self.control
            .commands
            .push(Command::Size(key.into(), width, height));
    }

    /// Show or hide a window. A hidden window is not rendered and requests no frames.
    pub fn set_visible(&mut self, key: impl Into<WindowKey>, visible: bool) {
        self.control
            .commands
            .push(Command::Visible(key.into(), visible));
    }

    /// Draw `key` again soon. A window repaints on its own input and animations; call this
    /// when application data changed in another window and this one shows it. Hidden or
    /// minimized windows catch up when they are shown again.
    pub fn request_repaint(&mut self, key: impl Into<WindowKey>) {
        self.control
            .commands
            .push(Command::Repaint(Some(key.into())));
    }

    /// [`request_repaint`](Self::request_repaint) for every window.
    pub fn request_repaint_all(&mut self) {
        self.control.commands.push(Command::Repaint(None));
    }

    /// End the application, closing every window, without asking.
    pub fn exit(&mut self) {
        self.control.commands.push(Command::Exit);
    }

    /// Whether `key` is open or has been requested.
    pub fn is_open(&self, key: &WindowKey) -> bool {
        matches!(
            self.control.status.get(key),
            Some(WindowStatus::Pending | WindowStatus::Open)
        )
    }

    pub fn status(&self, key: &WindowKey) -> Option<&WindowStatus> {
        self.control.status.get(key)
    }

    /// State of an open window as of the start of this callback.
    pub fn info(&self, key: &WindowKey) -> Option<&WindowInfo> {
        self.control.infos.get(key)
    }

    /// Keys of open and requested windows, in no particular order.
    pub fn keys(&self) -> impl Iterator<Item = &WindowKey> {
        self.control
            .status
            .iter()
            .filter(|(_, status)| !matches!(status, WindowStatus::Failed(_)))
            .map(|(key, _)| key)
    }

    pub fn main_key(&self) -> &WindowKey {
        &self.control.main
    }

    pub fn stats(&self) -> AppStats {
        self.control.stats
    }

    /// The resources every window shares: glyph atlas, image cache, appearance.
    pub fn resources(&self) -> &SharedResources {
        &self.control.resources
    }

    /// Re-theme every window with one call; each applies it in its next frame.
    pub fn set_theme(&mut self, theme: Theme) {
        self.control.resources.set_theme(theme);
    }

    /// Install a resolved style in every window; see [`set_theme`](Self::set_theme).
    pub fn set_style(&mut self, style: Style) {
        self.control.resources.set_style(style);
    }
}
