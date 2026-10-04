//! Creating, closing and recovering windows, and executing queued window requests.

use super::{native, Native, Runner, Slot};
use crate::{
    app::{
        callbacks::CloseSource,
        commands::Command,
        registry::{Closed, OpenRequest},
        App, RunError, WindowError, WindowKey, WindowOptions,
    },
    Context, PresentationMode, RenderError, Renderer,
};
use std::sync::Arc;
use winit::{
    dpi::LogicalSize, event_loop::ActiveEventLoop, keyboard::ModifiersState, window::Window,
};

/// Requests a callback may queue per settle: a bound against callbacks that keep asking.
const MAX_ROUNDS: usize = 16;

impl From<WindowError> for RunError {
    fn from(error: WindowError) -> Self {
        match error {
            WindowError::Create(error) => Self::Window(error),
            WindowError::Render(error) => Self::Render(error),
            // The main window has no parent; this only keeps the conversion total.
            WindowError::UnknownParent(_) => {
                Self::Render(RenderError::Validation(error.to_string()))
            }
        }
    }
}

impl<A: App> Runner<A> {
    /// The first window. Unlike a secondary window, failing to create it ends the run.
    pub(super) fn open_main(&mut self, event_loop: &ActiveEventLoop) -> Result<(), RunError> {
        let key = self.options.main_window.clone();
        let mut options = WindowOptions::from_attributes(self.options.window_attributes.clone());
        options.presentation_mode = Some(self.options.presentation_mode);
        self.hub.begin_open(&key, None, false);
        let native = self.create_native(event_loop, &options)?;
        self.attach(key, options, native);
        Ok(())
    }

    fn create_native(
        &self,
        event_loop: &ActiveEventLoop,
        options: &WindowOptions,
    ) -> Result<Native, WindowError> {
        let owner = options
            .parent
            .as_ref()
            .and_then(|parent| self.slots.get(parent))
            .and_then(|slot| slot.native.as_ref())
            .map(|native| native.window.as_ref());
        let sibling = self
            .slots
            .values()
            .find_map(|slot| slot.native.as_ref())
            .map(|native| &native.renderer);
        native::create(
            event_loop,
            options,
            owner,
            sibling,
            self.options.presentation_mode,
            &self.hub.control.resources,
        )
    }

    fn attach(&mut self, key: WindowKey, options: WindowOptions, native: Native) {
        let mut context = Context::with_shared(&self.hub.control.resources);
        context.set_viewport(native.window.inner_size(), native.window.scale_factor());
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

    /// Open a secondary window. A failure is reported to the application and leaves every
    /// other window untouched. One key yields at most one window.
    pub(super) fn open_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: WindowKey,
        options: WindowOptions,
        declared: bool,
    ) {
        match self.hub.begin_open(&key, options.parent.as_ref(), declared) {
            OpenRequest::Exists => return,
            OpenRequest::UnknownParent => {
                let parent = options.parent.clone().unwrap_or_default();
                return self.report_failure(event_loop, key, WindowError::UnknownParent(parent));
            }
            OpenRequest::Queued => {}
        }
        match self.create_native(event_loop, &options) {
            Ok(native) => self.attach(key, options, native),
            Err(error) => self.report_failure(event_loop, key, error),
        }
    }

    fn report_failure(&mut self, event_loop: &ActiveEventLoop, key: WindowKey, error: WindowError) {
        self.app.window_failed(&key, &error);
        let closed = self.hub.open_failed(&key, error.to_string());
        self.finish_close(event_loop, closed);
    }

    /// After a suspension: give every window a native window and renderer again.
    pub(super) fn recreate_natives(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), RunError> {
        let mut keys: Vec<_> = self.slots.keys().cloned().collect();
        keys.sort_by_key(|key| !self.hub.registry.is_main(key));
        for key in keys {
            let options = self.slots[&key].options.clone();
            match self.create_native(event_loop, &options) {
                Ok(native) => {
                    self.hub.registry.attach(&key, native.window.id());
                    let slot = self.slots.get_mut(&key).expect("slot exists");
                    slot.context
                        .set_viewport(native.window.inner_size(), native.window.scale_factor());
                    slot.context.request_repaint();
                    slot.native = Some(native);
                }
                Err(error) if self.hub.registry.is_main(&key) => return Err(error.into()),
                Err(error) => {
                    self.app.window_failed(&key, &error);
                    let closed = self.hub.close(&key);
                    self.finish_close(event_loop, closed);
                }
            }
        }
        Ok(())
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

    /// The GPU device was lost: rebuild one renderer on a new device and give every other
    /// window a sibling of it. A window that cannot be recovered is closed and reported;
    /// losing the device for the first window is fatal, as for a single window.
    pub(super) fn recover_device(
        &mut self,
        event_loop: &ActiveEventLoop,
        reason: String,
    ) -> Result<(), RenderError> {
        self.device_resets += 1;
        self.device_loss = Some(reason);
        let mut windows: Vec<(WindowKey, Arc<Window>, PresentationMode)> = self
            .slots
            .iter()
            .filter_map(|(key, slot)| {
                let native = slot.native.as_ref()?;
                Some((
                    key.clone(),
                    Arc::clone(&native.window),
                    native.renderer.presentation_mode(),
                ))
            })
            .collect();
        windows.sort_by_key(|(key, ..)| !self.hub.registry.is_main(key));
        let mut renderers: Vec<(WindowKey, Renderer)> = Vec::new();
        let mut lost = Vec::new();
        for (key, window, mode) in windows {
            let renderer = match renderers.first() {
                Some((_, first)) => first.create_sibling(window, mode),
                None => pollster::block_on(Renderer::new_with_presentation_mode(window, mode)),
            };
            match renderer {
                Ok(renderer) => renderers.push((key, renderer)),
                Err(error) if renderers.is_empty() => return Err(error),
                Err(error) => lost.push((key, error)),
            }
        }
        for (key, mut renderer) in renderers {
            let Some(slot) = self.slots.get_mut(&key) else {
                continue;
            };
            let Some(native) = slot.native.as_mut() else {
                continue;
            };
            renderer.set_transparent(native.transparent);
            native::clamp_image_limits(&self.hub.control.resources, &renderer);
            native.renderer = renderer;
            native.retry_at = None;
            slot.context.request_repaint();
            native.window.request_redraw();
        }
        for (key, error) in lost {
            self.report_failure(event_loop, key, WindowError::Render(error));
        }
        Ok(())
    }
}
