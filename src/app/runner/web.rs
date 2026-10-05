//! The browser runner: one canvas, a renderer created asynchronously, and no blocking call
//! anywhere. `run_with_options` registers the event loop with the page and returns; the
//! browser then drives it. winit schedules frames with `requestAnimationFrame` and wake-ups
//! with timers, so an idle page does no work and a background tab draws nothing.
//!
//! Sequence: the window (canvas) is created at once; `Renderer::new_with_backends` runs as a
//! task and reports back with [`UserEvent::Renderer`]; only then does the window get a
//! context and its first frame.

mod canvas;

use super::{
    shared_resources,
    window::{clamp_image_limits, Native},
    Runner, UserEvent,
};
use crate::app::{
    registry::OpenRequest, App, RunError, RunOptions, WindowError, WindowKey, WindowOptions,
};
use crate::{EventResponse, PresentationMode, RenderError, Renderer};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
use wasm_bindgen::JsValue;
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys},
    window::Window,
};

thread_local! {
    /// Whether the UI consumed the key press or wheel event winit just delivered. The
    /// canvas listener that runs right after winit's reads it to decide on `preventDefault`.
    static CONSUMED: Cell<bool> = const { Cell::new(false) };
}

/// What a finished renderer is for.
pub(super) enum Purpose {
    /// The first frame of a window that was waiting for its renderer.
    Open,
    /// A replacement after the device was lost.
    Recover(WindowKey),
}

/// A renderer (or the reason there is none) arriving from its task.
pub(in crate::app) struct Ready {
    window: Arc<Window>,
    result: Result<Renderer, RenderError>,
    purpose: Purpose,
}

/// The main window while its renderer is being created.
struct Pending {
    key: WindowKey,
    options: WindowOptions,
}

pub(in crate::app) struct State {
    proxy: EventLoopProxy<UserEvent>,
    pending: Option<Pending>,
    /// Renderers finished by their tasks, taken by [`Runner::renderers_ready`].
    inbox: Rc<RefCell<Vec<Ready>>>,
}

impl State {
    /// Errors after `run_with_options` returned have no caller; the console is where they go.
    pub(super) fn report_failure(&self, error: &RunError) {
        web_sys::console::error_1(&JsValue::from_str(&format!("zaxis: {error}")));
    }

    pub(super) fn note_input(&self, event: &WindowEvent, response: EventResponse) {
        if matches!(
            event,
            WindowEvent::KeyboardInput { .. } | WindowEvent::MouseWheel { .. }
        ) {
            CONSUMED.with(|consumed| consumed.set(response.consumed));
        }
    }
}

/// Copying must draw its frame inside the key press, where the browser permits a write.
pub(super) fn frame_before_returning(event: &WindowEvent, modifiers: ModifiersState) -> bool {
    matches!(
        event,
        WindowEvent::KeyboardInput { event, .. }
            if event.state == ElementState::Pressed
                && !event.repeat
                && matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyC | KeyCode::KeyX))
                && (modifiers.control_key() || modifiers.super_key())
    )
}

/// Register the application with the page's event loop and return at once.
pub(in crate::app) fn run<A: App + 'static>(app: A, options: RunOptions) -> Result<(), RunError> {
    console_error_panic_hook::set_once();
    crate::context::install_web_clipboard();
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(RunError::EventLoop)?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let resources = shared_resources(&options);
    let waker = proxy.clone();
    resources.set_image_waker(move || {
        let _ = waker.send_event(UserEvent::Wake);
    });
    let state = State {
        proxy,
        pending: None,
        inbox: Rc::default(),
    };
    let mut runner = Runner::new(app, options, resources, state);
    runner.hub.control.max_windows = Some(1);
    event_loop.spawn_app(runner);
    Ok(())
}

