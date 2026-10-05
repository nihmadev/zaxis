//! The desktop runner: a blocking event loop, GPU setup that waits for the adapter, several
//! native windows, native window chrome, and device recovery on the spot.

use super::{
    shared_resources,
    window::{clamp_image_limits, Native},
    Runner, UserEvent,
};
use crate::app::{
    registry::OpenRequest, App, RunError, RunOptions, WindowError, WindowKey, WindowOptions,
};
use crate::{EventResponse, PresentationMode, RenderError, Renderer, SharedResources};
use std::sync::Arc;
use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::ModifiersState,
    window::Window,
};

/// What the desktop retains: the proxy through which accessibility adapters report.
pub(in crate::app) struct State {
    #[cfg(feature = "accesskit")]
    proxy: winit::event_loop::EventLoopProxy<UserEvent>,
}

impl State {
    pub(super) fn report_failure(&self, _error: &RunError) {}
    pub(super) fn note_input(&self, _event: &WindowEvent, _response: EventResponse) {}
}

/// Desktop windows need no extra frame for any key.
pub(super) fn frame_before_returning(_event: &WindowEvent, _modifiers: ModifiersState) -> bool {
    false
}

/// Run an application on the main thread until its last window closes.
pub(in crate::app) fn run<A: App>(app: A, options: RunOptions) -> Result<(), RunError> {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(RunError::EventLoop)?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let resources = shared_resources(&options);
    resources.set_image_waker(move || {
        let _ = proxy.send_event(UserEvent::Wake);
    });
    let state = State {
        #[cfg(feature = "accesskit")]
        proxy: event_loop.create_proxy(),
    };
    let mut runner = Runner::new(app, options, resources, state);
    let result = event_loop.run_app(&mut runner).map_err(RunError::EventLoop);
    runner.error.take().map_or(result, Err)
}

impl<A: App> Runner<A> {
    /// The first window. Unlike a secondary window, failing to create it ends the run.
    pub(super) fn open_main(&mut self, event_loop: &ActiveEventLoop) -> Result<(), RunError> {
        let key = self.options.main_window.clone();
        let mut options = WindowOptions::from_attributes(self.options.window_attributes.clone());
        options.presentation_mode = Some(self.options.presentation_mode);
        options.accessibility = self.options.accessibility;
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
        #[cfg(feature = "accesskit")]
        let proxy = (self.options.accessibility && options.accessibility)
            .then(|| self.platform.proxy.clone());
        create(
            event_loop,
            options,
            owner,
            sibling,
            self.options.presentation_mode,
            &self.hub.control.resources,
            #[cfg(feature = "accesskit")]
            proxy,
        )
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
                    // The new window has a new adapter, which knows nothing yet.
                    slot.context.set_accessibility_active(false);
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
            clamp_image_limits(&self.hub.control.resources, &renderer);
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

/// Create the native window and its renderer. `sibling` is any existing renderer: the new
/// window then shares its device, pipelines and textures instead of starting a new device.
fn create(
    event_loop: &ActiveEventLoop,
    options: &WindowOptions,
    owner: Option<&Window>,
    sibling: Option<&Renderer>,
    default_mode: PresentationMode,
    resources: &SharedResources,
    #[cfg(feature = "accesskit")] proxy: Option<winit::event_loop::EventLoopProxy<UserEvent>>,
) -> Result<Native, WindowError> {
    let attributes = options.attributes.clone();
    let decorations = cfg!(target_os = "macos") || attributes.decorations;
    let attributes = owned(attributes.with_decorations(decorations), owner);
    #[cfg(feature = "accesskit")]
    let (window, adapter) =
        crate::accessibility::adapter::create_window(event_loop, attributes, proxy)
            .map_err(WindowError::Create)?;
    #[cfg(not(feature = "accesskit"))]
    let window = event_loop
        .create_window(attributes)
        .map_err(WindowError::Create)?;
    let window = Arc::new(window);
    let mode = options.presentation_mode.unwrap_or(default_mode);
    let mut renderer = match sibling {
        Some(sibling) => sibling.create_sibling(Arc::clone(&window), mode),
        None => pollster::block_on(Renderer::new_with_presentation_mode(
            Arc::clone(&window),
            mode,
        )),
    }
    .map_err(WindowError::Render)?;
    renderer.set_transparent(options.attributes.transparent);
    clamp_image_limits(resources, &renderer);
    if window.is_visible() != Some(false) {
        window.request_redraw();
    }
    #[allow(unused_mut)]
    let mut native = Native::new(renderer, window, options);
    #[cfg(feature = "accesskit")]
    {
        native.adapter = adapter;
    }
    Ok(native)
}

/// An owned window stays above its owner in the Z-order and is destroyed with it.
#[cfg(target_os = "windows")]
fn owned(
    attributes: winit::window::WindowAttributes,
    owner: Option<&Window>,
) -> winit::window::WindowAttributes {
    use winit::{
        platform::windows::WindowAttributesExtWindows,
        raw_window_handle::{HasWindowHandle, RawWindowHandle},
    };
    match owner.map(|owner| owner.window_handle().map(|handle| handle.as_raw())) {
        Some(Ok(RawWindowHandle::Win32(handle))) => attributes.with_owner_window(handle.hwnd.get()),
        _ => attributes,
    }
}

/// Other platforms leave the Z-order of a child to the system; the runner still closes it
/// together with its parent.
#[cfg(not(target_os = "windows"))]
fn owned(
    attributes: winit::window::WindowAttributes,
    _owner: Option<&Window>,
) -> winit::window::WindowAttributes {
    attributes
}
