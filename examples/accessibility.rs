//! A form that a screen reader can operate: every control has a role, a name and a value,
//! and accepts the requests assistive technology sends.
//!
//! Run it with Narrator (Win+Ctrl+Enter), NVDA or VoiceOver. `--smoke-test` first checks
//! the accessibility tree and a series of requests without a window, then presents a few
//! frames and exits.
use zaxis::{App, Context, Frame, RunOptions};

#[path = "accessibility/model.rs"]
mod model;
#[path = "accessibility/smoke.rs"]
mod smoke;
#[path = "accessibility/ui.rs"]
mod ui;

struct Form {
    model: model::Model,
    smoke: bool,
    frames: usize,
}

impl App for Form {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        ui::show(context, &mut self.model);
        if self.smoke {
            self.frames += 1;
            if self.frames >= 4 {
                frame.close();
            } else {
                context.request_repaint();
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke-test");
    if smoke {
        smoke::run()?;
        println!("accessibility smoke: tree valid, requests applied");
    }
    let options = RunOptions {
        window_attributes: zaxis::winit::window::Window::default_attributes()
            .with_title("zaxis — Accessible form")
            .with_inner_size(zaxis::winit::dpi::LogicalSize::new(560.0, 680.0)),
        ..Default::default()
    };
    zaxis::run_with_options(
        Form {
            model: model::Model::new(),
            smoke,
            frames: 0,
        },
        options,
    )?;
    Ok(())
}
