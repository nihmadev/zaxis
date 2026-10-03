use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use zaxis::winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window as NativeWindow, WindowId},
};
use zaxis::{
    vec2, Context, PresentationMode, RenderError, RenderStatus, Renderer, Separator, Text, Window,
};

#[path = "../tests/support/gpu_cache.rs"]
mod gpu_cache;

#[derive(Default)]
struct Demo {
    state: Option<State>,
    error: Option<Box<dyn Error>>,
    smoke_test: bool,
}

struct State {
    window: Arc<NativeWindow>,
    renderer: Renderer,
    context: Context,
    clicks: u64,
    checked: bool,
    occluded: bool,
    retry_at: Option<Instant>,
}

impl ApplicationHandler for Demo {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = &mut self.state {
            state.occluded = false;
            state.context.request_repaint();
            state.window.request_redraw();
            return;
        }
        let result = (|| -> Result<State, Box<dyn Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    NativeWindow::default_attributes()
                        .with_title("zaxis — desktop GUI")
                        .with_inner_size(LogicalSize::new(860.0, 560.0)),
                )?,
            );
            let mode = if std::env::args().any(|arg| arg == "--vsync") {
                PresentationMode::Vsync
            } else {
                PresentationMode::Immediate
            };
            let renderer = pollster::block_on(Renderer::new_with_presentation_mode(
                Arc::clone(&window),
                mode,
            ))?;
            let mut context = Context::new();
            context.set_viewport(window.inner_size(), window.scale_factor());
            window.request_redraw();
            Ok(State {
                window,
                renderer,
                context,
                clicks: 0,
                checked: true,
                occluded: false,
                retry_at: None,
            })
        })();
        match result {
            Ok(state) => self.state = Some(state),
            Err(error) => {
                self.error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Surfaces must be dropped on platforms which invalidate them on suspension.
        self.state = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else {
            return;
        };
        if state.window.id() != id {
            return;
        }
        let response = state.context.on_window_event(&event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                let size = state.window.inner_size();
                state
                    .context
                    .set_viewport(size, state.window.scale_factor());
                if let Err(error) = state.renderer.resize(size) {
                    self.error = Some(Box::new(error));
                    event_loop.exit();
                }
            }
            WindowEvent::Occluded(value) => {
                state.occluded = value;
                if !value {
                    state.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let size = state.window.inner_size();
                if state.occluded
                    || state.window.is_minimized() == Some(true)
                    || size.width == 0
                    || size.height == 0
                {
                    return;
                }
                state.retry_at = None;
                let clicks = &mut state.clicks;
                let checked = &mut state.checked;
                state
                    .context
                    .run(|context| show_ui(context, clicks, checked));
                state.context.sync_ime(&state.window);
                match state
                    .renderer
                    .render(state.context.draw_data(), state.context.style().background)
                {
                    Ok(RenderStatus::Presented) if self.smoke_test => {
                        gpu_cache::verify_gpu_cache(state);
                        event_loop.exit();
                    }
                    Ok(RenderStatus::Presented | RenderStatus::Dormant) => {}
                    Ok(RenderStatus::Retry) => {
                        state.retry_at = Some(Instant::now() + Duration::from_millis(16))
                    }
                    Err(RenderError::DeviceLost(_)) => {
                        match pollster::block_on(Renderer::new_with_presentation_mode(
                            Arc::clone(&state.window),
                            state.renderer.presentation_mode(),
                        )) {
                            Ok(renderer) => {
                                state.renderer = renderer;
                                state.window.request_redraw();
                            }
                            Err(error) => {
                                self.error = Some(Box::new(error));
                                event_loop.exit();
                            }
                        }
                    }
                    Err(error) => {
                        self.error = Some(Box::new(error));
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
        if response.repaint && !state.occluded {
            state.window.request_redraw();
        }
        state.window.set_cursor(state.context.cursor_icon());
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &mut self.state else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };
        let size = state.window.inner_size();
        if state.occluded
            || state.window.is_minimized() == Some(true)
            || size.width == 0
            || size.height == 0
        {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        let now = Instant::now();
        let moving = state.renderer.presentation_mode() == PresentationMode::Vsync
            && state.context.wants_animation_frame();
        let redraw = state
            .retry_at
            .map_or(state.context.needs_repaint_at(now) || moving, |deadline| {
                deadline <= now
            });
        if redraw {
            state.window.request_redraw();
        }
        let deadline = state
            .retry_at
            .or(state.context.next_repaint())
            .filter(|time| !redraw && *time > now);
        event_loop.set_control_flow(deadline.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut demo = Demo {
        smoke_test: std::env::args().any(|arg| arg == "--smoke-test"),
        ..Default::default()
    };
    event_loop.run_app(&mut demo)?;
    if let Some(error) = demo.error {
        return Err(error);
    }
    Ok(())
}

fn show_ui(context: &mut Context, clicks: &mut u64, checked: &mut bool) {
    Window::new("zaxis")
        .default_position(vec2(56.0, 48.0))
        .default_size(vec2(410.0, 410.0))
        .min_size(vec2(270.0, 230.0))
        .show(context, |ui| {
            ui.add(Text::new("A small desktop UI").size(23.0));
            ui.label("Immediate mode widgets, cached geometry, and an event-driven renderer.");
            ui.separator();
            if ui.button("Click me").clicked() { *clicks += 1; }
            ui.label(format!("Clicks: {clicks}"));
            ui.checkbox(checked, "Show checkmark");
            ui.add(Separator::new().thickness(2.0).inset(8.0).spacing(2.0));
            let muted = ui.style().muted_text;
            ui.add(Text::new("Drag the title bar or resize the lower-right corner. Tab focuses the button; Space or Enter clicks it.").size(14.0).color(muted));
        });
}
