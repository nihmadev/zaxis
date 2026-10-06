//! Routing one window's events into its context, and drawing its frames.

use super::{Runner, Slot};
use crate::time::Instant;
use crate::{
    app::{
        callbacks::{CloseSource, GlobalShortcut},
        App, Frame, WindowKey, Windows,
    },
    RenderError, RenderStatus,
};
use std::time::Duration;
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
};

impl<A: App> Runner<A> {
    /// Handle one event addressed to `key`. Returns whether the set of windows the
    /// application declares may have changed (after a frame or a close).
    pub(super) fn dispatch(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: &WindowKey,
        event: WindowEvent,
    ) -> Result<bool, RenderError> {
        if let Some(Slot {
            native: Some(native),
            ..
        }) = self.slots.get_mut(key)
        {
            native.accessibility_event(&event);
        }
        match &event {
            WindowEvent::CloseRequested => {
                self.request_close(event_loop, key, CloseSource::System);
                return Ok(true);
            }
            WindowEvent::RedrawRequested => return self.redraw(event_loop, key),
            WindowEvent::ModifiersChanged(modifiers) => {
                if let Some(slot) = self.slots.get_mut(key) {
                    slot.modifiers = modifiers.state();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && self.shortcut_taken(key, event) =>
            {
                return Ok(false)
            }
            _ => {}
        }
        let Some(Slot {
            context,
            native: Some(native),
            ..
        }) = self.slots.get_mut(key)
        else {
            return Ok(false);
        };
        if context.native_chrome_press(&event, &native.window) {
            native.window.request_redraw();
            return Ok(false);
        }
        let response = context.on_window_event(&event);
        self.platform.note_input(&event, response);
        match event {
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                let size = native.window.inner_size();
                context.set_viewport(size, native.window.scale_factor());
                native.renderer.resize(size)?;
            }
            WindowEvent::Occluded(value) => {
                native.occluded = value;
                if !value {
                    native.window.request_redraw();
                }
            }
            WindowEvent::Focused(_) => {
                // Losing focus ends composition and disables the IME for this window only.
                context.sync_ime(&native.window);
            }
            _ => {}
        }
        if response.repaint && native.visible() {
            native.window.request_redraw();
        }
        let cursor = context.cursor_icon();
        if cursor != native.cursor {
            native.window.set_cursor(cursor);
            native.cursor = cursor;
        }
        Ok(false)
    }

    /// Assistive technology asked `event.window_id` for its tree or for an action: the
    /// request becomes input of that window and is answered by its next frame.
    #[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
    pub(super) fn accessibility_event(&mut self, event: crate::accessibility::adapter::Event) {
        let Some(key) = self.hub.registry.route(event.window_id).cloned() else {
            return;
        };
        let Some(Slot {
            context,
            native: Some(native),
            ..
        }) = self.slots.get_mut(&key)
        else {
            return;
        };
        if crate::accessibility::adapter::handle(context, event.window_event) && native.visible() {
            native.window.request_redraw();
        }
    }

    /// Offer a key press to the application first; true when it took the key.
    fn shortcut_taken(&mut self, key: &WindowKey, event: &winit::event::KeyEvent) -> bool {
        let PhysicalKey::Code(code) = event.physical_key else {
            return false;
        };
        let Some(slot) = self.slots.get(key) else {
            return false;
        };
        let shortcut = GlobalShortcut {
            window: key,
            key: code,
            modifiers: slot.modifiers,
            repeat: event.repeat,
        };
        self.hub.shortcut(&mut self.app, &shortcut)
    }

    /// Publish a fresh snapshot of every window's state for callbacks to read.
    fn refresh_infos(&mut self) {
        for (key, slot) in &self.slots {
            if let Some(native) = &slot.native {
                self.hub.control.infos.insert(key.clone(), native.info());
            }
        }
    }

    /// Build and present one frame of one window; other windows are not touched.
    fn redraw(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: &WindowKey,
    ) -> Result<bool, RenderError> {
        self.refresh_infos();
        let Runner {
            app,
            hub,
            slots,
            device_resets,
            device_loss,
            ..
        } = self;
        let Some(Slot {
            context,
            native: Some(native),
            frames,
            ..
        }) = slots.get_mut(key)
        else {
            return Ok(false);
        };
        if !native.visible() {
            return Ok(false);
        }
        native.retry_at = None;
        let info = hub
            .control
            .infos
            .get(key)
            .cloned()
            .unwrap_or_else(|| native.info());
        let mut frame = Frame {
            window: &native.window,
            key,
            info,
            windows: Windows {
                control: &mut hub.control,
            },
            device_resets: *device_resets,
            device_loss: device_loss.as_deref(),
        };
        context.run(|context| app.update(context, &mut frame));
        #[cfg(feature = "file-dialogs")]
        let launches = context.take_dialog_launches();
        native.publish_accessibility(context);
        context.sync_ime(&native.window);
        let status = native
            .renderer
            .render(context.draw_data(), context.style().background);
        match status {
            Ok(RenderStatus::Presented) => {
                *frames += 1;
                hub.control.stats.frames_presented += 1;
                if hub.control.exit_after_present {
                    event_loop.exit();
                }
            }
            Ok(RenderStatus::Dormant) => {}
            Ok(RenderStatus::Retry) => {
                native.retry_at = Some(Instant::now() + Duration::from_millis(16));
            }
            Err(RenderError::DeviceLost(reason)) => self.recover_device(event_loop, reason)?,
            Err(error) => return Err(error),
        }
        let cursor = context_cursor(self.slots.get_mut(key));
        if let Some((native, cursor)) = cursor {
            if cursor != native.cursor {
                native.window.set_cursor(cursor);
                native.cursor = cursor;
            }
        }
        #[cfg(feature = "file-dialogs")]
        self.launch_dialogs(key, launches);
        Ok(true)
    }
}

fn context_cursor(
    slot: Option<&mut Slot>,
) -> Option<(&mut super::Native, winit::window::CursorIcon)> {
    let slot = slot?;
    let cursor = slot.context.cursor_icon();
    Some((slot.native.as_mut()?, cursor))
}
