//! Values the runner hands to optional [`App`](super::App) callbacks.

use super::{commands::Control, WindowKey, WindowOptions, Windows};
use winit::keyboard::{KeyCode, ModifiersState};

/// Who asked for a window to close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseSource {
    /// The user or the system: the close button, Alt+F4, the taskbar, a session end.
    System,
    /// The application, through [`Windows::request_close`].
    Application,
}

/// A request to close one window, delivered to
/// [`App::close_requested`](super::App::close_requested).
///
/// The window closes unless the callback calls [`reject`](Self::reject). One user action
/// produces one request, addressed to one window. While a request is rejected the window
/// stays open and repaints, so a [`Modal`](crate::Modal) opened in response is drawn at once;
/// confirming it later is [`Windows::close`].
pub struct CloseRequested<'a> {
    pub(crate) key: &'a WindowKey,
    pub(crate) source: CloseSource,
    pub(crate) windows: Windows<'a>,
    pub(crate) rejected: bool,
}

impl<'a> CloseRequested<'a> {
    pub(crate) fn new(key: &'a WindowKey, source: CloseSource, control: &'a mut Control) -> Self {
        Self {
            key,
            source,
            windows: Windows { control },
            rejected: false,
        }
    }

    /// The window that is being closed.
    pub fn window(&self) -> &WindowKey {
        self.key
    }

    pub fn source(&self) -> CloseSource {
        self.source
    }

    /// Whether this is the main window, whose closing follows the
    /// [`ExitPolicy`](super::ExitPolicy).
    pub fn is_main(&self) -> bool {
        self.windows.main_key() == self.key
    }

    /// Let the window close. This is the default.
    pub fn accept(&mut self) {
        self.rejected = false;
    }

    /// Keep the window open, for example while a confirmation is shown.
    pub fn reject(&mut self) {
        self.rejected = true;
    }

    /// Other windows, for instance to close or focus one together with this decision.
    pub fn windows(&mut self) -> &mut Windows<'a> {
        &mut self.windows
    }
}

/// A key press seen before the window it was addressed to handles it.
///
/// Shortcuts belong to the focused window by default: its [`Context`](crate::Context) sees
/// the key and no other window does. [`App::global_shortcut`](super::App::global_shortcut)
/// is the explicit way to act on a key whichever window has focus.
#[derive(Clone, Copy, Debug)]
pub struct GlobalShortcut<'a> {
    /// The window that has keyboard focus.
    pub window: &'a WindowKey,
    pub key: KeyCode,
    pub modifiers: ModifiersState,
    pub repeat: bool,
}

/// Collects the secondary windows the application wants open right now.
///
/// Passed to [`App::windows`](super::App::windows). The runner opens declared keys that are
/// closed and closes windows it opened from a declaration that is no longer made. A window
/// the user closes while it is still declared stays closed until the application stops
/// declaring it once, so a close is never undone behind the application's back.
#[derive(Default)]
pub struct WindowPlan {
    pub(crate) declared: Vec<(WindowKey, WindowOptions)>,
}

impl WindowPlan {
    /// Declare `key` open with `options`. Options matter only when the window is created.
    /// Declaring a key twice keeps the first declaration.
    pub fn window(&mut self, key: impl Into<WindowKey>, options: WindowOptions) -> &mut Self {
        let key = key.into();
        if !self.declared.iter().any(|(declared, _)| *declared == key) {
            self.declared.push((key, options));
        }
        self
    }

    /// Declare `key` only while `condition` holds.
    pub fn window_if(
        &mut self,
        condition: bool,
        key: impl Into<WindowKey>,
        options: impl FnOnce() -> WindowOptions,
    ) -> &mut Self {
        if condition {
            self.window(key, options());
        }
        self
    }
}
