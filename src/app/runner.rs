//! The winit application handler: one window and one context per [`WindowKey`].
//!
//! Everything here is platform independent: routing events into contexts, repaint
//! scheduling, the window table. What differs lives in the platform module: `native`
//! (blocking GPU setup, several windows, native chrome) or `web` (one canvas, asynchronous
//! GPU setup, no blocking and no threads).

#[cfg(feature = "file-dialogs")]
mod dialogs;
mod events;
mod lifecycle;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;
mod window;

use super::{
    hub::Hub,
    schedule::{schedule_all, WindowTiming},
    App, RunError, RunOptions, WindowKey, WindowOptions,
};
use crate::time::Instant;
use crate::{Context, PresentationMode, SharedResources};
#[cfg(not(target_arch = "wasm32"))]
use native as platform;
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use web as platform;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
    keyboard::ModifiersState,
    window::WindowId,
};

pub(super) use platform::run;
pub(super) use window::Native;

/// Events other threads and tasks send into the loop.
pub(super) enum UserEvent {
    /// A finished image job or a worker's wake-up: `about_to_wait` redraws exactly the
    /// windows whose context reports that it needs a frame.
    Wake,
    /// The browser finished creating a renderer asynchronously; it waits in the runner's
    /// inbox because a renderer cannot cross threads, and so could not ride in this event.
    #[cfg(target_arch = "wasm32")]
    Renderer,
    /// Files were dragged over or dropped on the canvas; the page's listeners queued them.
    #[cfg(target_arch = "wasm32")]
    Files,
    /// Assistive technology asked a window for its tree, requested an action, or went
    /// away. Platform adapters report from their own threads, hence through the proxy.
    #[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
    Access(crate::accessibility::adapter::Event),
}

#[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
impl From<crate::accessibility::adapter::Event> for UserEvent {
    fn from(event: crate::accessibility::adapter::Event) -> Self {
        Self::Access(event)
    }
}

/// Everything that belongs to one window. Dropping it releases the surface, the renderer's
/// per-window buffers and all retained UI state of the window's context.
pub(super) struct Slot {
    pub(super) context: Context,
    /// Kept to recreate the window after a suspension, which only the desktop does.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(super) options: WindowOptions,
    /// `None` while the system has suspended the application.
    pub(super) native: Option<Native>,
    pub(super) modifiers: ModifiersState,
    pub(super) frames: u64,
}

pub(super) struct Runner<A> {
    pub(super) app: A,
    pub(super) options: RunOptions,
    pub(super) hub: Hub<WindowId>,
    pub(super) slots: HashMap<WindowKey, Slot>,
    pub(super) error: Option<RunError>,
    pub(super) device_resets: u64,
    pub(super) device_loss: Option<String>,
    pub(super) started: bool,
    pub(super) platform: platform::State,
    #[cfg(feature = "file-dialogs")]
    pub(super) dialogs: crate::app::dialogs::DialogHost,
}

/// The resources every window shares, with the fonts the options ask for.
fn shared_resources(options: &RunOptions) -> SharedResources {
    match (&options.font_family, &options.monospace_family) {
        (None, None) => SharedResources::new(),
        (family, monospace) => SharedResources::with_font_families(
            family.clone().unwrap_or_default(),
            monospace
                .clone()
                .or_else(crate::FontFamily::default_monospace),
        ),
    }
}

impl<A: App> Runner<A> {
    pub(super) fn new(
        app: A,
        options: RunOptions,
        resources: SharedResources,
        platform: platform::State,
    ) -> Self {
        Self {
            hub: Hub::new(options.main_window.clone(), options.exit_policy, resources),
            app,
            options,
            slots: HashMap::new(),
            error: None,
            device_resets: 0,
            device_loss: None,
            started: false,
            platform,
            #[cfg(feature = "file-dialogs")]
            dialogs: crate::app::dialogs::DialogHost::system(),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: RunError) {
        self.platform.report_failure(&error);
        self.error = Some(error);
        event_loop.exit();
    }
}

impl<A: App> ApplicationHandler<UserEvent> for Runner<A> {
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Wake => {}
            #[cfg(target_arch = "wasm32")]
            UserEvent::Renderer => self.renderers_ready(event_loop),
            #[cfg(target_arch = "wasm32")]
            UserEvent::Files => self.web_files(),
            #[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
            UserEvent::Access(event) => self.accessibility_event(event),
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = event_loop;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = if self.started {
            self.recreate_natives(event_loop)
        } else {
            self.started = true;
            self.open_main(event_loop)
        };
        match result {
            Ok(()) => self.settle(event_loop, true),
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        // Drop every surface on platforms that invalidate them on suspension, while
        // preserving application state and each window's retained UI state.
        for slot in self.slots.values_mut() {
            slot.native = None;
            slot.context.on_window_event(&WindowEvent::Focused(false));
        }
        self.hub.registry.detach_all();
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(key) = self.hub.registry.route(id).cloned() else {
            return;
        };
        let modifiers = self.slots.get(&key).map(|slot| slot.modifiers);
        let frame_now = modifiers.is_some_and(|m| platform::frame_before_returning(&event, m));
        match self.dispatch(event_loop, &key, event) {
            Ok(changed) => self.settle(event_loop, changed),
            Err(error) => self.fail(event_loop, RunError::Render(error)),
        }
        if frame_now {
            // The browser allows a clipboard write only inside the gesture that asked
            // for it, so the key press that copies draws its frame before the handler returns.
            if let Err(error) = self.dispatch(event_loop, &key, WindowEvent::RedrawRequested) {
                self.fail(event_loop, RunError::Render(error));
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let timings: Vec<_> = self
            .slots
            .iter()
            .filter_map(|(key, slot)| {
                let native = slot.native.as_ref()?;
                let moving = native.renderer.presentation_mode() == PresentationMode::Vsync
                    && slot.context.wants_animation_frame();
                Some((
                    key.clone(),
                    WindowTiming {
                        visible: native.visible(),
                        needs_repaint: slot.context.needs_repaint_at(now) || moving,
                        next_repaint: slot.context.next_repaint(),
                        retry_at: native.retry_at,
                    },
                ))
            })
            .collect();
        let (redraw, flow) = schedule_all(now, timings);
        for key in redraw {
            if let Some(native) = self.slots.get(&key).and_then(|s| s.native.as_ref()) {
                native.window.request_redraw();
            }
        }
        event_loop.set_control_flow(flow);
    }
}
