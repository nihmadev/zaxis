//! zaxis inside the host: a context fed with the host's input, and the embedded renderer that
//! draws its frames into the host's pass or texture.

use crate::{gpu::Gpu, model::Model};
use std::error::Error;
use zaxis::winit::{dpi::PhysicalSize, event::WindowEvent};
use zaxis::{
    vec2, Button, Checkbox, Context, EmbedLoad, EmbedOptions, EmbedViewport, EmbeddedRenderer,
    Slider, Theme, Window,
};

/// How a frame of the interface reaches the host's target.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Recorded into the pass that draws the scene: its MSAA and depth attachments, but no
    /// backdrop effects.
    Pass,
    /// Rendered into the finished scene texture: glass and blur see the scene.
    Texture,
}

pub struct Interface {
    pub context: Context,
    pub model: Model,
    pub renderer: EmbeddedRenderer,
    pub mode: Mode,
}

impl Interface {
    pub fn new(gpu: &Gpu, options: EmbedOptions, mode: Mode) -> Result<Self, Box<dyn Error>> {
        let renderer = EmbeddedRenderer::new_with_adapter(
            gpu.device.clone(),
            gpu.queue.clone(),
            &gpu.adapter,
            options,
        )?;
        let mut context = Context::new();
        context.set_theme(Theme::dark());
        Ok(Self {
            context,
            model: Model::default(),
            renderer,
            mode,
        })
    }

    /// Feed one host event to the context; true when the interface wants a new frame.
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.context.on_window_event(event).repaint
    }

    /// Build the interface for a target of `size` pixels at `scale`.
    pub fn build(&mut self, size: PhysicalSize<u32>, scale: f64) {
        self.context.set_viewport(size, scale);
        let model = &mut self.model;
        self.context.run(|context| show(context, model));
    }

    /// Upload the frame for recording into the scene's pass.
    pub fn prepare(&mut self, size: [u32; 2]) -> Result<(), Box<dyn Error>> {
        self.renderer
            .prepare(self.context.draw_data(), EmbedViewport::whole(size))?;
        Ok(())
    }

    /// Write the frame into `encoder`, over the scene already in `target`.
    pub fn render_to(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) -> Result<(), Box<dyn Error>> {
        self.renderer.render_to(
            encoder,
            target,
            None,
            EmbedLoad::Keep,
            self.context.draw_data(),
        )?;
        Ok(())
    }
}

fn show(context: &mut Context, model: &mut Model) {
    Window::new("Tools")
        .default_position(vec2(24.0, 24.0))
        .fit_content(true)
        .blur(model.blur)
        .show(context, |ui| {
            ui.add(Checkbox::new(&mut model.rotate, "Rotate"));
            ui.label("Speed");
            ui.add(Slider::new(&mut model.speed, 0.0..=3.0).width(220.0));
            ui.label("Size");
            ui.add(Slider::new(&mut model.size, 0.4..=1.4).width(220.0));
            if ui.add(Button::new("Reset")).clicked() {
                *model = Model::default();
            }
        });
    Window::new("Glass")
        .default_position(vec2(330.0, 250.0))
        .fit_content(true)
        .blur(model.blur)
        .show(context, |ui| {
            ui.label("Blur");
            ui.add(Slider::new(&mut model.blur, 1.0..=40.0).width(220.0));
        });
}