impl<A: App> Runner<A> {
    /// Create the canvas window now and its renderer in the background.
    pub(super) fn open_main(&mut self, event_loop: &ActiveEventLoop) -> Result<(), RunError> {
        let key = self.options.main_window.clone();
        let mut options = WindowOptions::from_attributes(self.options.window_attributes.clone());
        options.presentation_mode = Some(self.options.presentation_mode);
        self.hub.begin_open(&key, None, false);
        let page = canvas::acquire(&self.options.web)?;
        // The page sizes the canvas; a fixed inner size would pin it in pixels.
        let mut attributes = options.attributes.clone();
        attributes.inner_size = None;
        let attributes = attributes
            .with_canvas(Some(page.canvas.clone()))
            .with_append(false)
            .with_prevent_default(false);
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(RunError::Window)?,
        );
        canvas::install(&page);
        let mode = self.options.presentation_mode;
        self.platform.pending = Some(Pending { key, options });
        self.start_renderer(window, mode, Purpose::Open);
        Ok(())
    }

    /// The browser draws on a single canvas; a second window is refused as a result the
    /// application can read, never a panic.
    pub(super) fn open_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: WindowKey,
        options: WindowOptions,
        declared: bool,
    ) {
        if self.hub.begin_open(&key, options.parent.as_ref(), declared) == OpenRequest::Exists {
            return;
        }
        let reason = "the browser runner draws on a single canvas".to_owned();
        self.report_failure(event_loop, key, WindowError::Unsupported(reason));
    }

    /// The browser never suspends the page's canvas the way a mobile system suspends a window.
    pub(super) fn recreate_natives(&mut self, _: &ActiveEventLoop) -> Result<(), RunError> {
        Ok(())
    }

    /// The device was lost: build a new renderer in the background and draw nothing until it
    /// arrives.
    pub(super) fn recover_device(
        &mut self,
        _: &ActiveEventLoop,
        reason: String,
    ) -> Result<(), RenderError> {
        self.device_resets += 1;
        self.device_loss = Some(reason);
        let rebuilding: Vec<_> = self
            .slots
            .iter_mut()
            .filter_map(|(key, slot)| {
                let native = slot.native.as_mut().filter(|n| !n.rebuilding)?;
                native.rebuilding = true;
                Some((
                    key.clone(),
                    Arc::clone(&native.window),
                    native.renderer.presentation_mode(),
                ))
            })
            .collect();
        for (key, window, mode) in rebuilding {
            self.start_renderer(window, mode, Purpose::Recover(key));
        }
        Ok(())
    }

    /// Continue what the finished renderer tasks were started for.
    pub(super) fn renderers_ready(&mut self, event_loop: &ActiveEventLoop) {
        let ready = std::mem::take(&mut *self.platform.inbox.borrow_mut());
        for ready in ready {
            self.renderer_ready(event_loop, ready);
        }
    }

    fn renderer_ready(&mut self, event_loop: &ActiveEventLoop, ready: Ready) {
        let Ready {
            window,
            result,
            purpose,
        } = ready;
        let mut renderer = match result {
            Ok(renderer) => renderer,
            Err(error) => return self.fail(event_loop, RunError::Render(error)),
        };
        clamp_image_limits(&self.hub.control.resources, &renderer);
        let info = renderer.adapter_info();
        web_sys::console::info_1(&JsValue::from_str(&format!(
            "zaxis: {:?} renderer on {}",
            info.backend, info.name
        )));
        match purpose {
            Purpose::Open => {
                let Some(Pending { key, options }) = self.platform.pending.take() else {
                    return;
                };
                renderer.set_transparent(options.attributes.transparent);
                let native = Native::new(renderer, window, &options);
                native.window.request_redraw();
                self.attach(key, options, native);
                self.settle(event_loop, true);
            }
            Purpose::Recover(key) => {
                let Some(slot) = self.slots.get_mut(&key) else {
                    return;
                };
                let Some(native) = slot.native.as_mut() else {
                    return;
                };
                renderer.set_transparent(native.transparent);
                native.renderer = renderer;
                native.rebuilding = false;
                native.retry_at = None;
                slot.context.request_repaint();
                native.window.request_redraw();
            }
        }
    }

    fn start_renderer(&self, window: Arc<Window>, mode: PresentationMode, purpose: Purpose) {
        let backends = canvas::backends(self.options.web.backend);
        let proxy = self.platform.proxy.clone();
        let inbox = Rc::clone(&self.platform.inbox);
        wasm_bindgen_futures::spawn_local(async move {
            let result =
                Renderer::new_with_backends(Arc::clone(&window), mode, Some(backends)).await;
            inbox.borrow_mut().push(Ready {
                window,
                result,
                purpose,
            });
            let _ = proxy.send_event(UserEvent::Renderer);
        });
    }
}
