//! Single-window desktop runner. The low-level Context and Renderer remain available.

use crate::{Context, PresentationMode, RenderError, RenderStatus, Renderer};
use std::{
    error::Error,
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{CursorIcon, Window, WindowAttributes, WindowId},
};

/// Application state lives here, independently of the native window and GPU.
pub trait App {
    /// Build the UI on each redraw. Do not call `Context::run` yourself.
    /// Widgets request follow-up frames automatically. For timers and animations,
    /// use `Context::request_repaint_after`; the runner sleeps between redraws.
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>);
}

/// Native window access and application shutdown during an update.
pub struct Frame<'a> {
    window: &'a Arc<Window>,
    close_requested: &'a mut bool,
}

impl Frame<'_> {
    /// Native window for changing the title, requesting redraws, or cloning its
    /// Arc into a background worker that needs to wake the sleeping event loop.
    pub fn window(&self) -> &Arc<Window> {
        self.window
    }

    /// Exit after the current frame has been presented successfully.
    pub fn close(&mut self) {
        *self.close_requested = true;
    }
}

/// Native window configuration. Sizes in `WindowAttributes` may be logical or physical.
/// On macOS the runner always enables system decorations for native controls and resizing.
#[derive(Debug, Clone)]
pub struct RunOptions {
    pub window_attributes: WindowAttributes,
    pub presentation_mode: PresentationMode,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            window_attributes: Window::default_attributes()
                .with_title("zaxis")
                .with_inner_size(LogicalSize::new(860.0, 560.0)),
            presentation_mode: PresentationMode::default(),
        }
    }
}

impl RunOptions {
    /// Request native rounded corners and the system shadow on Windows 11.
    /// Applies to undecorated windows as well. Maximized/snapped windows follow
    /// the OS policy. On other platforms this leaves window attributes unchanged.
    /// Call after setting `window_attributes`.
    pub fn with_rounded_corners(mut self, rounded: bool) -> Self {
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::{CornerPreference, WindowAttributesExtWindows};
            self.window_attributes = self
                .window_attributes
                .with_undecorated_shadow(rounded)
                .with_corner_preference(if rounded {
                    CornerPreference::Round
                } else {
                    CornerPreference::DoNotRound
                });
        }
        #[cfg(not(target_os = "windows"))]
        let _ = rounded;
        self
    }
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
    let mut context = Context::new();
    context.set_image_waker(move || {
        let _ = proxy.send_event(());
    });
    let mut runner = Runner {
        app,
        options,
        context,
        state: None,
        error: None,
        close_requested: false,
    };
    let result = event_loop.run_app(&mut runner).map_err(RunError::EventLoop);
    runner.error.map_or(result, Err)
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

struct State {
    window: Arc<Window>,
    renderer: Renderer,
    occluded: bool,
    retry_at: Option<Instant>,
    cursor: CursorIcon,
}

impl State {
    fn visible(&self) -> bool {
        let size = self.window.inner_size();
        !self.occluded
            && self.window.is_minimized() != Some(true)
            && size.width > 0
            && size.height > 0
    }
}

struct Runner<A> {
    app: A,
    options: RunOptions,
    context: Context,
    state: Option<State>,
    error: Option<RunError>,
    close_requested: bool,
}

