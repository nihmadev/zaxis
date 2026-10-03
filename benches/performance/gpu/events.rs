use super::*;
impl ApplicationHandler for GpuRunner<'_> {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _: ()) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let result = (|| -> BenchResult<GpuState> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("zaxis performance benchmark")
                        .with_inner_size(self.options.size)
                        .with_resizable(false),
                )?,
            );
            let start = Instant::now();
            let renderer = pollster::block_on(Renderer::new(Arc::clone(&window)))?;
            self.report.gpu_initialization_ms = Some(start.elapsed().as_secs_f64() * 1000.0);
            self.report.adapter = Some(format!("{:?}", renderer.adapter_info()));
            self.report.supported_present_modes = renderer
                .supported_present_modes()
                .iter()
                .map(|mode| format!("{mode:?}"))
                .collect();
            let size = window.inner_size();
            self.report.physical_size = [size.width, size.height];
            self.report.scale_factor = window.scale_factor();
            println!(
                "GPU: {:?}; modes {:?}; initialization {:.1} ms; native DPI {}",
                renderer.adapter_info(),
                self.report.supported_present_modes,
                start.elapsed().as_secs_f64() * 1000.0,
                window.scale_factor()
            );
            window.request_redraw();
            Ok(GpuState {
                window,
                renderer,
                active: None,
                retry_at: None,
            })
        })();
        match result {
            Ok(state) => self.state = Some(state),
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.fail(
            event_loop,
            "benchmark interrupted by window suspension".into(),
        );
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if !self.state.as_ref().is_some_and(|s| s.window.id() == id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Occluded(true) => {
                self.fail(event_loop, "benchmark window was occluded".into())
            }
            WindowEvent::Resized(size) => {
                let state = self.state.as_mut().unwrap();
                if size.width == 0 || size.height == 0 {
                    self.fail(event_loop, "benchmark window was minimized".into());
                } else if let Err(error) = state.renderer.resize(size) {
                    self.fail(event_loop, Box::new(error));
                } else if state.active.is_some() {
                    self.fail(
                        event_loop,
                        "viewport changed during a benchmark case".into(),
                    );
                }
            }
            WindowEvent::RedrawRequested => match self.redraw() {
                Ok(true) => {}
                Ok(false) => {
                    self.finished = true;
                    event_loop.exit();
                }
                Err(error) => self.fail(event_loop, error),
            },
            _ => {} // Live mouse/keyboard input must not contaminate scripted scenes.
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &self.state else {
            return;
        };
        if let Some(deadline) = state.retry_at.filter(|t| *t > Instant::now()) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        } else {
            if state
                .active
                .as_ref()
                .is_some_and(|a| a.image_transaction.is_some() && !a.scene.context.needs_repaint())
            {
                event_loop.set_control_flow(
                    state
                        .active
                        .as_ref()
                        .and_then(|a| a.scene.context.next_repaint())
                        .map_or(ControlFlow::Wait, ControlFlow::WaitUntil),
                );
                return;
            }
            state.window.request_redraw();
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
