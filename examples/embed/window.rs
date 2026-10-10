//! The host's window, surface and event loop. zaxis only sees the events forwarded to it and
//! the target views it is asked to draw into.

use crate::{
    frame,
    gpu::Gpu,
    interface::{Interface, Mode},
    scene::Scene,
    targets::{Targets, DEPTH},
};
use std::{error::Error, sync::Arc};
use zaxis::winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use zaxis::{EmbedOptions, Instant};

struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    view_format: wgpu::TextureFormat,
    gpu: Gpu,
    scene: Scene,
    targets: Targets,
    interface: Interface,
    last: Instant,
}

struct Host {
    pass: bool,
    state: Option<State>,
    error: Option<Box<dyn Error>>,
}

pub fn run(pass: bool) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut host = Host {
        pass,
        state: None,
        error: None,
    };
    event_loop.run_app(&mut host)?;
    host.error.map_or(Ok(()), Err)
}

impl State {
    fn new(event_loop: &ActiveEventLoop, pass: bool) -> Result<Self, Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("zaxis — embedded in a host")
                    .with_inner_size(LogicalSize::new(960.0, 600.0)),
            )?,
        );
        let (gpu, surface) = Gpu::new(Some(&window))?;
        let surface = surface.expect("a window has a surface");
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1))
            .ok_or("the adapter cannot present to this window")?;
        let capabilities = surface.get_capabilities(&gpu.adapter);
        // The sRGB view of the texture is what the shaders must render to; the host lists it
        // in `view_formats`, as a window renderer of zaxis does.
        config.format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(config.format);
        let view_format = config.format.add_srgb_suffix();
        if view_format != config.format {
            config.view_formats.push(view_format);
        }
        // Backdrop effects copy what the scene drew.
        if capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC) {
            config.usage |= wgpu::TextureUsages::COPY_SRC;
        }
        surface.configure(&gpu.device, &config);
        let samples = gpu.samples(view_format, DEPTH);
        let mode = if pass || !config.usage.contains(wgpu::TextureUsages::COPY_SRC) {
            Mode::Pass
        } else {
            Mode::Texture
        };
        let options = EmbedOptions::new(config.format)
            .sample_count(samples)
            .depth_format(DEPTH);
        let mut interface = Interface::new(&gpu, options, mode)?;
        interface.context.set_viewport(size, window.scale_factor());
        window.request_redraw();
        Ok(Self {
            scene: Scene::new(&gpu, view_format, samples),
            targets: Targets::new(&gpu, [config.width, config.height], samples, view_format),
            window,
            surface,
            config,
            view_format,
            gpu,
            interface,
            last: Instant::now(),
        })
    }

    fn resize(&mut self) {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.gpu.device, &self.config);
        self.targets = Targets::new(
            &self.gpu,
            [size.width, size.height],
            self.targets.samples,
            self.view_format,
        );
    }

    fn redraw(&mut self) -> Result<(), Box<dyn Error>> {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let now = Instant::now();
        self.interface
            .model
            .step(now.duration_since(self.last).as_secs_f32().min(0.1));
        self.last = now;
        self.interface.build(size, self.window.scale_factor());
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.resize();
                self.window.request_redraw();
                return Ok(());
            }
            _ => return Ok(()),
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.view_format),
            ..Default::default()
        });
        self.scene
            .update(&self.gpu, &self.interface.model, self.targets.size);
        frame::draw(
            &self.gpu,
            &self.scene,
            &self.targets,
            Some(&mut self.interface),
            &view,
        )?;
        self.window.pre_present_notify();
        self.gpu.queue.present(frame);
        self.window.set_cursor(self.interface.context.cursor_icon());
        self.interface.context.sync_ime(&self.window);
        Ok(())
    }
}

impl ApplicationHandler for Host {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            match State::new(event_loop, self.pass) {
                Ok(state) => self.state = Some(state),
                Err(error) => {
                    self.error = Some(error);
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else {
            return;
        };
        let repaint = state.interface.input(&event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => state.resize(),
            WindowEvent::RedrawRequested => {
                if let Err(error) = state.redraw() {
                    self.error = Some(error);
                    event_loop.exit();
                }
            }
            _ => {}
        }
        if repaint {
            state.window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &mut self.state else {
            return;
        };
        let now = Instant::now();
        let context = &state.interface.context;
        // The scene moves by itself; otherwise the interface says when it needs a frame.
        if state.interface.model.rotate || context.needs_repaint_at(now) {
            state.window.request_redraw();
        }
        event_loop.set_control_flow(match context.next_repaint().filter(|time| *time > now) {
            Some(time) if !state.interface.model.rotate => ControlFlow::WaitUntil(time),
            _ => ControlFlow::Wait,
        });
    }
}
