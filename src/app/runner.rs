//! The winit application handler: one native window and one context per [`WindowKey`].

mod events;
mod lifecycle;
mod native;

use super::{
    hub::Hub,
    schedule::{schedule_all, WindowTiming},
    App, RunError, RunOptions, WindowKey, WindowOptions,
};
use crate::{Context, PresentationMode};
use native::Native;
use std::{collections::HashMap, time::Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
    keyboard::ModifiersState,
    window::WindowId,
};

/// Everything that belongs to one window. Dropping it releases the surface, the renderer's
/// per-window buffers and all retained UI state of the window's context.
pub(super) struct Slot {
    pub(super) context: Context,
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
}

impl<A: App> Runner<A> {
    pub(super) fn new(app: A, options: RunOptions, resources: crate::SharedResources) -> Self {
        Self {
            hub: Hub::new(options.main_window.clone(), options.exit_policy, resources),
            app,
            options,
            slots: HashMap::new(),
            error: None,
            device_resets: 0,
            device_loss: None,
            started: false,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: RunError) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl<A: App> ApplicationHandler for Runner<A> {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _: ()) {
        // A finished image job or a worker's wake-up: `about_to_wait` redraws exactly the
        // windows whose context reports that it needs a frame.
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
        match self.dispatch(event_loop, &key, event) {
            Ok(changed) => self.settle(event_loop, changed),
            Err(error) => self.fail(event_loop, RunError::Render(error)),
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
