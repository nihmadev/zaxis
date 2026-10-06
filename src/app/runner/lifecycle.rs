//! Attaching, closing and executing queued requests for windows, on every platform.
//! Creating a window's renderer is the platform module's job.

use super::{Native, Runner, Slot};
use crate::{
    app::{
        callbacks::CloseSource, commands::Command, registry::Closed, App, RunError, WindowError,
        WindowKey, WindowOptions,
    },
    Context, RenderError,
};
use winit::{dpi::LogicalSize, event_loop::ActiveEventLoop, keyboard::ModifiersState};

/// Requests a callback may queue per settle: a bound against callbacks that keep asking.
const MAX_ROUNDS: usize = 16;

impl From<WindowError> for RunError {
    fn from(error: WindowError) -> Self {
        match error {
            WindowError::Create(error) => Self::Window(error),
            WindowError::Render(error) => Self::Render(error),
            // The main window has no parent and is never refused; this only keeps the
            // conversion total.
            WindowError::UnknownParent(_) | WindowError::Unsupported(_) => {
                Self::Render(RenderError::Validation(error.to_string()))
            }
        }
    }
}

impl<A: App> Runner<A> {
    /// Give a window its slot, a context sized to it, and its place in the registry.
    pub(super) fn attach(&mut self, key: WindowKey, options: WindowOptions, native: Native) {
        let mut context = Context::with_shared(&self.hub.control.resources);
        context.set_viewport(native.window.inner_size(), native.window.scale_factor());
        context.set_accessibility_title(options.attributes.title.clone());
        self.hub.opened(&key, native.window.id());
        self.slots.insert(
            key,
            Slot {
                context,
                options,
                native: Some(native),
                modifiers: ModifiersState::default(),
                frames: 0,
            },
        );
    }

    pub(super) fn report_failure(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: WindowKey,
        error: WindowError,
    ) {
        self.app.window_failed(&key, &error);
        let closed = self.hub.open_failed(&key, error.to_string());
        self.finish_close(event_loop, closed);
    }

    /// Ask the application to close a window; it is closed unless the application objects.
    pub(super) fn request_close(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: &WindowKey,
        source: CloseSource,
    ) {
        let outcome = self.hub.close_requested(&mut self.app, key, source);
        if outcome.rejected {
            // Redraw at once so a confirmation opened by the veto is on screen.
            if let Some(slot) = self.slots.get_mut(key) {
                slot.context.request_repaint();
                if let Some(native) = &slot.native {
                    native.window.request_redraw();
                }
            }
        }
        self.finish_close(event_loop, outcome.closed);
    }

    /// Drop the closed windows. Their surfaces, uniforms, blur targets and contexts (with all
    /// retained widget state) are released here; a reopened key starts from scratch.
    pub(super) fn finish_close(&mut self, event_loop: &ActiveEventLoop, closed: Closed) {
        #[cfg(feature = "file-dialogs")]
        self.dialogs.window_closed(&closed.keys);
        for key in &closed.keys {
            if let Some(Slot {
                native: Some(native),
                ..
            }) = self.slots.remove(key)
            {
                // A clone of the window handle held elsewhere must not keep it on screen.
                native.window.set_visible(false);
            }
        }
        if closed.exit {
            event_loop.exit();
        }
    }

    /// Execute requests queued by callbacks, then re-read the application's declared windows
    /// when `declare` is set (after a frame or a lifecycle change).
    pub(super) fn settle(&mut self, event_loop: &ActiveEventLoop, declare: bool) {
        if declare {
            self.hub.declare(&mut self.app);
        }
        for _ in 0..MAX_ROUNDS {
            let commands = self.hub.take_commands();
            if commands.is_empty() {
                break;
            }
            for command in commands {
                self.execute(event_loop, command);
            }
        }
    }

    fn execute(&mut self, event_loop: &ActiveEventLoop, command: Command) {
        match command {
            Command::Open {
                key,
                options,
                declared,
            } => self.open_window(event_loop, key, *options, declared),
            Command::Refused(key, reason) => {
                let error = WindowError::Unsupported(reason);
                self.app.window_failed(&key, &error);
                self.hub.control.stats.windows_failed += 1;
            }
            Command::Close(key) => {
                let closed = self.hub.close(&key);
                self.finish_close(event_loop, closed);
            }
            Command::RequestClose(key) => {
                self.request_close(event_loop, &key, CloseSource::Application)
            }
            Command::Exit => event_loop.exit(),
            Command::Repaint(key) => {
                for (name, slot) in &mut self.slots {
                    if key.as_ref().is_none_or(|key| key == name) {
                        slot.context.request_repaint();
                        if let Some(native) = slot.native.as_ref().filter(|n| n.visible()) {
                            native.window.request_redraw();
                        }
                    }
                }
            }
            Command::Focus(key) => {
                if let Some(native) = self.native(&key) {
                    if native.window.is_minimized() == Some(true) {
                        native.window.set_minimized(false);
                    }
                    native.window.focus_window();
                }
            }
            Command::Title(key, title) => {
                if let Some(native) = self.native(&key) {
                    native.window.set_title(&title);
                }
                if let Some(slot) = self.slots.get_mut(&key) {
                    slot.context.set_accessibility_title(title);
                }
            }
            Command::Size(key, width, height) => {
                if let Some(native) = self.native(&key) {
                    let _ = native
                        .window
                        .request_inner_size(LogicalSize::new(width, height));
                }
            }
            Command::Visible(key, visible) => {
                if let Some(slot) = self.slots.get_mut(&key) {
                    if let Some(native) = &slot.native {
                        native.window.set_visible(visible);
                        if visible {
                            slot.context.request_repaint();
                            native.window.request_redraw();
                        }
                    }
                }
            }
        }
    }

    fn native(&self, key: &WindowKey) -> Option<&Native> {
        self.slots.get(key).and_then(|slot| slot.native.as_ref())
    }
}