impl<A: App> Runner<A> {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: RunError) {
        self.error = Some(error);
        event_loop.exit();
    }

    fn create_state(&mut self, event_loop: &ActiveEventLoop) -> Result<State, RunError> {
        let window = Arc::new(
            event_loop
                .create_window(self.options.window_attributes.clone().with_decorations(
                    cfg!(target_os = "macos") || self.options.window_attributes.decorations,
                ))
                .map_err(RunError::Window)?,
        );
        let renderer = pollster::block_on(Renderer::new_with_presentation_mode(
            Arc::clone(&window),
            self.options.presentation_mode,
        ))
        .map_err(RunError::Render)?;
        let mut limits = self.context.image_limits().clone();
        limits.max_dimension = limits
            .max_dimension
            .min(renderer.max_texture_dimension_2d());
        if limits.max_dimension != self.context.image_limits().max_dimension {
            self.context.set_image_limits(limits);
        }
        self.context
            .set_viewport(window.inner_size(), window.scale_factor());
        window.request_redraw();
        Ok(State {
            window,
            renderer,
            occluded: false,
            retry_at: None,
            cursor: CursorIcon::Default,
        })
    }

    fn handle_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) -> Result<(), RenderError> {
        let state = self.state.as_mut().unwrap();
        if self.context.native_chrome_press(&event, &state.window) {
            state.window.request_redraw();
            return Ok(());
        }
        let response = self.context.on_window_event(&event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                let size = state.window.inner_size();
                self.context.set_viewport(size, state.window.scale_factor());
                state.renderer.resize(size)?;
            }
            WindowEvent::Occluded(value) => {
                state.occluded = value;
                if !value {
                    state.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested if state.visible() => {
                state.retry_at = None;
                let mut frame = Frame {
                    window: &state.window,
                    close_requested: &mut self.close_requested,
                };
                self.context
                    .run(|context| self.app.update(context, &mut frame));
                self.context.sync_ime(&state.window);
                match state
                    .renderer
                    .render(self.context.draw_data(), self.context.style().background)
                {
                    Ok(RenderStatus::Presented) => {
                        if self.close_requested {
                            event_loop.exit();
                        }
                    }
                    Ok(RenderStatus::Dormant) => {}
                    Ok(RenderStatus::Retry) => {
                        state.retry_at = Some(Instant::now() + Duration::from_millis(16))
                    }
                    Err(RenderError::DeviceLost(_)) => {
                        state.renderer = pollster::block_on(Renderer::new_with_presentation_mode(
                            Arc::clone(&state.window),
                            state.renderer.presentation_mode(),
                        ))?;
                        let mut limits = self.context.image_limits().clone();
                        limits.max_dimension = limits
                            .max_dimension
                            .min(state.renderer.max_texture_dimension_2d());
                        if limits.max_dimension != self.context.image_limits().max_dimension {
                            self.context.set_image_limits(limits);
                        }
                        state.window.request_redraw();
                    }
                    Err(error) => return Err(error),
                }
            }
            _ => {}
        }
        if response.repaint && state.visible() {
            state.window.request_redraw();
        }
        let cursor = self.context.cursor_icon();
        if cursor != state.cursor {
            state.window.set_cursor(cursor);
            state.cursor = cursor;
        }
        Ok(())
    }
}

impl<A: App> ApplicationHandler for Runner<A> {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _: ()) {
        if let Some(state) = &self.state {
            if state.visible() {
                state.window.request_redraw();
            }
        }
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = &mut self.state {
            state.occluded = false;
            self.context.request_repaint();
            state.window.request_redraw();
        } else {
            match self.create_state(event_loop) {
                Ok(state) => self.state = Some(state),
                Err(error) => self.fail(event_loop, error),
            }
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        // Drop the surface on platforms that invalidate it on suspension, while
        // preserving both application state and retained UI state.
        self.state = None;
        self.context.on_window_event(&WindowEvent::Focused(false));
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if !self
            .state
            .as_ref()
            .is_some_and(|state| state.window.id() == id)
        {
            return;
        }
        if let Err(error) = self.handle_event(event_loop, event) {
            self.fail(event_loop, RunError::Render(error));
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &mut self.state else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };
        let now = Instant::now();
        let moving = state.renderer.presentation_mode() == PresentationMode::Vsync
            && self.context.wants_animation_frame();
        let (redraw, control_flow) = repaint_schedule(
            now,
            state.visible(),
            self.context.needs_repaint_at(now) || moving,
            self.context.next_repaint(),
            state.retry_at,
        );
        if redraw {
            state.window.request_redraw();
        }
        event_loop.set_control_flow(control_flow);
    }
}

fn repaint_schedule(
    now: Instant,
    visible: bool,
    needs_repaint: bool,
    next_repaint: Option<Instant>,
    retry_at: Option<Instant>,
) -> (bool, ControlFlow) {
    if !visible {
        return (false, ControlFlow::Wait);
    }
    // Back off transient surface failures even when the UI requests another frame.
    let redraw = retry_at.map_or(needs_repaint, |deadline| deadline <= now);
    let deadline = retry_at
        .or(next_repaint)
        .filter(|time| !redraw && *time > now);
    (
        redraw,
        deadline.map_or(ControlFlow::Wait, ControlFlow::WaitUntil),
    )
}

#[cfg(test)]
mod tests;
